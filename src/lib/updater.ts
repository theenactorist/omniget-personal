export interface UpdateInfo {
  available: boolean;
  version?: string;
  body?: string;
}

export async function checkForUpdate(): Promise<UpdateInfo> {
  // Upstream releases would overwrite the changes in this personal build.
  return { available: false };
}

export async function installUpdate(): Promise<void> {
  throw new Error("Update OmniGet Personal by rebuilding your reviewed local source.");
}
