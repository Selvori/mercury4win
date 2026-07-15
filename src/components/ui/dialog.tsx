// mercury4win-linux/src/components/ui/dialog.tsx
// simplified shadcn/ui dialog using native <dialog> element

import { type ReactNode, useEffect, useRef } from "react";
import { cn } from "@/lib/utils";

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
}

export function Dialog({ open, onOpenChange, children }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) el.showModal();
    else if (!open && el.open) el.close();
  }, [open]);

  function handleClose() {
    onOpenChange(false);
  }

  return (
    <dialog
      ref={ref}
      onClose={handleClose}
      className={cn(
        "rounded-xl border border-border bg-popover p-0 shadow-xl backdrop:bg-black/50",
        "open:flex open:flex-col",
        "max-h-[85vh] max-w-lg w-full",
      )}
    >
      {children}
    </dialog>
  );
}

export function DialogContent({
  children,
  className,
  ...props
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("p-6", className)} {...props}>
      {children}
    </div>
  );
}

export function DialogHeader({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return <div className={cn("mb-4", className)}>{children}</div>;
}

export function DialogTitle({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <h2 className={cn("text-lg font-semibold text-foreground", className)}>
      {children}
    </h2>
  );
}

export function DialogFooter({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("-mx-6 -mb-6 mt-6 flex justify-end gap-2 rounded-b-xl border-t bg-muted/50 p-4", className)}>
      {children}
    </div>
  );
}
