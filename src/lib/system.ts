import type { T } from "@/lib/i18n";

/// What to call the machine this is running on.
///
/// The socket path limit and the empty state both name the system rather than
/// saying "this computer", because a person recognises their own platform and a
/// number attached to it reads as a fact about their machine. Taken from the
/// webview's own user agent: the backend knows the platform, but not in any form
/// the window is told about, and a command to ask would be a round trip for a
/// word. Falls back to the neutral phrasing on anything unrecognised.
///
/// Takes `t` rather than calling `useT`, because this is called from a `useMemo`
/// body and from `format.ts`, neither of which is a component.
export function systemNames(t: T): { system: string; machine: string } {
  const agent = navigator.userAgent;
  if (agent.includes("Mac OS X") || agent.includes("Macintosh")) {
    return { system: t("system.macos"), machine: t("machine.mac") };
  }
  if (agent.includes("Windows")) {
    return { system: t("system.windows"), machine: t("machine.pc") };
  }
  if (agent.includes("Linux")) {
    return { system: t("system.linux"), machine: t("machine.computer") };
  }
  return { system: t("system.unknown"), machine: t("machine.computer") };
}
