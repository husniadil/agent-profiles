import { useState } from "react";
import { ExternalLink, Pencil, Trash2 } from "lucide-react";

import { ByteCount } from "@/components/Counters";
import { IdentityChip } from "@/components/IdentityChip";
import { PathText } from "@/components/PathText";
import { DeletePanel, RenamePanel } from "@/components/RowPanel";
import { IconButton } from "@/components/ui/IconButton";
import { Tag } from "@/components/ui/Tag";
import * as api from "@/lib/api";
import type { ProfileView } from "@/lib/api";

type Panel = { kind: "none" } | { kind: "rename" } | { kind: "delete"; bytes: number };

export function ProfileRow({
  profile,
  bytes,
  reload,
  onError,
  clearError,
}: {
  profile: ProfileView;
  bytes: number | undefined;
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
    <div className="group relative rounded-lg px-2 py-2 transition-colors duration-150 ease-out hover:bg-sunken">
      <div className="flex items-start gap-3">
        <IdentityChip
          appId={profile.app_id}
          profileId={profile.id}
          label={profile.label}
          running={profile.running}
        />

        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1.5">
            <span className="truncate text-[13.5px] font-medium text-ink">{profile.label}</span>
            {/* Colour is never the whole message: the badge on the chip and this
                word say the same thing twice on purpose. */}
            {profile.running ? <Tag token="var(--live)">Running</Tag> : null}
            {profile.shares_account ? <Tag token="var(--warning)">Shared sign-in</Tag> : null}
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
        <div className="grid w-[88px] shrink-0 place-items-end">
          <span
            aria-hidden={bytes === undefined}
            className="col-start-1 row-start-1 self-center font-mono text-[11px] text-ink-2 transition-opacity duration-150 ease-out group-hover:opacity-0 group-focus-within:opacity-0"
          >
            {bytes === undefined ? "—" : <ByteCount bytes={bytes} />}
          </span>
          <div className="col-start-1 row-start-1 flex items-center gap-0.5 self-center opacity-0 transition-opacity duration-150 ease-out group-hover:opacity-100 group-focus-within:opacity-100">
            <IconButton
              icon={ExternalLink}
              label={`Open ${profile.label}`}
              onClick={() => void act(api.openProfile(profile.app_id, profile.id))}
            />
            <IconButton
              icon={Pencil}
              label={`Rename ${profile.label}`}
              onClick={() => setPanel({ kind: "rename" })}
            />
            {/* The Default profile is the app's own existing installation, so
                its directory is never ours to delete. Its label is still just a
                label, so rename stays. */}
            {profile.is_default ? null : (
              <IconButton
                icon={Trash2}
                tone="danger"
                label={`Delete ${profile.label}`}
                onClick={() => void askToDelete()}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
