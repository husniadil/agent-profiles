import type { Strings } from "./index";
import { en } from "./en";

/// Not yet translated. Annotated `: Strings` from the start, so the moment a key
/// is added to `en.ts` and not here, `pnpm build` says so.
export const id: Strings = { ...en };
