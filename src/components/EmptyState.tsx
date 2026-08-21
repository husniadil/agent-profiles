import { PackageOpen } from "lucide-react";

import type { AppView } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { systemNames } from "@/lib/system";

/// The window with nothing to manage.
///
/// Nothing installed is the only case worth explaining. With one app working,
/// the other's absence is not an error — it is simply not installed. The apps
/// are named from what the backend actually looked for rather than from a fixed
/// list here, so this sentence cannot come to describe a different set of apps
/// than the one the app supports.
export function EmptyState({ apps }: { apps: AppView[] }) {
  const t = useT();
  const names = apps.map((app) => app.label).join(", ");
  return (
    // Takes the list card's slot and the height that comes with it. Centring
    // the block vertically is what supplies the air now, so the fixed padding
    // no longer has to carry it alone.
    <div className="grid min-h-0 flex-1 place-content-center overflow-y-auto rounded-xl border border-hairline bg-surface px-6 py-8 text-center shadow-card">
      <PackageOpen
        size={22}
        strokeWidth={1.5}
        aria-hidden="true"
        className="mx-auto text-ink-3"
      />
      <p className="mt-3 text-title font-semibold text-ink">
        {t("empty.title")}
      </p>
      <p className="mx-auto mt-1.5 max-w-[46ch] text-body text-ink-2">
        {t("empty.body", {
          machine: systemNames(t).machine,
          names: names ? ` — ${names}` : "",
        })}
      </p>
      {/* These are the apps this tool supports, not ones found on the machine:
          this screen only renders when nothing is installed, so a "found" count
          would always be the whole list under a heading that says nothing was
          found — a contradiction. "supported" is the reading that stays true. */}
      <p className="mt-3 font-mono text-sub text-ink-2">
        {t("empty.appsSupported", { count: apps.length })}
      </p>
      {/* What was looked for, and where. The same reasons the tray shows: with
          nothing installed, "install one of these" is only actionable once the
          reader knows which of them this machine already failed to find. Written
          by the backend, which is the side that did the looking, so it is not
          translated here. */}
      {apps.some((app) => app.unavailable) ? (
        <ul className="mx-auto mt-2 max-w-[46ch] space-y-1 text-left text-sub text-ink-2">
          {apps
            .filter((app) => app.unavailable)
            .map((app) => (
              <li key={app.id}>{app.unavailable}</li>
            ))}
        </ul>
      ) : null}
    </div>
  );
}
