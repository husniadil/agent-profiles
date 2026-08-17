/// Size overrides for vendored beUI controls that more than one call site
/// needs. A control used in one place keeps its override next to that use — the
/// slider and the radio on the Keep Awake tab do — but a control the window
/// wears twice has to be one decision, or the two copies drift apart the first
/// time either is touched.
///
/// The overrides are descendant selectors because `className` lands on the
/// wrapper, not on the parts. One class beats one, two beat one, and the
/// vendored file stays byte-identical to beui.dev so it can be re-synced.

/// beUI's switch is 28×48 with a 20px thumb — a page's control, and by some way
/// the largest thing in a window whose chrome rows are 40px tall. Brought down
/// to 16×28 with a 12px thumb: the same scale as the 14px radio on the other
/// tab, and small enough that the two-line sentence beside it stays the thing
/// the eye lands on first.
///
/// The travel is what has to survive the shrink — 28 wide, less 2px of padding
/// a side, less a 12px thumb, leaves 12px of visible movement. A switch whose
/// thumb barely moves reads as a decoration rather than as a state.
export const SWITCH =
  "[&>button]:h-4 [&>button]:w-7 [&>button]:px-0.5 " +
  "[&>button>div]:size-3 [&>button>div>div]:size-3";
