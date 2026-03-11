use std::time::Duration;

pub fn backoff(attempt: u32) -> Duration {
    let capped = attempt.min(6);
    Duration::from_secs(2_u64.pow(capped))
}
