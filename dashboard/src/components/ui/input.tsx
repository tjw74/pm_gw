import * as React from "react";
import { cn } from "@/lib/utils";

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn("h-11 w-full rounded-2xl border border-white/10 bg-black/20 px-4 text-sm outline-none ring-0 placeholder:text-muted-foreground focus:border-accent/60", className)}
      {...props}
    />
  ),
);
Input.displayName = "Input";
