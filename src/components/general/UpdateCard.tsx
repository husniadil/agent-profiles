import { Button } from "@/components/motion/button/base";
import { Switch } from "@/components/motion/switch";
import type { General } from "@/hooks/useGeneral";
import type { UpdateState, Updater } from "@/hooks/useUpdater";
import { SWITCH } from "@/lib/controls";
import { useT, type T } from "@/lib/i18n";

/// One line of prose for whatever the updater is doing. Every state says
/// something a person can act on, or at least understand — an updater that goes
/// quiet is indistinguishable from one that is broken.
function line(t: T, state: UpdateState): string {
  switch (state.kind) {
    case "disabled":
      return t("general.update.disabled");
    case "idle":
      return t("general.update.idle");
    case "checking":
      return t("general.update.checking");
    case "current":
      return t("general.update.current");
    case "downloading":
      return t("general.update.downloading", { percent: state.percent });
    case "installing":
      return t("general.update.installing");
    case "failed":
      return t("general.update.failed", { reason: state.reason });
  }
}

export function UpdateCard({
  general,
  updater,
}: {
  general: General;
  updater: Updater;
}) {
  const t = useT();
  const autoUpdate = general.settings?.autoUpdate ?? true;
  const working =
    updater.state.kind === "checking" ||
    updater.state.kind === "downloading" ||
    updater.state.kind === "installing";

  return (
    <>
      <div>
        <p className="text-callout text-ink">{t("general.update.label")}</p>
        <p className="text-sub text-ink-2">{t("general.update.detail")}</p>
      </div>
      <Switch
        className={`shrink-0 ${SWITCH}`}
        checked={autoUpdate}
        onCheckedChange={(next) => void general.save({ autoUpdate: next })}
        ariaLabel={t("general.update.aria")}
      />
      {/* Second band: what the updater is doing, and the manual escape hatch.
          The version sits here rather than in the status strip — it is a fact
          about updating, and this is the only place anyone looks for it. */}
      <div className="col-span-2 flex items-center justify-between gap-4 border-t border-hairline pt-2.5">
        <div className="min-w-0">
          <p className="text-callout text-ink">
            {t("general.update.version", { version: updater.version })}
          </p>
          <p className="truncate text-sub text-ink-2">{line(t, updater.state)}</p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          className="h-7 shrink-0 rounded-lg px-2.5 text-callout"
          disabled={working}
          onClick={() => void updater.checkNow()}
        >
          {t("general.update.checkNow")}
        </Button>
      </div>
    </>
  );
}
