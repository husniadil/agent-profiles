import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/motion/select";
import type { General } from "@/hooks/useGeneral";
import type { Locale } from "@/lib/api";
import { LOCALE_NAMES, useT } from "@/lib/i18n";

/// Same shell as the other two tabs use, for the same reason: a card here has to
/// be the same object as a card there.
const CARD = "shrink-0 rounded-xl border border-hairline bg-surface shadow-card";
const BAND = "flex items-center justify-between gap-4 p-2.5";
const DIVIDED = "border-t border-hairline";
const FIELD = "h-7 rounded-lg px-2.5 text-callout";

/// The sentinel the picker uses for "no explicit choice". `Select` deals in
/// strings and has no way to carry `null`, and an empty string would be
/// indistinguishable from an unset value on the way back.
const SYSTEM = "system";

export function GeneralTab({ general }: { general: General }) {
  const t = useT();

  if (!general.settings) {
    // Blank rather than a spinner, matching `KeepAwakeTab`: the first read lands
    // in milliseconds and a one-frame spinner is noise.
    return (
      <section id="panel-general" role="tabpanel" className="flex min-h-0 flex-1 flex-col p-2" />
    );
  }

  const { locale } = general.settings;

  return (
    <section
      id="panel-general"
      role="tabpanel"
      className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2"
    >
      <div className={CARD}>
        <div className={BAND}>
          <div>
            <p className="text-callout text-ink">{t("general.language.label")}</p>
            <p className="text-sub text-ink-2">{t("general.language.detail")}</p>
          </div>
          <Select
            value={locale ?? SYSTEM}
            onValueChange={(next) =>
              void general.save({ locale: next === SYSTEM ? null : (next as Locale) })
            }
          >
            <SelectTrigger className={`w-44 shrink-0 ${FIELD}`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={SYSTEM}>{t("general.language.system")}</SelectItem>
              {/* Each language named in itself — see `LOCALE_NAMES`. Not passed
                  through `t`, and that is the point. */}
              {(Object.keys(LOCALE_NAMES) as Locale[]).map((id) => (
                <SelectItem key={id} value={id}>
                  {LOCALE_NAMES[id]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        {/* The update band lands here in Task 12. The divider class is defined
            now so that task is one insertion rather than a restructure. */}
        <div className={`${BAND} ${DIVIDED} hidden`} />
      </div>
    </section>
  );
}
