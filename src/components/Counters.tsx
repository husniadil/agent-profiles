import { useReducedMotion } from "motion/react";

import { AnimatedNumber } from "@/components/motion/animated-number";
import { formatBytes } from "@/format";
import { useAwake } from "@/lib/hooks/use-awake";
import { cn } from "@/lib/utils";

/// A count that arrives by counting up, because the counting *is* the change.
///
/// Two things can make the animation the wrong answer, and both end the same
/// way — the plain number, correct and still:
///
///  * `prefers-reduced-motion`, because motion has to carry meaning or go. The
///    global stylesheet cannot reach this one; it animates in script, not CSS.
///  * a hidden window, because `AnimatedNumber` displays whatever its animation
///    has reached and starts that at 0. With no frames to run, 0 is what the
///    reader would find waiting for them.
///
/// Once awake the ticker is handed `startOnView={false}`: the component's own
/// gate is an intersection observer, which answers a question about scrolling
/// that this window — one screen, no scroll to speak of — never asks.
export function Count({ value, className }: { value: number; className?: string }) {
  const still = useReducedMotion();
  const awake = useAwake();

  if (still || !awake) return <span className={cn("tabular-nums", className)}>{value}</span>;
  return (
    <AnimatedNumber
      value={value}
      startOnView={false}
      format={(n) => String(Math.round(n))}
      className={className}
    />
  );
}

/// A size that arrives by counting up, which is the measurement happening.
///
/// The unit is set apart and left still: "1.4" climbing to "2.1" is a directory
/// being walked, but "MB" flickering through "KB" would only be noise. The split
/// is taken from `formatBytes` rather than recomputed, so there is one answer to
/// what a number of bytes is called, and the ticker is told how to render a
/// figure rather than handed one to re-derive.
export function ByteCount({ bytes, className }: { bytes: number; className?: string }) {
  const still = useReducedMotion();
  const awake = useAwake();
  const text = formatBytes(bytes);
  const cut = text.lastIndexOf(" ");
  const figure = text.slice(0, cut);
  const unit = text.slice(cut + 1);
  const places = figure.includes(".") ? 1 : 0;

  if (still || !awake) return <span className={cn("tabular-nums", className)}>{text}</span>;
  return (
    <span className={cn("tabular-nums", className)}>
      {/* Keyed on the unit: crossing from KB to MB restates the figure rather
          than gliding 900-something down to 0.9. */}
      <AnimatedNumber
        key={unit}
        value={Number(figure)}
        startOnView={false}
        format={(n) => n.toFixed(places)}
      />{" "}
      {unit}
    </span>
  );
}
