import type { CSSProperties } from "react";

export const TEXT_SHIMMER_KEYFRAMES =
  "@keyframes beui-text-shimmer{from{background-position:200% 0}to{background-position:-200% 0}}";

// beUI ships this gradient written against `--muted-foreground` / `--foreground`.
// This project's `@theme inline` block maps the shadcn names onto its palette
// for *utilities*, and an inline theme deliberately emits no custom properties —
// so those two names resolve to nothing here, the whole `linear-gradient()` is
// invalid, and `bg-clip-text` over no background paints transparent text. The
// palette's own tokens are what is actually in the cascade, in both themes.
export const TEXT_SHIMMER_CLASS_NAME =
  "bg-[length:200%_100%] bg-clip-text text-transparent bg-[linear-gradient(110deg,var(--ink-2)_30%,var(--ink)_50%,var(--ink-2)_70%)]";

export function textShimmerStyle(duration: number): CSSProperties {
  return {
    animation: `beui-text-shimmer ${duration}s linear infinite`,
  };
}
