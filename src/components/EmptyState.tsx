import { PackageOpen } from "lucide-react";

import type { AppView } from "@/lib/api";
import { systemNames } from "@/lib/system";

/// The window with nothing to manage.
///
/// Nothing installed is the only case worth explaining. With one app working,
/// the other's absence is not an error — it is simply not installed. The apps
/// are named from what the backend actually looked for rather than from a fixed
/// list here, so this sentence cannot come to describe a different set of apps
/// than the one the app supports.
export function EmptyState({ apps }: { apps: AppView[] }) {
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
        Nothing to open yet
      </p>
      <p className="mx-auto mt-1.5 max-w-[46ch] text-body text-ink-2">
        Agent Profiles runs the coding agents already installed on {systemNames().machine}
        {names ? ` — ${names}` : ""}. Install one, then reopen this window.
      </p>
      <p className="mt-3 font-mono text-sub text-ink-2">{apps.length} apps found</p>
    </div>
  );
}
