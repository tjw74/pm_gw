import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center rounded-2xl text-sm font-medium transition focus:outline-none focus:ring-2 focus:ring-accent/40 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-accent text-accent-foreground hover:bg-accent/90",
        subtle: "bg-white/5 text-foreground hover:bg-white/10",
        ghost: "bg-transparent text-muted-foreground hover:bg-white/5 hover:text-foreground",
        danger: "bg-danger/20 text-danger hover:bg-danger/30"
      },
      size: {
        default: "h-11 px-4",
        sm: "h-9 px-3",
      }
    },
    defaultVariants: {
      variant: "default",
      size: "default"
    }
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button ref={ref} className={cn(buttonVariants({ variant, size }), className)} {...props} />
  ),
);
Button.displayName = "Button";
