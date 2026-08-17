import { createContext, useContext, useMemo, type ReactNode } from "react";

import type { Locale } from "@/lib/api";
import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { id } from "./id";
import { ja } from "./ja";
import { pt } from "./pt";

/// Every key of the English dictionary, each mapping to a plain `string`.
///
/// Deliberately NOT `typeof en`: `en` is `as const`, so `typeof en` types each
/// value as its exact English literal (`"Version {{version}}"`), which would
/// reject every translation — a locale file could only ever restate English.
/// Mapping over the keys keeps the contract that matters — a locale missing a
/// key, or inventing one, is a build error — while letting each value be any
/// string. The keys are the shared shape; the values are what differ per
/// language.
export type Strings = { [K in keyof typeof en]: string };
export type Key = keyof Strings;

/// The picker's options, each named in its own language. A list of languages
/// where every entry is written in the language you are currently reading is
/// useless to the one person who needs it — someone who cannot read the current
/// one.
export const LOCALE_NAMES: Record<Locale, string> = {
  en: "English",
  id: "Bahasa Indonesia",
  ja: "日本語",
  de: "Deutsch",
  es: "Español",
  pt: "Português",
};

/// Static imports, all six at once. Six flat records of ~80 short strings is a
/// few kilobytes in a bundle that ships inside the binary; a dynamic import
/// would buy nothing and cost both a top-level `await` here and a loading state
/// on the one thing that must never flash.
const DICTIONARIES: Record<Locale, Strings> = { en, id, ja, de, es, pt };

export type T = (key: Key, vars?: Record<string, string | number>) => string;

/// English rather than the key, when a string is missing at runtime.
///
/// This cannot happen through the type system — every dictionary is annotated
/// `: Strings` — but it can happen through a hand-edited `general.json` naming a
/// locale, or a dictionary file that shipped truncated. Rendering `row.open` to
/// a user is strictly worse than rendering English.
function translate(dictionary: Strings, key: Key, vars?: Record<string, string | number>) {
  const template = dictionary[key] ?? en[key];
  if (!vars) return template;
  return Object.entries(vars).reduce(
    // `split(...).join(...)` rather than `String.prototype.replaceAll`: the
    // project's `tsconfig.json` targets `lib: ["ES2020", ...]`, and
    // `replaceAll` is ES2021. Same effect for a literal (non-regex) needle.
    (text, [name, value]) => text.split(`{{${name}}}`).join(String(value)),
    template as string,
  );
}

const I18nContext = createContext<T | null>(null);

export function I18nProvider({
  locale,
  children,
}: {
  locale: Locale;
  children: ReactNode;
}) {
  const t = useMemo<T>(() => {
    const dictionary = DICTIONARIES[locale] ?? en;
    return (key, vars) => translate(dictionary, key, vars);
  }, [locale]);

  return <I18nContext.Provider value={t}>{children}</I18nContext.Provider>;
}

export function useT(): T {
  const t = useContext(I18nContext);
  if (!t) throw new Error("useT must be used inside an I18nProvider");
  return t;
}
