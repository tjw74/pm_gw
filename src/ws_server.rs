use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::{
    auth::AuthenticatedUser, broadcast::EventBroadcaster, commands::ClientMessage,
    error::GatewayError, execution::polymarket::PolymarketExecutionClient,
    responses::ServerMessage, session::Session, state::AppState,
};

pub async fn ws_route(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    if let Err(err) = process_socket(socket, state.clone()).await {
        warn!(?err, "websocket connection failed");
    }
}

async fn process_socket(socket: WebSocket, state: Arc<AppState>) -> Result<(), GatewayError> {
    let execution = Arc::new(
        PolymarketExecutionClient::new(state.config.clone(), state.clone())
            .map_err(|err| GatewayError::internal(err.to_string()))?,
    );
    let (mut sender, mut receiver) = socket.split();

    let first = receiver
        .next()
        .await
        .ok_or_else(|| GatewayError::Unauthorized)?
        .map_err(|err| GatewayError::bad_request(err.to_string()))?;
    let auth_message = parse_client_message(first)?;
    let ClientMessage::Auth { token } = auth_message else {
        let payload = serde_json::to_string(&ServerMessage::AuthError {
            error: "first message must be auth".to_string(),
        })
        .map_err(|err| GatewayError::internal(err.to_string()))?;
        sender
            .send(Message::Text(payload.into()))
            .await
            .map_err(|err| GatewayError::internal(err.to_string()))?;
        return Err(GatewayError::Unauthorized);
    };

    let auth_user = state.auth.verify_token(&token)?;
    let session = state.create_session(auth_user.user_id.clone()).await;
    let auth_ok = serde_json::to_string(&ServerMessage::AuthOk {
        session_id: session.id,
        user_id: auth_user.user_id.clone(),
    })
    .map_err(|err| GatewayError::internal(err.to_string()))?;
    sender
        .send(Message::Text(auth_ok.into()))
        .await
        .map_err(|err| GatewayError::internal(err.to_string()))?;
    sender
        .send(Message::Text(
            serde_json::to_string(&ServerMessage::Snapshot {
                payload: state.snapshot().await,
            })
            .map_err(|err| GatewayError::internal(err.to_string()))?
            .into(),
        ))
        .await
        .map_err(|err| GatewayError::internal(err.to_string()))?;
    info!(user = %auth_user.user_id, session_id = %session.id, "client authenticated");

    let mut event_rx = state.broadcaster.subscribe_events();
    let mut snapshot_rx = state.broadcaster.subscribe_snapshots();
    let heartbeat_state = state.clone();
    let heartbeat_session_id = session.id;
    let heartbeat_task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            ticker.tick().await;
            heartbeat_state.touch_session(heartbeat_session_id).await;
        }
    });

    loop {
        tokio::select! {
            maybe_message = receiver.next() => {
                match maybe_message {
                    Some(Ok(message)) => {
                        if handle_incoming(message, &state, &execution, &auth_user, &session, &mut sender).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(err)) => {
                        warn!(?err, "websocket read error");
                        break;
                    }
                    None => break,
                }
            }
            result = event_rx.recv() => {
                match result {
                    Ok(event) => {
                        let message = EventBroadcaster::route_event(event);
                        if send_message(&mut sender, &message).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let snapshot = snapshot_rx.borrow().clone();
                        if send_message(&mut sender, &ServerMessage::Snapshot { payload: snapshot }).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            changed = snapshot_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let payload = snapshot_rx.borrow().clone();
                if send_message(&mut sender, &ServerMessage::Snapshot { payload }).await.is_err() {
                    break;
                }
            }
        }
    }

    heartbeat_task.abort();
    state.remove_session(session.id).await;
    Ok(())
}

async fn handle_incoming(
    message: Message,
    state: &Arc<AppState>,
    execution: &Arc<PolymarketExecutionClient>,
    auth_user: &AuthenticatedUser,
    session: &Session,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<(), GatewayError> {
    match message {
        Message::Text(_) | Message::Binary(_) => {
            let command = parse_client_message(message)?;
            route_command(command, state, execution, auth_user, session, sender).await
        }
        Message::Ping(payload) => {
            sender
                .send(Message::Pong(payload))
                .await
                .map_err(|err| GatewayError::internal(err.to_string()))?;
            Ok(())
        }
        Message::Close(_) => Err(GatewayError::Unauthorized),
        _ => Ok(()),
    }
}

async fn route_command(
    command: ClientMessage,
    state: &Arc<AppState>,
    execution: &Arc<PolymarketExecutionClient>,
    auth_user: &AuthenticatedUser,
    session: &Session,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<(), GatewayError> {
    if let (Some(command_id), Some(timestamp)) = (command.command_id(), command.timestamp()) {
        state
            .validate_command(&auth_user.user_id, command_id, timestamp)
            .await?;
        send_message(
            sender,
            &ServerMessage::CommandAck {
                command_id,
                accepted_at: time::OffsetDateTime::now_utc(),
            },
        )
        .await?;
    }

    match &command {
        ClientMessage::SubscribeMarket { market, .. } => {
            let target_market = if let Some(market) = market.clone() {
                market
            } else {
                state
                    .active_market_slug()
                    .await
                    .unwrap_or_else(|| "btc".to_string())
            };
            state.subscribe_market(session.id, target_market).await;
            send_message(
                sender,
                &ServerMessage::Snapshot {
                    payload: state.snapshot().await,
                },
            )
            .await?;
            Ok(())
        }
        ClientMessage::UnsubscribeMarket { market, .. } => {
            if let Some(market) = market
                .as_ref()
                .or(state.active_market_slug().await.as_ref())
            {
                state.unsubscribe_market(session.id, market).await;
            }
            Ok(())
        }
        ClientMessage::GetOpenOrders { .. } => {
            send_message(
                sender,
                &ServerMessage::OrderUpdate {
                    payload: execution
                        .handle_command(&auth_user.user_id, &command)
                        .await?,
                },
            )
            .await
        }
        ClientMessage::GetPositions { .. } => {
            send_message(
                sender,
                &ServerMessage::PositionUpdate {
                    payload: execution
                        .handle_command(&auth_user.user_id, &command)
                        .await?,
                },
            )
            .await
        }
        ClientMessage::GetAccountState { .. } => {
            let payload = execution
                .handle_command(&auth_user.user_id, &command)
                .await?;
            send_message(sender, &ServerMessage::AccountUpdate { payload }).await
        }
        ClientMessage::SetTargetPrice { .. } => {
            let payload = execution
                .handle_command(&auth_user.user_id, &command)
                .await?;
            send_message(sender, &ServerMessage::Snapshot { payload }).await
        }
        ClientMessage::Ping { .. } => {
            send_message(
                sender,
                &ServerMessage::Heartbeat {
                    timestamp: time::OffsetDateTime::now_utc(),
                },
            )
            .await
        }
        ClientMessage::PlaceLimitOrder { .. }
        | ClientMessage::PlaceMarketOrder { .. }
        | ClientMessage::CancelOrder { .. }
        | ClientMessage::CancelAll { .. } => {
            let payload = execution
                .handle_command(&auth_user.user_id, &command)
                .await?;
            send_message(sender, &ServerMessage::OrderUpdate { payload }).await
        }
        ClientMessage::Auth { .. } => Err(GatewayError::Unauthorized),
    }
}

fn parse_client_message(message: Message) -> Result<ClientMessage, GatewayError> {
    let bytes = match message {
        Message::Text(text) => text.as_bytes().to_vec(),
        Message::Binary(bin) => bin.to_vec(),
        _ => {
            return Err(GatewayError::bad_request(
                "unsupported websocket message type",
            ));
        }
    };
    serde_json::from_slice(&bytes).map_err(|err| GatewayError::bad_request(err.to_string()))
}

async fn send_message(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> Result<(), GatewayError> {
    let payload =
        serde_json::to_string(message).map_err(|err| GatewayError::internal(err.to_string()))?;
    sender
        .send(Message::Text(payload.into()))
        .await
        .map_err(|err| GatewayError::internal(err.to_string()))
}
