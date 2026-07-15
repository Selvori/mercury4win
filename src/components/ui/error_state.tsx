// mercury4win-linux/src/components/ui/error_state.tsx
// Reusable error state with retry action

import { type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "./button";
import { cn } from "@/lib/utils";

interface Props {
  message?: string;
  on_retry?: () => void;
  className?: string;
  children?: ReactNode;
}

export function ErrorState({
  message = "Something went wrong",
  on_retry,
  className,
  children,
}: Props) {
  return (
    <div className={cn("flex h-full items-center justify-center p-8", className)}>
      <div className="text-center max-w-sm">
        <AlertTriangle className="mx-auto h-8 w-8 text-destructive/60" />
        <h3 className="mt-3 text-sm font-medium text-foreground">{message}</h3>
        {children && (
          <p className="mt-1 text-xs text-muted-foreground">{children}</p>
        )}
        {on_retry && (
          <Button variant="outline" size="sm" className="mt-4" onClick={on_retry}>
            Try again
          </Button>
        )}
      </div>
    </div>
  );
}
