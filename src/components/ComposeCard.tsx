import { useEffect, useState, type FormEvent } from "react";
import { Plus } from "lucide-react";

import { BudgetMeter } from "@/components/BudgetMeter";
import { Button } from "@/components/ui/Button";
import { FIELD } from "@/components/ui/Field";
import * as api from "@/lib/api";
import type { AppView, SocketBudget } from "@/lib/api";
import { cn } from "@/lib/utils";

export function ComposeCard({
  apps,
  appId,
  onAppId,
  budget,
  reload,
  visit,
}: {
  apps: AppView[];
  appId: string;
  onAppId: (id: string) => void;
  budget: SocketBudget | null;
  reload: () => Promise<void>;
  visit: number;
}) {
  const [label, setLabel] = useState("");
  // Adding a profile reports next to the form rather than in the page banner.
  // The banner sits above the profile list, which on any populated window is far
  // enough above the form to be scrolled out of sight — a refused label then
  // looks like a button that did nothing at all.
  const [error, setError] = useState<string | null>(null);

  // A refusal is a verdict about one moment. Every visit starts from a clean
  // form rather than from what was true the last time the window was open.
  useEffect(() => setError(null), [visit]);

  const over =
    budget !== null && budget.limit_bytes !== null && budget.used_bytes > budget.limit_bytes;
  const appLabel = apps.find((app) => app.id === appId)?.label ?? "This app";

  async function submit(event: FormEvent): Promise<void> {
    event.preventDefault();
    const name = label.trim();
    if (!name) {
      setError("Enter a name for this profile.");
      return;
    }
    if (!appId) {
      setError("No supported app was found to add a profile to.");
      return;
    }
    try {
      await api.addProfile(appId, name);
      setLabel("");
      setError(null);
      await reload();
    } catch (cause) {
      setError(api.errorMessage(cause));
    }
  }

  return (
    <section
      aria-labelledby="compose-heading"
      className="rounded-xl border border-hairline bg-surface p-3 shadow-card"
    >
      <h2
        id="compose-heading"
        className="mb-2 font-wide text-[11px] font-semibold tracking-[0.06em] text-ink-2 uppercase [font-stretch:112%]"
      >
        New profile
      </h2>

      <form className="flex gap-2" onSubmit={(event) => void submit(event)}>
        <label className="sr-only" htmlFor="new-label">
          Profile name
        </label>
        <input
          id="new-label"
          type="text"
          maxLength={80}
          autoComplete="off"
          placeholder="Name this profile"
          value={label}
          // A refusal is about the label as it was submitted. The moment it is
          // edited the verdict is stale, and leaving it on screen invites the
          // reader to believe the new label was rejected too.
          onChange={(event) => {
            setLabel(event.target.value);
            setError(null);
          }}
          className={cn(FIELD, "flex-1")}
        />

        {/* The picker is only a question when there is more than one answer. */}
        {apps.length > 1 ? (
          <>
            <label className="sr-only" htmlFor="new-app">
              App
            </label>
            <select
              id="new-app"
              value={appId}
              // Switching app switches which data root the meter is about — the
              // two apps sit at different depths under the same root, so the
              // number is not the same twice.
              onChange={(event) => {
                setError(null);
                onAppId(event.target.value);
              }}
              className={cn(FIELD, "max-w-[10rem]")}
            >
              {apps.map((app) => (
                <option key={app.id} value={app.id}>
                  {app.label}
                </option>
              ))}
            </select>
          </>
        ) : null}

        {/* Over the limit the backend has already decided to refuse, so the
            button would only submit into that refusal. */}
        <Button type="submit" tone="accent" disabled={over}>
          <Plus size={14} strokeWidth={2} aria-hidden="true" />
          Add profile
        </Button>
      </form>

      {error ? (
        <p
          role="alert"
          className="mt-2 text-[12px] font-medium"
          style={{ color: "color-mix(in oklab, var(--danger) 70%, var(--ink))" }}
        >
          {error}
        </p>
      ) : null}

      {budget ? <BudgetMeter budget={budget} appLabel={appLabel} /> : null}
    </section>
  );
}
