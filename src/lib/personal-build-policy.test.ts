import { beforeEach, describe, expect, it, vi } from "vitest";
import { isRedirect } from "@sveltejs/kit";
import { isUnavailablePlatform, isUnavailableRoute } from "./personal-build-policy";
import { buildIndex, CATEGORIES, search, toolById, toolHref, TOOLS } from "./tools/catalog";

const { loadSettings, loadTranslations } = vi.hoisted(() => ({
  loadSettings: vi.fn(),
  loadTranslations: vi.fn(),
}));
vi.mock("$lib/stores/settings-store.svelte", () => ({ loadSettings }));
vi.mock("$lib/i18n", () => ({ defaultLocale: "en", loadTranslations }));

import { load } from "../routes/+layout";
import type { LayoutLoadEvent } from "../routes/$types";

function loadEventFor(path: string): LayoutLoadEvent {
  // This load function reads only the URL; keep its framework fixture local to the test.
  return { url: new URL(path, "http://localhost") } as LayoutLoadEvent;
}

const removedRoutes = [
  "/omnidisc", "/omnidisc/stream", "/league", "/telegram", "/courses/hotmart",
  "/convert", "/study", "/study/library", "/study/music", "/marketplace",
  "/tools/music/music-lyrics", "/tools/video/video-convert",
];

describe("personal-build feature policy", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    loadSettings.mockResolvedValue({ appearance: { language: "en" } });
    loadTranslations.mockResolvedValue(undefined);
  });

  it.each(removedRoutes)("redirects %s before any settings or page startup", async (path) => {
    expect(isUnavailableRoute(path)).toBe(true);
    try {
      await load(loadEventFor(path));
      throw new Error("Expected the removed route to redirect");
    } catch (error) {
      expect(isRedirect(error)).toBe(true);
      expect(error).toMatchObject({ status: 307, location: "/" });
    }
    expect(loadSettings).not.toHaveBeenCalled();
    expect(loadTranslations).not.toHaveBeenCalled();
  });

  it("blocks encoded removed routes while allowing unrelated prefixes", () => {
    expect(isUnavailableRoute("/%73tudy/music")).toBe(true);
    expect(isUnavailableRoute("/study-guide")).toBe(false);
    expect(isUnavailableRoute("/courses-backup")).toBe(false);
  });

  it.each(["/", "/downloads", "/settings", "/about/changelog", "/tools/instagram/ig-download", "/tools/video/video-compress"])(
    "keeps %s available", async (path) => {
      expect(isUnavailableRoute(path)).toBe(false);
      await expect(load(loadEventFor(path))).resolves.toEqual({});
      expect(loadSettings).toHaveBeenCalledOnce();
      expect(loadTranslations).toHaveBeenCalledWith("en", path);
    },
  );

  it("has no removed Music/Convert tools to surface in search or command results", () => {
    const index = buildIndex((key) => key);
    expect(CATEGORIES.some((category) => category.id === "music")).toBe(false);
    expect(search(index, "music-lyrics", "all")).toEqual([]);
    expect(toolById("video-convert")).toBeUndefined();
    expect(TOOLS.every((tool) => !isUnavailableRoute(toolHref(tool)))).toBe(true);
    expect(toolById("video-compress")?.runner).toBe("video-compress");
    expect(toolById("ig-download")).toBeDefined();
  });

  it("keeps Instagram available while refusing removed platform actions", () => {
    for (const platform of ["hotmart", "udemy", "telegram", "p2p"]) {
      expect(isUnavailablePlatform(platform)).toBe(true);
    }
    expect(isUnavailablePlatform("instagram")).toBe(false);
    expect(isUnavailablePlatform("youtube")).toBe(false);
  });
});
