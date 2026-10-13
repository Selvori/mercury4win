// mercury4win/src/components/ui/error_state.tsx
// Reusable error state with retry action

import { type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "./button";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";

interface Props {
  message?: string;
  on_retry?: () => void;
  className?: string;
  children?: ReactNode;
}

export function ErrorState({
  message,
  on_retry,
  className,
  children,
}: Props) {
  const { t } = useTranslation();
  return (
    <div className={cn("flex h-full items-center justify-center p-8", className)}>
      <div className="text-center max-w-sm">
        <AlertTriangle className="mx-auto h-8 w-8 text-destructive/60" />
        <h3 role="alert" className="mt-3 text-sm font-medium text-foreground">{message ?? t("common.error")}</h3>
        {children && (
          <p className="mt-1 text-xs text-muted-foreground">{children}</p>
        )}
        {on_retry && (
          <Button variant="outline" size="sm" className="mt-4" onClick={on_retry}>
            {t("common.retry")}
          </Button>
        )}
      </div>
    </div>
  );
}
