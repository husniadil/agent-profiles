import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

type Tone = "accent" | "quiet" | "danger";

const BASE =
  "inline-flex h-8 shrink-0 items-center justify-center gap-1.5 rounded-lg px-3 text-[13px] font-medium " +
  "transition-[background-color,border-color,color,opacity] duration-150 ease-out " +
  "disabled:cursor-not-allowed disabled:opacity-45";

// Hover and press both mix toward `--ink`, which is near-black in the light
// theme and near-white in the dark one — so the same class darkens on white and
// lightens on charcoal, and there is no second set of hover colours to keep in
// step with the first.
const TONES: Record<Tone, string> = {
  accent: cn(
    "bg-accent text-accent-ink",
    "enabled:hover:bg-[color-mix(in_oklab,var(--accent)_86%,var(--ink))]",
    "enabled:active:bg-[color-mix(in_oklab,var(--accent)_74%,var(--ink))]",
  ),
  quiet: cn(
    "border border-line bg-surface text-ink-2",
    "enabled:hover:bg-sunken enabled:hover:text-ink",
    "enabled:active:bg-[color-mix(in_oklab,var(--sunken)_88%,var(--ink))]",
  ),
  danger: cn(
    "bg-danger text-[oklch(0.99_0.01_25)]",
    "enabled:hover:bg-[color-mix(in_oklab,var(--danger)_86%,var(--ink))]",
    "enabled:active:bg-[color-mix(in_oklab,var(--danger)_74%,var(--ink))]",
  ),
};

export function Button({
  tone = "quiet",
  className,
  children,
  ...props
}: ComponentProps<"button"> & { tone?: Tone }) {
  return (
    <button className={cn(BASE, TONES[tone], className)} {...props}>
      {children}
    </button>
  );
}
