import { clsx, type ClassValue } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

/// tailwind-merge only knows Tailwind's own class names. This window's type
/// scale is its own — `text-body`, `text-sub`, `text-caption` — and to a merger
/// that has never heard of them they look like colours, because `text-<name>`
/// is overwhelmingly a colour in stock Tailwind. So every one of them was being
/// dropped from any class list that also named a colour: `text-body text-ink`
/// arrived in the DOM as `text-ink`, and the element fell back to inheriting a
/// size. Silent, and invisible in the source — the class is right there in the
/// file that produced the element.
///
/// Naming the scale here is what stops that. The two groups are then distinct,
/// and a size and a colour can sit in the same list without one deleting the
/// other.
const twMerge = extendTailwindMerge({
  extend: {
    classGroups: {
      "font-size": [{ text: ["title", "body", "callout", "sub", "caption"] }],
    },
  },
});

/// Merge class names, letting a later Tailwind utility win over an earlier one
/// of the same kind. The vendored beUI components expect this helper under this
/// name; it is the shadcn convention they are written against, and the "later
/// wins" half is what lets a call site override the sizes they ship with.
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
