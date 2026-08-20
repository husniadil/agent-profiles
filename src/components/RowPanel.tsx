import { useEffect, useRef, useState, type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";

import { Button } from "@/components/motion/button/base";
import { HoldActionButton } from "@/components/motion/hold-action-button";
import { Input, type InputClassNames } from "@/components/motion/input";
import { formatBytes } from "@/format";
import { EASE_OUT } from "@/lib/ease";
import { useT } from "@/lib/i18n";
import { cn } from "@/lib/utils";

/// Rename and delete both used to call `window.prompt` / `window.confirm`.
/// Tauri's webview implements neither, so both actions silently did nothing.
/// Everything here is drawn in the row instead — in the page, under the profile
/// it is about, and never as a modal.

// beUI's buttons are 40px pills at 12px; this window's controls are 32px
// rounded rectangles at 13px, and the compose row above agrees with that.
const ACTION = "h-7 rounded-lg px-2.5 text-callout";

// Destructive is not one of beUI's four variants, and it is not a colour to
// invent per call site either — it is the danger token, the same one the meter
// and the delete icon use.
const DESTRUCTIVE = "bg-danger text-[oklch(0.99_0.01_25)]";

// The sweep that crosses the button while it is held. A fixed deep oxblood
// rather than a mix with `--ink`, which is near-black in one theme and near-
// white in the other: the near-white label has to stay legible over it either
// way. beUI hard-codes `text-sky-400` on the crest that rides the leading edge,
// and it takes no prop — the descendant selector outranks that single class.
const HOLD_FILL =
  "bg-[oklch(0.34_0.115_25)] [&_svg]:text-[oklch(0.34_0.115_25)]";

// Long enough to be a decision rather than a twitch, short enough not to be a
// punishment. Held, not clicked, because this destroys gigabytes for good.
const HOLD_MS = 1400;

// beUI's field is a 44px pill at 16px with a transparent ground. The panel it
// sits in is sunken, so the field takes the surface back to stay a field.
const FIELD: InputClassNames = {
  field: "h-7 rounded-lg bg-surface",
  input: "px-2 text-callout placeholder:text-ink-3",
};

/// The panel is a reveal, not an entrance: it exists because the user asked a
/// question, and the short settle is what ties it to the row it opened under.
function Panel({ danger = false, children }: { danger?: boolean; children: ReactNode }) {
  const still = useReducedMotion();
  const body = (
    <div
      className={cn(
        "mt-1.5 rounded-lg p-2",
        danger
          ? "bg-[color-mix(in_oklab,var(--danger)_10%,var(--surface))]"
          : "bg-sunken",
      )}
    >
      {children}
    </div>
  );
  if (still) return body;
  return (
    <motion.div
      initial={{ opacity: 0, y: -4, filter: "blur(3px)" }}
      animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
      transition={{ duration: 0.18, ease: EASE_OUT }}
    >
      {body}
    </motion.div>
  );
}

export function RenamePanel({
  label,
  onSave,
  onCancel,
}: {
  label: string;
  onSave: (next: string) => void;
  onCancel: () => void;
}) {
  const t = useT();
  const [value, setValue] = useState(label);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    input.current?.select();
  }, []);

  return (
    <Panel>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          const next = value.trim();
          // Nothing to say and nothing to ask: an empty name and the name it
          // already has both mean the same as pressing Cancel.
          if (!next || next === label) {
            onCancel();
            return;
          }
          onSave(next);
        }}
      >
        <Input
          ref={input}
          autoFocus
          type="text"
          maxLength={80}
          value={value}
          onChange={setValue}
          aria-label={t("row.renameNameAria", { name: label })}
          className="w-full"
          classNames={FIELD}
        />
        <div className="mt-2 flex gap-2">
          <Button type="submit" size="sm" className={ACTION}>
            {t("row.saveName")}
          </Button>
          <Button type="button" variant="secondary" size="sm" className={ACTION} onClick={onCancel}>
            {t("row.cancel")}
          </Button>
        </div>
      </form>
    </Panel>
  );
}

export function DeletePanel({
  label,
  bytes,
  onConfirm,
  onCancel,
}: {
  label: string;
  bytes: number;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const t = useT();
  const confirm = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    confirm.current?.focus();
  }, []);

  return (
    <Panel danger>
      {/* The size is set in the mono face here, as it is on the row two lines
          above. It is one of the two facts this sentence turns on, and the row
          is already showing the other — the path — so this says "its folder"
          rather than pushing the sentence to three lines to repeat it. */}
      <p className="text-callout text-ink-2">
        {t("row.deleteBody")
          .split(/(\{\{label\}\}|\{\{bytes\}\})/)
          .map((part, i) =>
            part === "{{label}}" ? (
              <strong key={i} className="font-semibold text-ink">
                {label}
              </strong>
            ) : part === "{{bytes}}" ? (
              <strong key={i} className="font-mono font-normal text-ink">
                {formatBytes(bytes)}
              </strong>
            ) : (
              part
            ),
          )}
      </p>
      <div className="mt-2 flex gap-2">
        {/* Held rather than clicked. A click is one slip away from destroying a
            folder that cannot be brought back, and the second the hold lasts is
            the second in which the reader can still change their mind — letting
            go early cancels, and nothing has happened.

            The sweep is only the clock made visible. Under reduced motion the
            fill stops travelling and simply deepens over the same interval, and
            the hold itself is untouched: deliberateness is the point here, the
            animation was only how long it takes made legible. */}
        <HoldActionButton
          ref={confirm}
          type="horizontal"
          holdDuration={HOLD_MS}
          onHoldComplete={onConfirm}
          aria-label={t("row.delete", { name: label })}
          holdingLabel={t("row.holdingLabel")}
          completeLabel={t("row.completeLabel")}
          fillClassName={HOLD_FILL}
          labelClassName="text-callout font-medium"
          className={cn(
            ACTION,
            DESTRUCTIVE,
            // beUI's own is a 64px pill 288px wide; this is one of a pair of
            // 28px controls. The three labels it swaps between are all laid out
            // at once inside (opacity is the only thing that changes), so the
            // box already sizes to the widest of them and never resizes the row
            // mid-hold. A min-width, not a fixed width: a fixed one was English's
            // width, and five of six locales have a longer label than that. The
            // floor keeps the English pairing; a longer translation grows the box
            // to fit rather than wrapping and clipping under `overflow: hidden`.
            "h-7 min-w-[132px] rounded-lg px-2.5",
            "focus-visible:ring-danger focus-visible:ring-offset-1",
          )}
        >
          {t("row.holdToDelete")}
        </HoldActionButton>
        <Button type="button" variant="secondary" size="sm" className={ACTION} onClick={onCancel}>
          {t("row.keepIt")}
        </Button>
      </div>
    </Panel>
  );
}
