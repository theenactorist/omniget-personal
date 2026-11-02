export type DownloadLocation = {
  /** Exact recorded output path, passed unchanged to the file manager. */
  path: string;
  name: string;
  parent: string | null;
};

/** Display a recorded output path without guessing whether it names a file or folder. */
export function downloadLocation(path: string | null | undefined): DownloadLocation | null {
  if (!path || !path.trim()) return null;

  const windows = /^[a-z]:[\\/]/i.test(path) || path.startsWith("\\\\") || (!path.includes("/") && path.includes("\\"));
  const trailingSeparators = windows ? /[\\/]+$/ : /\/+$/;
  const isRoot = windows
    ? /^[\\/]+$/.test(path) || /^[a-z]:[\\/]+$/i.test(path) || /^\\\\[^\\/]+[\\/][^\\/]+[\\/]*$/.test(path)
    : /^\/+$/i.test(path);
  if (isRoot) return { path, name: path, parent: null };

  const display = path.replace(trailingSeparators, "");
  const separator = windows ? Math.max(display.lastIndexOf("/"), display.lastIndexOf("\\")) : display.lastIndexOf("/");
  if (separator < 0) return { path, name: display, parent: null };

  let parent = separator === 0 ? display[0] : display.slice(0, separator);
  if (windows && /^[a-z]:$/i.test(parent)) parent += display[separator];
  return { path, name: display.slice(separator + 1), parent };
}
