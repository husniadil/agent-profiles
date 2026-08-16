import { AnimatedBadge } from "@/components/motion/animated-badge";
import { edge, readable, wash } from "@/lib/color";

// beUI's badge is a 24px pill at 11px; the rows this sits in are dense, so this
// takes the caption size — already the smallest step on the window's scale. The
// status colours it ships are Tailwind's emerald and amber; this palette has its
// own, set as styles so one token drives the tag, the dot and the meter.
//
// Sentence case, and that is the whole fix for a chip that read as oversized
// while measuring 10px against 11px neighbours. It was the only upper-case text
// in the entire window: capitals put every letter at cap height, so the word
// became a solid block heavier than the 11px sentence beside it, and the
// tracking that capitals need widened it further. Nothing else here shouts —
// section labels included, which carry weight instead — so this no longer does.
const TAG = "h-auto rounded-full px-1.5 py-px text-caption";

/// A fact about state, never about identity.
///
/// State is kept apart from a profile's own hue so a colour never has to mean
/// two things at once: live is always the same green, a shared sign-in is always
/// the warning amber, and neither one ever tints an identity chip. Colour is not
/// the message either — the word is right there beside it, which is also the only
/// thing beUI's icon would repeat, so it is turned off.
///
/// Shared between the profile rows and the watch list on purpose. "Running" and
/// "Working" are the same kind of claim about the same kind of thing, and two
/// tabs that spell that differently would be two vocabularies for one idea.
export function StateTag({
  token,
  status,
  children,
}: {
  token: string;
  status: "success" | "warning";
  children: string;
}) {
  return (
    <AnimatedBadge
      status={status}
      size="sm"
      showIcon={false}
      className={TAG}
      style={{ color: readable(token), background: wash(token), borderColor: edge(token) }}
    >
      {children}
    </AnimatedBadge>
  );
}
