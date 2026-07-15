// mercury4win-linux/src/components/ui/loading_state.tsx
// Reusable loading spinner component

import { Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

interface Props {
  message?: string;
  className?: string;
  size?: "sm" | "md" | "lg";
}

const sizes = { sm: "h-4 w-4", md: "h-6 w-6", lg: "h-8 w-8" };

export function LoadingState({ message, className, size = "md" }: Props) {
  return (
    <div className={cn("flex h-full items-center justify-center", className)}>
      <div className="flex flex-col items-center gap-2 text-center">
        <Loader2
          className={cn(sizes[size], "animate-spin text-muted-foreground")}
        />
        {message && (
          <p className="text-xs text-muted-foreground">{message}</p>
        )}
      </div>
    </div>
  );
}
