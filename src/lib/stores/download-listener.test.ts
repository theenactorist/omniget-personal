import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type EventHandler = (event: { payload: unknown }) => unknown;
type CompletionOptions = {
  duration?: number;
  detail?: string;
  action?: { label: string; onClick: () => void | Promise<void> };
};

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, EventHandler>(),
  invoke: vi.fn(),
  showToast: vi.fn(),
  notificationPermission: vi.fn(async () => false),
  recordDownloadComplete: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, handler: EventHandler) => {
    mocks.handlers.set(name, handler);
    return () => mocks.handlers.delete(name);
  },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: mocks.notificationPermission,
  requestPermission: async () => "denied",
  sendNotification: vi.fn(),
}));
vi.mock("$lib/i18n", () => ({
  t: {
    subscribe: (run: (translate: (key: string, args?: Record<string, string>) => string) => void) => {
      run((key, args) => {
        if (key === "downloads.saved_to_path") return `Saved to ${args?.path}`;
        if (key === "downloads.open_folder") return "Show in folder";
        if (key === "downloads.saved_location_unavailable") return "Saved location unavailable";
        return key;
      });
      return () => {};
    },
  },
}));
vi.mock("./download-store.svelte", () => ({
  upsertProgress: vi.fn(),
  markComplete: vi.fn(),
  syncQueueState: vi.fn(),
  upsertGenericProgress: vi.fn(),
  getDownloads: () => new Map(),
}));
vi.mock("./toast-store.svelte", () => ({ showToast: mocks.showToast }));
vi.mock("./convert-store.svelte", () => ({
  updateFileProgress: vi.fn(), markFileComplete: vi.fn(), markFileError: vi.fn(),
}));
vi.mock("./media-preview-store.svelte", () => ({ setMediaPreview: vi.fn() }));
vi.mock("./debug-store.svelte", () => ({ addLog: vi.fn() }));
vi.mock("./download-stats.svelte", () => ({ recordDownloadComplete: mocks.recordDownloadComplete }));
vi.mock("$lib/rpc", () => ({ rpcSyncIdleStats: vi.fn() }));

function queueItem(type: string, path: string | null = null) {
  return {
    id: 42,
    url: "https://www.instagram.com/reel/example/",
    platform: "instagram",
    title: "Media title differs from saved filename",
    status: { type },
    percent: type === "Complete" ? 100 : 50,
    speed_bytes_per_sec: 0,
    downloaded_bytes: 1000,
    total_bytes: 1000,
    file_path: path,
    file_size_bytes: 1000,
    file_count: 1,
    thumbnail_url: null,
  };
}

async function emitQueue(items: ReturnType<typeof queueItem>[]) {
  await mocks.handlers.get("queue-state-update")!({ payload: items });
}

function completionOptions(): CompletionOptions {
  const success = mocks.showToast.mock.calls.find(([type]) => type === "success");
  expect(success).toBeDefined();
  return success![2] as CompletionOptions;
}

describe("download completion location feedback", () => {
  let cleanup: (() => void) | undefined;

  beforeEach(async () => {
    vi.useFakeTimers();
    vi.resetModules();
    vi.clearAllMocks();
    mocks.handlers.clear();
    mocks.invoke.mockResolvedValue(undefined);
    const { initDownloadListener } = await import("./download-listener");
    cleanup = await initDownloadListener();
  });

  afterEach(() => {
    cleanup?.();
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it("does not replay toasts for completed downloads loaded at startup", async () => {
    const item = queueItem("Complete", "/Users/Me/Downloads/already saved.mp4");
    await emitQueue([item]);
    await emitQueue([item]);
    expect(mocks.showToast).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("announces an eligible completion once, with its actual saved path", async () => {
    const path = "/Users/Me/Downloads/  actual final name.mp4  ";
    await emitQueue([queueItem("Downloading")]);
    await emitQueue([queueItem("Complete", path)]);
    await emitQueue([queueItem("Complete", path)]);

    expect(mocks.showToast).toHaveBeenCalledOnce();
    expect(completionOptions()).toMatchObject({ duration: 10000, detail: `Saved to ${path}` });
    expect(completionOptions().action?.label).toBe("Show in folder");
    expect(mocks.invoke).not.toHaveBeenCalled();

    await completionOptions().action!.onClick();
    expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("reveal_file", { path });
  });

  it.each([null, "", "   "])("does not invent a path or action for %s", async (path) => {
    await emitQueue([queueItem("Downloading")]);
    await emitQueue([queueItem("Complete", path)]);
    expect(completionOptions().detail).toBe("Saved location unavailable");
    expect(completionOptions().action).toBeUndefined();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("never announces success for a terminal result explicitly marked unsuccessful", async () => {
    await emitQueue([queueItem("Downloading")]);
    const failed = {
      ...queueItem("Complete", "/Users/Me/Downloads/incomplete.mp4"),
      status: { type: "Complete", data: { success: false } },
    };
    await emitQueue([failed]);
    await emitQueue([failed]);
    expect(mocks.showToast).not.toHaveBeenCalled();
    expect(mocks.notificationPermission).not.toHaveBeenCalled();
    expect(mocks.recordDownloadComplete).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("surfaces a reveal failure and never falls back to opening the media", async () => {
    const path = "/Users/Me/Downloads/moved.mp4";
    await emitQueue([queueItem("Downloading")]);
    await emitQueue([queueItem("Complete", path)]);
    mocks.invoke.mockRejectedValueOnce(new Error("The saved file was moved"));

    await completionOptions().action!.onClick();

    expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("reveal_file", { path });
    expect(mocks.showToast).toHaveBeenLastCalledWith("error", "The saved file was moved");
  });
});
