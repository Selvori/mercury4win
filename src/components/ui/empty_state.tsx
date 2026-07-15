// mercury4win-linux/src/components/ui/empty_state.tsx
// Reusable empty state component (standardized across app)

import { type ReactNode } from "react";
import { cn } from "@/lib/utils";

interface Props {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon, title, description, action, className }: Props) {
  return (
    <div className={cn("flex h-full items-center justify-center p-8", className)}>
      <div className="text-center max-w-sm">
        {icon && (
          <div className="mx-auto mb-3 text-muted-foreground/40">{icon}</div>
        )}
        <h3 className="text-sm font-medium text-foreground">{title}</h3>
        {description && (
          <p className="mt-1 text-xs text-muted-foreground">{description}</p>
        )}
        {action && <div className="mt-4">{action}</div>}
      </div>
    </div>
  );
}
