import { getVersion } from "@tauri-apps/api/app";
import { LOCAL_CHANGELOG } from "$lib/local-changelog";

const STORAGE_KEY = "omniget_last_seen_version";
const CHANGELOG_BODY_KEY = "omniget_pending_changelog";

let showDialog = $state(false);
let changelogBody = $state(LOCAL_CHANGELOG);
let currentVersion = $state("");

export function getChangelogVisible(): boolean {
  return showDialog;
}

export function getChangelogBody(): string {
  return changelogBody;
}

export function getCurrentVersion(): string {
  return currentVersion;
}

export function dismissChangelog(): void {
  showDialog = false;
  if (currentVersion) {
    localStorage.setItem(STORAGE_KEY, currentVersion);
  }
  localStorage.removeItem(CHANGELOG_BODY_KEY);
}

export function showChangelog(): void {
  showDialog = true;
}

export async function initChangelog(): Promise<void> {
  try {
    currentVersion = await getVersion();
  } catch {
    currentVersion = "0.9.6";
  }

  const lastSeen = localStorage.getItem(STORAGE_KEY);
  changelogBody = LOCAL_CHANGELOG;
  if (lastSeen && lastSeen !== currentVersion) showDialog = true;
  localStorage.setItem(STORAGE_KEY, currentVersion);
}

export async function fetchChangelog(): Promise<string> {
  return LOCAL_CHANGELOG;
}
