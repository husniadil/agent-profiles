import { useState } from "react";
import { ExternalLink, Pencil, Trash2 } from "lucide-react";

import { ByteCount, PendingSize } from "@/components/Counters";
import { formatBytes } from "@/format";
import { IdentityChip } from "@/components/IdentityChip";
import { Button } from "@/components/motion/button/base";
import { PathText } from "@/components/PathText";
import { StateTag } from "@/components/StateTag";
import { DeletePanel, RenamePanel } from "@/components/RowPanel";
import * as api from "@/lib/api";
import type { ProfileView } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { cn } from "@/lib/utils";

type Panel =
  | { kind: "none" }
  | { kind: "rename" }
  | { kind: "delete"; bytes: number; approximate: boolean };

// A control whose whole face is a picture, so its name has to reach a screen
// reader some other way. `title` covers the pointer; `aria-label` covers
// everything else, and both are spelled out on every one of them below.
const ICON_ACTION = "size-7 rounded-md";

// The trailing column is exactly as wide as the widest thing it holds: the
// action row, at 28n + 2(n−1) for n actions — three 28px buttons and two 2px
// gaps. Anything narrower is not a smaller column, it is an overflowing one,
// and the size text rides out with it. A fourth action makes this 118.
const TRAILING = "w-[88px]";

