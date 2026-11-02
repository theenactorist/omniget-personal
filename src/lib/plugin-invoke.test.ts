import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { pluginInvoke } from "./plugin-invoke";
import { PLUGINS_UNAVAILABLE_MESSAGE } from "./personal-build-policy";

describe("pluginInvoke in the personal build", () => {
  beforeEach(() => invokeMock.mockReset());

  it.each(["courses", "telegram", "convert", "study", "unreviewed"])(
    "blocks %s before invoking native plugin code",
    async (pluginId) => {
      await expect(pluginInvoke(pluginId, "load", { enabled: true }))
        .rejects.toThrow(PLUGINS_UNAVAILABLE_MESSAGE);
      expect(invokeMock).not.toHaveBeenCalled();
    },
  );
});
