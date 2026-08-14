import { shortenPath, splitTail, usePathNames } from "@/lib/paths";
import { cn } from "@/lib/utils";

/// A path drawn with its last segment set apart, because the last segment is
/// the part that tells two similar profiles apart.
///
/// The line is ellipsised like every other path in the window, so the full value
/// has to stay reachable: it is on the `title`, and it is the full value rather
/// than the shortened one.
export function PathText({ path, className }: { path: string; className?: string }) {
  const names = usePathNames();
  const [head, tail] = splitTail(shortenPath(path, names));
  return (
    <p className={cn("truncate font-mono text-[10.5px] text-ink-2", className)} title={path}>
      {head}
      <span className="text-ink">{tail}</span>
    </p>
  );
}
