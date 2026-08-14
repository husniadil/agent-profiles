import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/// Merge class names, letting a later Tailwind utility win over an earlier one
/// of the same kind. The vendored beUI components expect this helper under this
/// name; it is the shadcn convention they are written against, and the "later
/// wins" half is what lets a call site override the sizes they ship with.
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
