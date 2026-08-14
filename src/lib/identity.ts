/// Colour is identity, not decoration.
///
/// The product exists to keep sign-ins apart, so every profile carries a hue of
/// its own — the same hue on every render and after every restart, because it is
/// derived from the profile's id rather than from its position in a list. That
/// stability is the whole point: a hue that moved when a profile was deleted
/// above it would teach the reader nothing.

const RAMP = 8;

/// FNV-1a, 32-bit. Chosen for being short enough to read and stable enough to
/// promise: the same string gives the same number on every machine, which a
/// hash borrowed from a runtime (`String.prototype` tricks, `Math.random`
/// seeding) would not.
function hash(text: string): number {
  let value = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    value ^= text.charCodeAt(index);
    value = Math.imul(value, 0x01000193);
  }
  return value >>> 0;
}

/// Which rung of the identity ramp a profile sits on, 0 through 7.
///
/// Keyed on the app as well as the profile so that "Work" under two different
/// agents are two different things on screen, which is exactly what they are.
export function identityIndex(appId: string, profileId: string): number {
  return hash(`${appId}:${profileId}`) % RAMP;
}

/// The CSS value for a profile's hue, light and dark handled by the token.
export function identityColor(appId: string, profileId: string): string {
  return `var(--id-${identityIndex(appId, profileId)})`;
}

/// The letters the chip carries: one, or two when the label is two words.
///
/// Spread rather than indexed so an emoji or an accented letter is one
/// character rather than half of a surrogate pair.
export function initials(label: string): string {
  const words = label.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "?";
  const first = [...words[0]][0] ?? "?";
  if (words.length === 1) return first.toLocaleUpperCase();
  const second = [...words[1]][0] ?? "";
  return `${first}${second}`.toLocaleUpperCase();
}
