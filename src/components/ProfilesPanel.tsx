import { ComposeCard } from "@/components/ComposeCard";
import { EmptyState } from "@/components/EmptyState";
import { ErrorBanner } from "@/components/ErrorBanner";
import { ProfileList } from "@/components/ProfileList";
import type { Sizes } from "@/hooks/useSizes";
import type { AppView, SocketBudget } from "@/lib/api";

/// The window as it was before there were two of them. Extracted so `App` is a
/// switch between panels rather than a switch wrapped around thirty lines of one
/// of them.
export function ProfilesPanel({
  apps,
  available,
  error,
  sizes,
  appId,
  onAppId,
  budget,
  reload,
  visit,
  fail,
  clearError,
}: {
  apps: AppView[];
  available: AppView[];
  error: string | null;
  sizes: Sizes;
  appId: string;
  onAppId: (id: string) => void;
  budget: SocketBudget | null;
  reload: () => Promise<void>;
  visit: number;
  fail: (error: unknown) => void;
  clearError: () => void;
}) {
  return (
    // `min-h-0` is what lets this shrink at all: a flex item's default
    // `min-height: auto` refuses to go below its content, which would push the
    // bar below the bottom edge of a window this size.
    // No `aria-labelledby`: beUI's `TabsTrigger` renders its own button and
    // takes no `id`, so pointing at one would dangle. The tablist, the tabs and
    // `aria-selected` all still come from the component; only the panel-to-tab
    // back-reference is missing.
    <section id="panel-profiles" role="tabpanel" className="flex min-h-0 flex-1 flex-col gap-2 p-2">
      <ErrorBanner message={error} />

      {available.length === 0 ? (
        <EmptyState apps={apps} />
      ) : (
        // Every app the tool knows about, not only the usable ones: an app that
        // is not installed is drawn greyed with its reason rather than dropped,
        // so a missing app reads as missing rather than as forgotten.
        <ProfileList
          apps={apps}
          sizes={sizes}
          reload={reload}
          onError={fail}
          clearError={clearError}
        />
      )}

      {/* With nothing to add a profile to, the whole band goes: a label over an
          empty space reads as something failing to load, and the form beneath it
          is a control that could only fail. */}
      {available.length > 0 ? (
        <ComposeCard
          apps={available}
          appId={appId}
          onAppId={onAppId}
          budget={budget}
          reload={reload}
          visit={visit}
        />
      ) : null}
    </section>
  );
}
