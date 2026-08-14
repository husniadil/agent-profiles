import type { ComponentProps } from "react";
import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

/// A control whose whole face is a picture, so its name has to reach a screen
/// reader some other way. `title` covers the pointer; `aria-label` covers
/// everything else, and both are required rather than optional.
export function IconButton({
  icon: Icon,
  label,
  tone = "quiet",
  className,
  ...props
}: ComponentProps<"button"> & {
  icon: LucideIcon;
  label: string;
  tone?: "quiet" | "danger";
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      className={cn(
        "grid size-7 place-items-center rounded-md text-ink-2",
        "transition-colors duration-150 ease-out",
        tone === "danger"
          ? "hover:bg-[color-mix(in_oklab,var(--danger)_14%,var(--surface))] hover:text-[color-mix(in_oklab,var(--danger)_70%,var(--ink))]"
          : "hover:bg-line hover:text-ink",
        "active:bg-[color-mix(in_oklab,var(--line)_80%,var(--ink))]",
        "disabled:cursor-not-allowed disabled:opacity-45",
        className,
      )}
      {...props}
    >
      <Icon size={15} strokeWidth={1.75} aria-hidden="true" />
    </button>
  );
}
