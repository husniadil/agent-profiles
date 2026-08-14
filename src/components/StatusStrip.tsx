import { FolderOpen } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import { ByteCount, Count } from "@/components/Counters";
import { statusLine } from "@/format";
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
  onError,
}: {
  profiles: number;
  running: number;
  bytes: number | null;
  onError: (error: unknown) => void;
}) {
  const { dataRoot, homePath } = usePathNames();

  return (
    <header className="sticky top-0 z-10 flex h-10 items-center justify-between gap-3 border-b border-hairline bg-surface px-3">
      <p aria-hidden="true" className="flex items-baseline gap-1.5 text-[12px] text-ink-2">
        <Count value={profiles} className="font-mono text-ink" />
        <span>{profiles === 1 ? "profile" : "profiles"}</span>
        <Separator />
        <Count value={running} className="font-mono text-ink" />
        <span>running</span>
        {/* Absent until every row has reported: a total that counts half the
            profiles is a wrong number stated confidently. */}
        {bytes !== null ? (
          <>
            <Separator />
            <ByteCount bytes={bytes} className="font-mono text-ink" />
            <span>on disk</span>
          </>
        ) : null}
      </p>
      <span className="sr-only" aria-live="polite">
        {statusLine(profiles, running, bytes)}
      </span>

      {/* The strip names the folder; this is the only way to actually get to it. */}
      {dataRoot ? (
        <button
          type="button"
          title={dataRoot}
          onClick={() => {
            // Unlike reading the root, this one the user asked for.
            void revealItemInDir(dataRoot).catch(onError);
          }}
          className="flex min-w-0 shrink items-center gap-1.5 rounded-md px-1.5 py-1 text-ink-2 transition-colors duration-150 ease-out hover:bg-sunken hover:text-ink"
        >
          <FolderOpen size={13} strokeWidth={1.75} aria-hidden="true" className="shrink-0" />
          <span className="sr-only">Show the profiles folder in the file manager: </span>
          <bdi className="truncate font-mono text-[10.5px]">{shortenRoot(dataRoot, homePath)}</bdi>
        </button>
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
