import { useEffect, useRef, useState, type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";

import { Button } from "@/components/motion/button/base";
import { Input, type InputClassNames } from "@/components/motion/input";
import { formatBytes } from "@/format";
import { EASE_OUT } from "@/lib/ease";
import { cn } from "@/lib/utils";

/// Rename and delete both used to call `window.prompt` / `window.confirm`.
/// Tauri's webview implements neither, so both actions silently did nothing.
/// Everything here is drawn in the row instead — in the page, under the profile
/// it is about, and never as a modal.

// beUI's buttons are 40px pills at 12px; this window's controls are 32px
// rounded rectangles at 13px, and the compose row above agrees with that.
const ACTION = "h-7 rounded-lg px-2.5 text-[12px]";

// Destructive is not one of beUI's four variants, and it is not a colour to
// invent per call site either — it is the danger token, the same one the meter
// and the delete icon use.
const DESTRUCTIVE = cn(
  "bg-danger text-[oklch(0.99_0.01_25)]",
  "hover:bg-[color-mix(in_oklab,var(--danger)_86%,var(--ink))]",
);

// beUI's field is a 44px pill at 16px with a transparent ground. The panel it
// sits in is sunken, so the field takes the surface back to stay a field.
const FIELD: InputClassNames = {
  field: "h-7 rounded-lg bg-surface",
  input: "px-2 text-[12px] placeholder:text-ink-3",
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
          aria-label={`New name for ${label}`}
          className="w-full"
          classNames={FIELD}
        />
        <div className="mt-2 flex gap-2">
          <Button type="submit" size="sm" className={ACTION}>
            Save name
          </Button>
          <Button type="button" variant="secondary" size="sm" className={ACTION} onClick={onCancel}>
            Cancel
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
      <p className="text-[12px] text-ink-2">
        Delete <strong className="font-semibold text-ink">{label}</strong> and the{" "}
        <strong className="font-mono font-normal text-ink">{formatBytes(bytes)}</strong> in its
        folder. This can’t be undone.
      </p>
      <div className="mt-2 flex gap-2">
        <Button
          ref={confirm}
          type="button"
          size="sm"
          className={cn(ACTION, DESTRUCTIVE)}
          onClick={onConfirm}
        >
          Delete permanently
        </Button>
        <Button type="button" variant="secondary" size="sm" className={ACTION} onClick={onCancel}>
          Keep it
        </Button>
      </div>
    </Panel>
  );
}
