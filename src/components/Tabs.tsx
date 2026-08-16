import { cn } from "@/lib/utils";

export type TabId = "profiles" | "keep-awake";

const TABS: { id: TabId; label: string }[] = [
  { id: "profiles", label: "Agent Profiles" },
  { id: "keep-awake", label: "Keep Awake" },
];

/// Two tabs, drawn as a row of underlined labels rather than as a segmented
/// control.
///
/// A segmented control reads as a filter over one body of content; these are two
/// unrelated panels, and the underline is the convention that says so. It sits
/// directly under the status strip and shares its `px-5`, so the window keeps
/// one left edge from the top of the chrome to the bottom of it.
export function Tabs({ value, onChange }: { value: TabId; onChange: (next: TabId) => void }) {
  return (
    <div
      role="tablist"
      aria-label="Settings sections"
      className="flex h-9 shrink-0 items-stretch gap-4 border-b border-hairline bg-surface px-5"
    >
      {TABS.map((tab) => {
        const active = tab.id === value;
        return (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={active}
            aria-controls={`panel-${tab.id}`}
            id={`tab-${tab.id}`}
            onClick={() => onChange(tab.id)}
            className={cn(
              // `-mb-px` pulls the underline onto the container's own border, so
              // the active tab reads as continuous with the panel below it
              // rather than as a second line above it.
              "relative -mb-px border-b-2 text-callout transition-colors duration-150 ease-out",
              active ? "border-accent text-ink" : "border-transparent text-ink-2 hover:text-ink",
            )}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
