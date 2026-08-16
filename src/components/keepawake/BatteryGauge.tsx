/// The system's own battery glyph, filled to a level.
///
/// Drawn rather than taken from the icon set: lucide ships `battery-low`,
/// `battery-medium` and `battery-full` as three separate pictures, and a
/// threshold that slides continuously would jump between them in thirds. The
/// point of putting it beside the slider is that the picture and the number say
/// the same thing at every step.
///
/// Proportions follow the menu-bar battery — a rounded shell, a cap on the
/// right, and an inset fill that shortens from the right as the level drops.
export function BatteryGauge({ percent, className }: { percent: number; className?: string }) {
  const level = Math.min(Math.max(percent, 0), 100) / 100;
  // The fill spans the shell's inner width. It keeps a sliver at zero rather
  // than vanishing, so the glyph still reads as a battery at the bottom of the
  // range instead of as an empty outline.
  const inner = 18.5;
  const width = Math.max(level * inner, level > 0 ? 1.5 : 0);

  return (
    <svg
      viewBox="0 0 27 13"
      className={className}
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      {/* Shell */}
      <rect
        x="0.6"
        y="0.6"
        width="21.8"
        height="11.8"
        rx="3.4"
        stroke="currentColor"
        strokeOpacity="0.45"
        strokeWidth="1.2"
      />
      {/* Cap */}
      <path
        d="M24.2 4.6v3.8c1-.4 1.6-1 1.6-1.9s-.6-1.5-1.6-1.9Z"
        fill="currentColor"
        fillOpacity="0.45"
      />
      {/* Level. Rounded to match the shell's inner corner, and drawn only when
          there is something to draw — a zero-width rounded rect renders as a
          dot in some engines. */}
      {width > 0 ? (
        <rect x="2.25" y="2.25" width={width} height="8.5" rx="1.8" fill="currentColor" />
      ) : null}
    </svg>
  );
}
