import { CircleAlert } from "lucide-react";

import { edge, readable, wash } from "@/lib/color";

/// The page-level banner: for an action the user asked for and did not get.
///
/// Readings the window offers on its own — the data root, the socket budget —
/// never come through here. A banner that fires when something the user never
/// asked about failed to load teaches them to ignore banners.
export function ErrorBanner({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <div
      role="alert"
      className="flex shrink-0 items-start gap-2 rounded-lg border px-3 py-2 text-body"
      style={{
        color: readable("var(--danger)"),
        background: wash("var(--danger)"),
        borderColor: edge("var(--danger)"),
      }}
    >
      <CircleAlert size={14} strokeWidth={1.75} aria-hidden="true" className="mt-0.5 shrink-0" />
      <span>{message}</span>
    </div>
  );
}
