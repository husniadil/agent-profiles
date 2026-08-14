import type { ReactNode } from "react";

import { readable, wash, edge } from "@/lib/color";

/// A fact about state, never about identity.
///
/// State is kept apart from the profile's own hue so a colour never has to mean
/// two things at once: running is always the live green, a shared sign-in is
/// always the warning amber, and neither one ever tints the identity chip.
/// Colour is not the message either — the word is right there beside it.
export function Tag({ token, children }: { token: string; children: ReactNode }) {
  return (
    <span
      className="shrink-0 rounded-full border px-1.5 py-px text-[10px] font-medium tracking-[0.04em] whitespace-nowrap uppercase"
      style={{ color: readable(token), background: wash(token), borderColor: edge(token) }}
    >
      {children}
    </span>
  );
}
