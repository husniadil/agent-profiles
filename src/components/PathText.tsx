import { Tooltip } from "@/components/motion/tooltip";
import { shortenPath, splitTail, usePathNames } from "@/lib/paths";
import { cn } from "@/lib/utils";

/// A path drawn with its last segment set apart, because the last segment is
/// the part that tells two similar profiles apart.
///
/// The line is ellipsised like every other path in the window, so the full value
/// has to stay reachable — and reachable by more than a mouse. `title` was only
/// ever the former: it waits a second, it appears in the OS's own styling, and a
/// keyboard reaches it never.
///
/// So the full path is said twice, to two audiences that do not overlap:
///
///  * on screen, in a tooltip that opens on hover *and* on focus, which is why
///    the line is a tab stop at all;
///  * to assistive technology, as the element's own text — the visible, shortened
///    line is marked hidden and the unabridged one is read in its place. beUI's
///    tooltip is `aria-hidden` by design (it is a picture of what is already
///    said), so the accessible copy cannot be the tooltip.
export function PathText({ path, className }: { path: string; className?: string }) {
  const names = usePathNames();
  const [head, tail] = splitTail(shortenPath(path, names));
  return (
    <Tooltip
      content={path}
      side="bottom"
      wrapperClassName="block min-w-0 max-w-full"
      // A path is long and a window this narrow is not; it wraps rather than
      // running off the edge with its middle unreadable.
      className="max-w-[min(420px,calc(100vw-16px))] break-all whitespace-normal font-mono text-sub font-normal"
    >
      {/* The quietest line in the window. The path is where a profile lives,
          not what it is called: the name is what the eye should land on, and a
          directory set as loudly as the name competes with it. So: the same
          `text-sub` the New profile card sets its own path in — one step below
          the name beside it — but in `ink-2` rather than full ink.
          The face stays monospace, like every path and id here. A proportional
          one was tried and reverted: it made this the one path in the window
          that did not look like a path, a few rows from one that did.
          The tail used to take the full-strength ink and was the loudest thing
          on the row after the name; it carries its emphasis in weight now, so
          the whole line sits at one value. Not `ink-3`, which would be quieter
          still: against the light theme's surface it reads at 3.5:1. */}
      <p
        tabIndex={0}
        className={cn(
          "truncate rounded-sm font-mono text-sub text-ink-2",
          className,
        )}
      >
        <span className="sr-only">{path}</span>
        <span aria-hidden="true">
          {head}
          <span className="font-medium">{tail}</span>
        </span>
      </p>
    </Tooltip>
  );
}