export function ProfileRow({
  profile,
  bytes,
  sizeApproximate,
  sizeFailed,
  reload,
  onError,
  clearError,
}: {
  profile: ProfileView;
  bytes: number | undefined;
  /// This row's walk reached a number but not every entry, so what it reports is
  /// short by an unknown amount and is marked rather than stated flat.
  sizeApproximate: boolean;
  /// This row's walk threw. Together with `bytes`, the three states the size
  /// slot can be in: a number, a walk still running, a walk that came back
  /// empty-handed.
  sizeFailed: boolean;
  reload: () => Promise<void>;
  onError: (error: unknown) => void;
  clearError: () => void;
}) {
  const t = useT();
  const [panel, setPanel] = useState<Panel>({ kind: "none" });

  async function act(work: Promise<unknown>): Promise<void> {
    try {
      await work;
      clearError();
      setPanel({ kind: "none" });
      await reload();
    } catch (cause) {
      onError(cause);
    }
  }

  async function askToDelete(): Promise<void> {
    // The confirmation states the size, so it is read now rather than taken from
    // the row: the row's figure is what was true when the list was drawn, and
    // this sentence is about a folder that is about to stop existing.
    try {
      const size = await api.profileSizeBytes(profile.app_id, profile.id);
      setPanel({ kind: "delete", bytes: size.bytes, approximate: size.skipped > 0 });
    } catch (cause) {
      onError(cause);
    }
  }

  return (
    <div className="group relative rounded-lg px-2 py-1.5 transition-colors duration-150 ease-out hover:bg-sunken">
      <div className="flex items-start gap-2.5">
        <IdentityChip
          appId={profile.app_id}
          profileId={profile.id}
          label={profile.label}
          running={profile.running}
        />

        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1.5">
            <span className="truncate text-body font-medium text-ink">{profile.label}</span>
            {/* Colour is never the whole message: the badge on the chip and this
                word say the same thing twice on purpose. */}
            {profile.running ? (
              <StateTag token="var(--live)" status="success">
                {t("row.running")}
              </StateTag>
            ) : null}
            {profile.shares_account ? (
              <StateTag token="var(--warning)" status="warning">
                {t("row.sharedSignIn")}
              </StateTag>
            ) : null}
          </div>
          <PathText path={profile.path} className="mt-0.5" />

          {panel.kind === "rename" ? (
            <RenamePanel
              label={profile.label}
              onCancel={() => setPanel({ kind: "none" })}
              onSave={(label) =>
                void act(api.renameProfile(profile.app_id, profile.id, label))
              }
            />
          ) : null}
          {panel.kind === "delete" ? (
            <DeletePanel
              label={profile.label}
              bytes={panel.bytes}
              approximate={panel.approximate}
              onCancel={() => setPanel({ kind: "none" })}
              onConfirm={() => void act(api.deleteProfile(profile.app_id, profile.id))}
            />
          ) : null}
        </div>

        {/* Size and actions share one slot: the size is what the row says at
            rest, the icons are what it offers when reached for. Stacked in one
            grid cell so the row does not change width when they trade places. */}
        {/* Fixed width, because both things it holds are read down a column: a
            size that shifts left and right by a character makes five rows look
            like five different lists. */}
        {/* One column, explicitly `minmax(0,1fr)`: an implicit `auto` track is
            floored at its content's min-content width, so the action row would
            widen the track past the box and push the size text out of line. */}
        <div className={cn("grid shrink-0 grid-cols-[minmax(0,1fr)] justify-items-end", TRAILING)}>
          {/* The `≥` mark reaches the eye but not a screen reader, which skips it
              as punctuation and hears a bare, exact-sounding figure. So when the
              walk fell short the visible number is hidden from the reader and the
              lower bound is spoken in words instead — the same direction the mark
              means. An exact figure is left to speak for itself. */}
          <span
            aria-hidden={bytes === undefined || sizeApproximate}
            className="col-start-1 row-start-1 self-center font-mono text-sub text-ink-2 transition-opacity duration-150 ease-out group-hover:opacity-0 group-focus-within:opacity-0"
          >
            {bytes !== undefined ? (
              <ByteCount bytes={bytes} approximate={sizeApproximate} />
            ) : sizeFailed ? (
              "—"
            ) : (
              <PendingSize />
            )}
          </span>
          {bytes !== undefined && sizeApproximate ? (
            <span className="sr-only col-start-1 row-start-1">
              {t("status.sizeAtLeast", { size: formatBytes(bytes) })}
            </span>
          ) : null}
          <div className="col-start-1 row-start-1 flex items-center gap-0.5 self-center opacity-0 transition-opacity duration-150 ease-out group-hover:opacity-100 group-focus-within:opacity-100">
            <Button
              variant="ghost"
              size="icon"
              className={ICON_ACTION}
              title={t("row.open", { name: profile.label })}
              aria-label={t("row.open", { name: profile.label })}
              onClick={() => void act(api.openProfile(profile.app_id, profile.id))}
            >
              <ExternalLink size={15} strokeWidth={1.75} aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className={ICON_ACTION}
              title={t("row.rename", { name: profile.label })}
              aria-label={t("row.rename", { name: profile.label })}
              onClick={() => setPanel({ kind: "rename" })}
            >
              <Pencil size={15} strokeWidth={1.75} aria-hidden="true" />
            </Button>
            {/* The Default profile is the app's own existing installation, so
                its directory is never ours to delete. Its label is still just a
                label, so rename stays.

                The slot is kept even where the action is not, because position
                is how this row is read: if the last 28px means Delete on one
                row and Open on the next, the hand that learned the first row
                mis-clicks the second.
                It holds a disabled trash rather than nothing. An empty slot
                reads as an icon that failed to load; a greyed one reads as an
                action this row does not have, which is the truth. The label
                says why, because a disabled control that does not is a dead end
                — and it says *cannot*, not *not yet*, so it promises no
                condition under which this profile could be deleted. */}
            {profile.is_default ? (
              <Button
                variant="ghost"
                size="icon"
                className={ICON_ACTION}
                disabled
                aria-label={t("row.deleteUnavailable", { name: profile.label })}
              >
                <Trash2 size={15} strokeWidth={1.75} aria-hidden="true" />
              </Button>
            ) : (
              <Button
                variant="ghost"
                size="icon"
                className={cn(
                  ICON_ACTION,
                  "hover:bg-[color-mix(in_oklab,var(--danger)_14%,var(--surface))]",
                  "hover:text-[color-mix(in_oklab,var(--danger)_70%,var(--ink))]",
                )}
                title={t("row.deleteTrigger", { name: profile.label })}
                aria-label={t("row.deleteTrigger", { name: profile.label })}
                onClick={() => void askToDelete()}
              >
                <Trash2 size={15} strokeWidth={1.75} aria-hidden="true" />
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
