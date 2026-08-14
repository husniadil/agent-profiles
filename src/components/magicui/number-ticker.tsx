"use client"

import { useEffect, useRef, useState, type ComponentPropsWithoutRef } from "react"
import { useInView, useMotionValue, useSpring } from "motion/react"

import { cn } from "@/lib/utils"

interface NumberTickerProps extends ComponentPropsWithoutRef<"span"> {
  value: number
  startValue?: number
  direction?: "up" | "down"
  delay?: number
  decimalPlaces?: number
}

/// Vendored from MagicUI, with one change: the resting text is the real value.
///
/// Upstream renders `startValue` and only writes the true number once the spring
/// has run. That makes the figure on screen conditional on an animation, and the
/// animation is conditional on the element being in view with rAF running —
/// neither of which holds while this window is hidden, which is most of its life.
/// A count that reads 0 because nothing animated is simply wrong. So the span
/// renders the value, and the spring jumps back to the start before climbing.
export function NumberTicker({
  value,
  startValue = 0,
  direction = "up",
  delay = 0,
  className,
  decimalPlaces = 0,
  ...props
}: NumberTickerProps) {
  const ref = useRef<HTMLSpanElement>(null)
  const motionValue = useMotionValue(direction === "down" ? value : startValue)
  const springValue = useSpring(motionValue, {
    damping: 60,
    stiffness: 100,
  })
  const isInView = useInView(ref, { once: true, margin: "0px" })

  // This window spends most of its life hidden behind a tray icon, and a hidden
  // document gets no animation frames — so a spring started now would jump to
  // its starting value and freeze there, leaving a count of 0 on screen until
  // something else moved. Wait until the window is actually being looked at.
  const [awake, setAwake] = useState(() => !document.hidden)
  useEffect(() => {
    if (awake) return
    const wake = () => {
      if (!document.hidden) setAwake(true)
    }
    document.addEventListener("visibilitychange", wake)
    return () => document.removeEventListener("visibilitychange", wake)
  }, [awake])

  const format = (latest: number) =>
    Intl.NumberFormat("en-US", {
      minimumFractionDigits: decimalPlaces,
      maximumFractionDigits: decimalPlaces,
    }).format(Number(latest.toFixed(decimalPlaces)))

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null

    if (isInView && awake) {
      timer = setTimeout(() => {
        // Jump to the start without animating, then climb. The resting text is
        // already the answer, so this is the one frame where it steps back.
        springValue.jump(direction === "down" ? value : startValue)
        motionValue.set(direction === "down" ? startValue : value)
      }, delay * 1000)
    }

    return () => {
      if (timer !== null) {
        clearTimeout(timer)
      }
    }
  }, [motionValue, isInView, delay, value, direction, startValue])

  useEffect(
    () =>
      springValue.on("change", (latest) => {
        if (ref.current) {
          ref.current.textContent = Intl.NumberFormat("en-US", {
            minimumFractionDigits: decimalPlaces,
            maximumFractionDigits: decimalPlaces,
          }).format(Number(latest.toFixed(decimalPlaces)))
        }
      }),
    [springValue, decimalPlaces]
  )

  return (
    <span
      ref={ref}
      className={cn(
        "inline-block tracking-wider text-black tabular-nums dark:text-white",
        className
      )}
      {...props}
    >
      {format(value)}
    </span>
  )
}
