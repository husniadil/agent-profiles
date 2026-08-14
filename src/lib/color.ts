/// Two mixes, so a state colour can be legible without a second palette.
///
/// `--ink` is near-black in the light theme and near-white in the dark one, so
/// the same expression darkens a hue on white and lightens it on charcoal. One
/// rule, two correct answers, and nothing to keep in step by hand.
///
/// Mixed in oklab and never in oklch. The tokens on the other side of every one
/// of these mixes are all but colourless — white, charcoal, ink — and a
/// colourless colour still carries a hue angle of zero, which oklch dutifully
/// interpolates toward. That is how a 12% wash of the running green came out
/// pink. Oklab has no hue axis to interpolate, so a green stays a green and only
/// its lightness moves.

/// A state colour pulled toward the ink, so small text set in it clears 4.5:1
/// in both themes. The hue is unchanged — `--color-live` still reads as the
/// running green, it simply stops being a pale green on a pale ground.
/// The mix is deliberately gentle. Pulled much further than this the dark theme
/// pays for it: `--ink` there is near-white, and a green dragged most of the way
/// to white is a green that has to be told apart from an amber dragged the same
/// distance. 84% keeps the chroma that carries the meaning and still clears the
/// ratio on the light side, where the mix is doing the real work.
export function readable(token: string, strength = 84): string {
  return `color-mix(in oklab, ${token} ${strength}%, var(--ink))`;
}

/// The same colour as a wash, for the ground under a tag or a banner.
export function wash(token: string, strength = 12): string {
  return `color-mix(in oklab, ${token} ${strength}%, var(--surface))`;
}

/// The same colour as a hairline: enough to bound a shape, not enough to shout.
export function edge(token: string, strength = 30): string {
  return `color-mix(in oklab, ${token} ${strength}%, var(--surface))`;
}
