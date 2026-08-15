import { useState } from "react";
import { ExternalLink, Pencil, Trash2 } from "lucide-react";

import { ByteCount, PendingSize } from "@/components/Counters";
import { IdentityChip } from "@/components/IdentityChip";
import { AnimatedBadge } from "@/components/motion/animated-badge";
import { Button } from "@/components/motion/button/base";
import { PathText } from "@/components/PathText";
import { DeletePanel, RenamePanel } from "@/components/RowPanel";
import * as api from "@/lib/api";
import type { ProfileView } from "@/lib/api";
import { edge, readable, wash } from "@/lib/color";
import { cn } from "@/lib/utils";

type Panel = { kind: "none" } | { kind: "rename" } | { kind: "delete"; bytes: number };

// beUI's badge is a 24px pill at 11px; a row this dense wants the smaller,
// upper-case chip the window already reads in. The status colours it ships are
// Tailwind's emerald and amber — this palette has its own, and they are set as
// styles so the same token drives the tag, the dot and the meter.
const TAG =
  "h-auto rounded-full px-1.5 py-px text-caption tracking-[0.04em] uppercase";

/// A fact about state, never about identity.
///
/// State is kept apart from the profile's own hue so a colour never has to mean
/// two things at once: running is always the live green, a shared sign-in is
/// always the warning amber, and neither one ever tints the identity chip.
/// Colour is not the message either — the word is right there beside it, which
/// is also the only thing beUI's icon would repeat, so it is turned off.
function Tag({
  token,
  status,
  children,
}: {
  token: string;
  status: "success" | "warning";
  children: string;
}) {
  return (
    <AnimatedBadge
      status={status}
      size="sm"
      showIcon={false}
      className={TAG}
      style={{ color: readable(token), background: wash(token), borderColor: edge(token) }}
    >
      {children}
    </AnimatedBadge>
  );
}

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
  sizeFailed,
  reload,
  onError,
  clearError,
}: {
  profile: ProfileView;
  bytes: number | undefined;
  /// This row's walk threw. Together with `bytes`, the three states the size
  /// slot can be in: a number, a walk still running, a walk that came back
  /// empty-handed.
  sizeFailed: boolean;
  reload: () => Promise<void>;
  onError: (error: unknown) => void;
  clearError: () => void;
}) {
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
      setPanel({ kind: "delete", bytes: size });
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
              <Tag token="var(--live)" status="success">
                Running
              </Tag>
            ) : null}
            {profile.shares_account ? (
              <Tag token="var(--warning)" status="warning">
                Shared sign-in
              </Tag>
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
          <span
            aria-hidden={bytes === undefined}
            className="col-start-1 row-start-1 self-center font-mono text-sub text-ink-2 transition-opacity duration-150 ease-out group-hover:opacity-0 group-focus-within:opacity-0"
          >
            {bytes !== undefined ? (
              <ByteCount bytes={bytes} />
            ) : sizeFailed ? (
              "—"
            ) : (
              <PendingSize />
            )}
          </span>
          <div className="col-start-1 row-start-1 flex items-center gap-0.5 self-center opacity-0 transition-opacity duration-150 ease-out group-hover:opacity-100 group-focus-within:opacity-100">
            <Button
              variant="ghost"
              size="icon"
              className={ICON_ACTION}
              title={`Open ${profile.label}`}
              aria-label={`Open ${profile.label}`}
              onClick={() => void act(api.openProfile(profile.app_id, profile.id))}
            >
              <ExternalLink size={15} strokeWidth={1.75} aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className={ICON_ACTION}
              title={`Rename ${profile.label}`}
              aria-label={`Rename ${profile.label}`}
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
                aria-label={`${profile.label} is the app's own installation and cannot be deleted`}
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
                title={`Delete ${profile.label}`}
                aria-label={`Delete ${profile.label}`}
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
