/** Features excluded from the reviewed personal build, regardless of saved settings. */
const UNAVAILABLE_ROUTES = [
  "/omnidisc",
  "/league",
  "/telegram",
  "/courses",
  "/convert",
  "/study",
  "/marketplace",
  "/library",
  "/music",
  "/tools/music",
  "/tools/video/video-convert",
] as const;

export const PLUGINS_UNAVAILABLE_MESSAGE = "Plugins are unavailable in this personal build.";

export function isUnavailableRoute(pathname: string): boolean {
  let decoded: string;
  try {
    decoded = decodeURIComponent(pathname).toLowerCase();
  } catch {
    return true;
  }
  return UNAVAILABLE_ROUTES.some((root) => decoded === root || decoded.startsWith(`${root}/`));
}

export function isUnavailablePlatform(platform: string): boolean {
  return ["telegram", "hotmart", "udemy", "p2p"].includes(platform.toLowerCase());
}
