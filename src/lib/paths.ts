import { createContext, useContext } from "react";

/// Paths, written the way a person says them.
///
/// The data root and the home directory are both learned once and never change
/// while the app runs, and they are only ever used to shorten what is drawn —
/// every element also keeps the full path — so arriving late, or not at all,
/// costs an abbreviation and nothing else.

export type PathNames = { dataRoot: string; homePath: string };

export const PathNamesContext = createContext<PathNames>({ dataRoot: "", homePath: "" });

export function usePathNames(): PathNames {
  return useContext(PathNamesContext);
}

/// A path split at its last separator, either kind, so the same code reads a
/// Windows path and a POSIX one.
export function splitTail(path: string): [string, string] {
  const cut = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return cut < 0 ? ["", path] : [path.slice(0, cut + 1), path.slice(cut + 1)];
}

/// A profile path as the window shows it: everything above our own data root is
/// scenery, and the home directory is a name the reader already knows.
export function shortenPath(path: string, { dataRoot, homePath }: PathNames): string {
  if (dataRoot && path.startsWith(`${dataRoot}/`)) {
    return `…/${splitTail(dataRoot)[1]}${path.slice(dataRoot.length)}`;
  }
  if (homePath && path.startsWith(`${homePath}/`)) {
    return `~${path.slice(homePath.length)}`;
  }
  return path;
}

/// The data root, shortened to the one segment that names it.
export function shortenRoot(root: string, homePath: string): string {
  const tail = splitTail(root)[1];
  if (!homePath || !root.startsWith(`${homePath}/`)) return `…/${tail}`;
  const inside = root.slice(homePath.length + 1);
  return inside === tail ? `~/${tail}` : `~/…/${tail}`;
}
