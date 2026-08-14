import { identityColor, initials } from "@/lib/identity";
import { cn } from "@/lib/utils";

/// The signature element: a profile's initial, in a hue that belongs to it and
/// to nothing else. This is what makes "Work" recognisable before it is read, so
/// this is where the design spends its boldness and everything around it stays
/// quiet.
///
/// The chip is never tinted by state. Running is a badge fixed to its corner in
/// the one green that always means running, so the hue underneath can go on
/// meaning only "this is Work".
export function IdentityChip({
  appId,
  profileId,
  label,
  running,
}: {
  appId: string;
  profileId: string;
  label: string;
  running: boolean;
}) {
  return (
    <span className="relative shrink-0" aria-hidden="true">
      <span
        className={cn(
          "grid size-9 place-items-center rounded-[11px]",
          "font-wide text-[13px] leading-none font-semibold",
          // Archivo's width axis, narrowed a little so two letters sit as
          // comfortably in the square as one does.
          "[font-stretch:88%]",
        )}
        style={{
          background: identityColor(appId, profileId),
          // One ink for all eight hues. The ramp holds a single lightness across
          // the wheel in each theme — 0.62 light, 0.70 dark — so a near-black
          // letter clears 4.5:1 on every rung of it, in both themes, and the
          // chip never has to guess which foreground it needs.
          color: "oklch(0.17 0.02 50)",
        }}
      >
        {initials(label)}
      </span>
      {running ? (
        <span
          className={cn(
            "absolute -right-0.5 -bottom-0.5 size-3 rounded-full bg-live",
            // The cut-out follows the row, so the badge stays a badge on hover
            // instead of growing a pale halo.
            "border-2 border-surface transition-colors duration-150 group-hover:border-sunken",
          )}
        />
      ) : null}
    </span>
  );
}
