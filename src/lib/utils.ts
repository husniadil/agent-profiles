import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/// Merge class names, letting a later Tailwind utility win over an earlier one
/// of the same kind. The vendored MagicUI components expect this helper under
/// this name; it is the shadcn convention they are written against.
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
