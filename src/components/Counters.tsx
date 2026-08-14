import { useReducedMotion } from "motion/react";

import { NumberTicker } from "@/components/magicui/number-ticker";
import { formatBytes } from "@/format";
import { cn } from "@/lib/utils";

// The vendored ticker paints itself black on white and white on black. Both are
// wrong here — a count takes the colour of the line it sits in — and its wide
// tracking belongs to a hero number, not to a figure inside a sentence.
const TICKER = "text-inherit tracking-normal dark:text-inherit";

/// A count that arrives by counting up, because the counting *is* the change.
///
/// Motion has to carry meaning or go: under `prefers-reduced-motion` the number
/// is simply the number. The global stylesheet cannot reach this one — the
/// ticker animates in script, not in CSS — so the preference is honoured here
/// rather than fought with.
export function Count({ value, className }: { value: number; className?: string }) {
  const still = useReducedMotion();
  if (still) return <span className={cn("tabular-nums", className)}>{value}</span>;
  return <NumberTicker value={value} className={cn(TICKER, className)} />;
}

/// A size that arrives by counting up, which is the measurement happening.
///
/// The unit is set apart and left still: "1.4" climbing to "2.1" is a directory
/// being walked, but "MB" flickering through "KB" would only be noise. The split
/// is taken from `formatBytes` rather than recomputed, so there is one answer to
/// what a number of bytes is called.
export function ByteCount({ bytes, className }: { bytes: number; className?: string }) {
  const still = useReducedMotion();
  const text = formatBytes(bytes);
  const cut = text.lastIndexOf(" ");
  const figure = text.slice(0, cut);
  const unit = text.slice(cut + 1);

  if (still) return <span className={cn("tabular-nums", className)}>{text}</span>;
  return (
    <span className={cn("tabular-nums", className)}>
      {/* Keyed on the unit: crossing from KB to MB restates the figure rather
          than gliding 900-something down to 0.9. */}
      <NumberTicker
        key={unit}
        value={Number(figure)}
        decimalPlaces={figure.includes(".") ? 1 : 0}
        className={TICKER}
      />{" "}
      {unit}
    </span>
  );
}
