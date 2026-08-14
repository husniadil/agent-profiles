import { cn } from "@/lib/utils";

/// The one input treatment in the window. Height matches the buttons beside it
/// so a compose row is one band rather than three things of three sizes.
export const FIELD = cn(
  "h-8 min-w-0 rounded-lg border border-line bg-surface px-2.5 text-[13px] text-ink",
  "placeholder:text-ink-3",
  "transition-[border-color,box-shadow] duration-150 ease-out",
  "hover:border-ink-4",
  // The focus ring itself is the global `:focus-visible` rule; this is the
  // resting border catching up with it, so a field that is being typed into
  // still looks different from one that is merely being pointed at.
  "focus:border-accent",
  "disabled:cursor-not-allowed disabled:opacity-45",
);
