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
    <div className="rounded-xl border border-hairline bg-surface px-6 py-10 text-center shadow-card">
      <PackageOpen
        size={22}
        strokeWidth={1.5}
        aria-hidden="true"
        className="mx-auto text-ink-3"
      />
      <p className="mt-3 font-wide text-[15px] font-semibold text-ink [font-stretch:105%]">
        Nothing to open yet
      </p>
      <p className="mx-auto mt-1.5 max-w-[46ch] text-[13px] text-ink-2">
        Agent Profiles runs the coding agents already installed on {systemNames().machine}
        {names ? ` — ${names}` : ""}. Install one, then reopen this window.
      </p>
      <p className="mt-3 font-mono text-[11px] text-ink-2">{apps.length} apps found</p>
    </div>
  );
}
