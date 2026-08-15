/// What to call the machine this is running on.
///
/// The socket path limit and the empty state both name the system rather than
/// saying "this computer", because a person recognises their own platform and a
/// number attached to it reads as a fact about their machine. Taken from the
/// webview's own user agent: the backend knows the platform, but not in any form
/// the window is told about, and a command to ask would be a round trip for a
/// word. Falls back to the neutral phrasing on anything unrecognised.
export function systemNames(): { system: string; machine: string } {
  const agent = navigator.userAgent;
  if (agent.includes("Mac OS X") || agent.includes("Macintosh")) {
    return { system: "macOS", machine: "this Mac" };
  }
  if (agent.includes("Windows")) return { system: "Windows", machine: "this PC" };
  if (agent.includes("Linux")) return { system: "Linux", machine: "this computer" };
  return { system: "this system", machine: "this computer" };
}
