import { FolderOpen } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import { ByteCount, Count } from "@/components/Counters";
import { Tooltip } from "@/components/motion/tooltip";
import { summary } from "@/format";
import { useT } from "@/lib/i18n";
import { shortenRoot, usePathNames } from "@/lib/paths";

/// The one line that replaces the eyebrow, the h1 and the lede.
///
/// The counts are spoken once, quietly, in a region of their own. The visible
/// figures are ticked into place by script, and a live region wrapped around
/// them would be re-read on every frame of that; this way a screen reader hears
/// one settled sentence and only when the sentence has actually changed.
export function StatusStrip({
  profiles,
  running,
  bytes,
  approximate,
  onError,
}: {
  profiles: number;
  running: number;
  bytes: number | null;
  /// At least one profile's walk could not reach every entry. The total is real
  /// but short by an unknown amount, so it is marked and the mark is explained
  /// — a bare glyph in a status strip is a riddle, not a disclosure.
  approximate: boolean;
  onError: (error: unknown) => void;
}) {
  const t = useT();
  const { dataRoot, homePath } = usePathNames();

  return (
    // Not sticky any more: the list scrolls inside its own frame, so there is
    // nothing left for this to stick over — only a stacking context and an
    // overlap waiting to happen. `px-5` lines its content up with the first
    // row's chip.
    <header className="flex h-10 shrink-0 items-center justify-between gap-3 border-b border-hairline bg-surface px-5">
      <p aria-hidden="true" className="flex items-baseline gap-1.5 text-callout text-ink-2">
        <Count value={profiles} className="font-mono text-ink" />
        <span>{profiles === 1 ? t("status.profile") : t("status.profiles")}</span>
        <Separator />
        <Count value={running} className="font-mono text-ink" />
        <span>{t("status.running")}</span>
        {/* Absent until every row has reported: a total that counts half the
            profiles is a wrong number stated confidently. */}
        {bytes !== null ? (
          <>
            <Separator />
            {approximate ? (
              <Tooltip
                content={t("status.onDiskApproxWhy")}
                side="bottom"
                className="max-w-[min(320px,calc(100vw-16px))] whitespace-normal text-sub font-normal"
              >
                <span className="flex items-baseline gap-1.5">
                  <ByteCount bytes={bytes} approximate className="font-mono text-ink" />
                  <span>{t("status.onDisk")}</span>
                </span>
              </Tooltip>
            ) : (
              <>
                <ByteCount bytes={bytes} className="font-mono text-ink" />
                <span>{t("status.onDisk")}</span>
              </>
            )}
          </>
        ) : null}
      </p>
      {/* The tooltip is `aria-hidden`, so the spoken sentence carries the
          shortfall in words of its own rather than a mark nobody hears. */}
      <span className="sr-only" aria-live="polite">
        {summary(t, profiles, running, bytes, approximate)}
      </span>

      {/* The strip names the folder; this is the only way to actually get to it. */}
      {/* The label on screen is an abbreviation, so the whole root is said in
          two places that reach two different audiences: the tooltip, which
          opens on focus as well as hover — a button is already a tab stop, so
          this costs nothing — and the button's own accessible name, since beUI's
          tooltip is `aria-hidden` and nothing in it is ever read aloud. */}
      {dataRoot ? (
        <Tooltip
          content={dataRoot}
          side="bottom"
          wrapperClassName="flex min-w-0 shrink"
          className="max-w-[min(420px,calc(100vw-16px))] break-all whitespace-normal font-mono text-sub font-normal"
        >
          <button
            type="button"
            onClick={() => {
              // Unlike reading the root, this one the user asked for.
              void revealItemInDir(dataRoot).catch(onError);
            }}
            className="flex min-w-0 shrink items-center gap-1.5 rounded-md px-1.5 py-1 text-ink-2 transition-colors duration-150 ease-out hover:bg-sunken hover:text-ink"
          >
            <FolderOpen size={13} strokeWidth={1.75} aria-hidden="true" className="shrink-0" />
            <span className="sr-only">
              {t("status.revealFolder", { path: dataRoot })}
            </span>
            <bdi aria-hidden="true" className="truncate font-mono text-sub">
              {shortenRoot(dataRoot, homePath)}
            </bdi>
          </button>
        </Tooltip>
      ) : null}
    </header>
  );
}

function Separator() {
  return (
    <span aria-hidden="true" className="px-0.5 text-ink-4">
      ·
    </span>
  );
}
