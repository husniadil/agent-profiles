import { useEffect, useRef, useState, type ReactNode } from "react";
import { useReducedMotion } from "motion/react";

import { BlurFade } from "@/components/magicui/blur-fade";
import { Button } from "@/components/ui/Button";
import { FIELD } from "@/components/ui/Field";
import { formatBytes } from "@/format";
import { cn } from "@/lib/utils";

/// Rename and delete both used to call `window.prompt` / `window.confirm`.
/// Tauri's webview implements neither, so both actions silently did nothing.
/// Everything here is drawn in the row instead — in the page, under the profile
/// it is about, and never as a modal.

/// The panel is a reveal, not an entrance: it exists because the user asked a
/// question, and the short settle is what ties it to the row it opened under.
function Panel({ danger = false, children }: { danger?: boolean; children: ReactNode }) {
  const still = useReducedMotion();
  const body = (
    <div
      className={cn(
        "mt-2 rounded-lg p-2.5",
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
    <BlurFade duration={0.18} offset={4} blur="3px">
      {body}
    </BlurFade>
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
        <input
          ref={input}
          autoFocus
          type="text"
          maxLength={80}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          aria-label={`New name for ${label}`}
          className={cn(FIELD, "w-full")}
        />
        <div className="mt-2 flex gap-2">
          <Button type="submit" tone="accent">
            Save name
          </Button>
          <Button type="button" onClick={onCancel}>
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
      <p className="text-[13px] text-ink-2">
        Delete <strong className="font-semibold text-ink">{label}</strong> and the{" "}
        <strong className="font-mono font-normal text-ink">{formatBytes(bytes)}</strong> in its
        folder. This can’t be undone.
      </p>
      <div className="mt-2 flex gap-2">
        <Button ref={confirm} type="button" tone="danger" onClick={onConfirm}>
          Delete permanently
        </Button>
        <Button type="button" onClick={onCancel}>
          Keep it
        </Button>
      </div>
    </Panel>
  );
}
