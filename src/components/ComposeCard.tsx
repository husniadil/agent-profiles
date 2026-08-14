import { useEffect, useState, type FormEvent } from "react";
import { Plus } from "lucide-react";

import { BudgetMeter } from "@/components/BudgetMeter";
import { Button } from "@/components/motion/button/base";
import { Input, type InputClassNames } from "@/components/motion/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/motion/select";
import * as api from "@/lib/api";
import type { AppView, SocketBudget } from "@/lib/api";

// One band of 32px controls at 13px: the field, the picker and the button are
// the same height and the same type size, so the compose row reads as one thing
// rather than three things of three sizes. beUI ships all three larger.
const CONTROL = "h-8 rounded-lg px-3 text-[13px]";

const FIELD: InputClassNames = {
  field: "h-8 rounded-lg",
  input: "px-2.5 text-[13px] placeholder:text-ink-3",
  // The refusal keeps the pulled-toward-ink mix it had as a standalone banner,
  // which is what clears 4.5:1 at this size in both themes.
  errorMessage:
    "px-0 text-[12px] font-medium text-[color-mix(in_oklab,var(--danger)_70%,var(--ink))]",
};

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
  // looks like a button that did nothing at all. Handing it to the field makes
  // that closer still: the field shakes, which is the refusal arriving on the
  // control that caused it.
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

      {/* Aligned to the top, not stretched: a refusal grows the field's box
          downward, and the picker and the button have no business growing with
          it. */}
      <form className="flex items-start gap-2" onSubmit={(event) => void submit(event)}>
        <Input
          id="new-label"
          type="text"
          maxLength={80}
          autoComplete="off"
          placeholder="Name this profile"
          aria-label="Profile name"
          value={label}
          // A refusal is about the label as it was submitted. The moment it is
          // edited the verdict is stale, and leaving it on screen invites the
          // reader to believe the new label was rejected too.
          onChange={(next) => {
            setLabel(next);
            setError(null);
          }}
          error={error ?? false}
          className="flex-1"
          classNames={FIELD}
        />

        {/* The picker is only a question when there is more than one answer. */}
        {apps.length > 1 ? (
          <Select
            value={appId}
            // Switching app switches which data root the meter is about — the
            // two apps sit at different depths under the same root, so the
            // number is not the same twice.
            onValueChange={(next) => {
              setError(null);
              onAppId(next);
            }}
            className="w-40 shrink-0"
          >
            {/* beUI's trigger takes no label prop, so the name is given the way
                any button gets one: inside it, out of sight. The accessible
                name reads "App, <chosen app>". */}
            <SelectTrigger className={CONTROL}>
              <span className="sr-only">App</span>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {apps.map((app) => (
                <SelectItem key={app.id} value={app.id}>
                  {app.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : null}

        {/* Over the limit the backend has already decided to refuse, so the
            button would only submit into that refusal. */}
        <Button type="submit" size="sm" className={CONTROL} disabled={over}>
          <Plus size={14} strokeWidth={2} aria-hidden="true" />
          Add profile
        </Button>
      </form>

      {budget ? <BudgetMeter budget={budget} appLabel={appLabel} /> : null}
    </section>
  );
}
