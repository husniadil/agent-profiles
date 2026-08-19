import { useEffect, useState, type FormEvent } from "react";

import { BudgetMeter } from "@/components/BudgetMeter";
import {
  StatefulButton,
  type ButtonState,
} from "@/components/motion/button/stateful";
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
import { useT } from "@/lib/i18n";

// One band of 32px controls at 13px: the field, the picker and the button are
// the same height and the same type size, so the compose row reads as one thing
// rather than three things of three sizes. beUI ships all three larger.
const CONTROL = "h-7 rounded-lg px-2.5 text-callout";

const FIELD: InputClassNames = {
  field: "h-7 rounded-lg",
  input: "px-2 text-callout placeholder:text-ink-3",
  // The refusal keeps the pulled-toward-ink mix it had as a standalone banner,
  // which is what clears 4.5:1 at this size in both themes.
  errorMessage:
    "px-0 text-callout font-medium text-[color-mix(in_oklab,var(--danger)_70%,var(--ink))]",
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
  const t = useT();
  const [label, setLabel] = useState("");
  // Adding a profile reports next to the form rather than in the page banner.
  // The banner sits above the profile list, which on any populated window is far
  // enough above the form to be scrolled out of sight — a refused label then
  // looks like a button that did nothing at all. Handing it to the field makes
  // that closer still: the field shakes, which is the refusal arriving on the
  // control that caused it.
  const [error, setError] = useState<string | null>(null);

  // Adding a profile is not instant — it creates a directory on disk, and on a
  // slow volume that is long enough for a click to look like it did nothing. So
  // the button carries the operation: idle while there is nothing happening,
  // working while the call is out, and then the verdict.
  //
  // The button says *that* it failed; the field beside it still says *why*, and
  // it is the field the reader has to go back to anyway. Neither replaces the
  // other.
  const [state, setState] = useState<ButtonState>("idle");

  // A refusal is a verdict about one moment. Every visit starts from a clean
  // form rather than from what was true the last time the window was open.
  useEffect(() => {
    setError(null);
    setState("idle");
  }, [visit]);

  // The verdict settles back into the button's own name once it has been read.
  // A timer and not an animation: this window spends its life hidden, and
  // `setTimeout` still runs there — a button left reading "Added" until someone
  // happens to look at it would be describing a profile added minutes ago.
  useEffect(() => {
    if (state === "idle" || state === "loading") return;
    const settle = setTimeout(() => setState("idle"), state === "success" ? 1400 : 2400);
    return () => clearTimeout(settle);
  }, [state]);

  const over =
    budget !== null && budget.limit_bytes !== null && budget.used_bytes > budget.limit_bytes;
  const appLabel = apps.find((app) => app.id === appId)?.label ?? t("compose.thisApp");

  function refuse(message: string): void {
    setError(message);
    setState("error");
  }

  async function submit(event: FormEvent): Promise<void> {
    event.preventDefault();
    if (state === "loading") return;
    const name = label.trim();
    if (!name) {
      refuse(t("compose.needName"));
      return;
    }
    if (!appId) {
      refuse(t("compose.noApp"));
      return;
    }
    setState("loading");
    try {
      await api.addProfile(appId, name);
      setLabel("");
      setError(null);
      setState("success");
      await reload();
    } catch (cause) {
      refuse(api.errorMessage(cause));
    }
  }

  return (
    <section
      aria-labelledby="compose-heading"
      className="shrink-0 rounded-xl border border-hairline bg-surface p-2.5 shadow-card"
    >
      <h2
        id="compose-heading"
        className="mb-1.5 text-sub font-semibold text-ink-2"
      >
        {t("compose.heading")}
      </h2>

      {/* Aligned to the top, not stretched: a refusal grows the field's box
          downward, and the picker and the button have no business growing with
          it. */}
      <form className="flex items-start gap-2" onSubmit={(event) => void submit(event)}>
        <Input
          id="new-label"
          type="text"
          // A display name, not a path: the profile directory is named after a
          // generated id, so length here costs nothing but row space. Fifteen
          // sits on one line beside the running badge without truncating.
          maxLength={15}
          autoComplete="off"
          placeholder={t("compose.namePlaceholder")}
          aria-label={t("compose.nameAria")}
          value={label}
          // A refusal is about the label as it was submitted. The moment it is
          // edited the verdict is stale, and leaving it on screen invites the
          // reader to believe the new label was rejected too.
          onChange={(next) => {
            setLabel(next);
            setError(null);
            // And the button's verdict goes with it, for the same reason: it is
            // about the label that was submitted, not the one being typed.
            if (state !== "loading") setState("idle");
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
              <span className="sr-only">{t("compose.appAria")}</span>
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
        {/* Fixed width: the four labels it swaps between are four different
            lengths, and a button that resizes takes the field beside it with
            it. The label is spoken once, politely, from inside — a person who
            cannot see the spinner still hears that it is working. */}
        <StatefulButton
          type="submit"
          size="sm"
          // Fixed so the label swapping through Add → Adding → Added → Retry
          // never resizes the button under the pointer. 88px is the widest of
          // those, "Adding" with its spinner and padding; the resting "Add"
          // sits comfortably inside. No idle icon: "Add" is the whole label,
          // and a trailing plus only repeated the word.
          className={`${CONTROL} w-[88px] shrink-0`}
          disabled={over}
          state={state}
          loadingText={t("compose.adding")}
          successText={t("compose.added")}
          errorText={t("compose.retry")}
        >
          {t("compose.add")}
        </StatefulButton>
      </form>

      {budget ? <BudgetMeter budget={budget} appLabel={appLabel} /> : null}
    </section>
  );
}
