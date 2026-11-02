import { describe, it, expect } from "vitest";
import { CORE_NAV_ITEMS } from "./nav-config";
import { isUnavailableRoute } from "./personal-build-policy";

describe("CORE_NAV_ITEMS", () => {
  it("contains only the retained personal-build sections", () => {
    expect(CORE_NAV_ITEMS.map((item) => item.href)).toEqual([
      "/", "/downloads", "/tools", "/settings", "/about",
    ]);
    expect(CORE_NAV_ITEMS.every((item) => !isUnavailableRoute(item.href))).toBe(true);
    expect(CORE_NAV_ITEMS.some((item) => item.group === "plugins")).toBe(false);
  });

  it("Home is first in primary group", () => {
    const home = CORE_NAV_ITEMS.find((i) => i.href === "/");
    expect(home?.group).toBe("primary");
    expect(home?.labelKey).toBe("nav.home");
  });

  it("Downloads carries the downloads badge", () => {
    const downloads = CORE_NAV_ITEMS.find((i) => i.href === "/downloads");
    expect(downloads?.badge).toBe("downloads");
  });

  it("About is in the app group after settings", () => {
    const about = CORE_NAV_ITEMS.find((i) => i.href === "/about");
    expect(about?.group).toBe("app");
    expect(about?.order).toBe(50);
  });

  it("order is strictly increasing among primary items", () => {
    const primaryOrders = CORE_NAV_ITEMS.filter((i) => i.group === "primary")
      .map((i) => i.order ?? 0)
      .sort((a, b) => a - b);
    for (let i = 1; i < primaryOrders.length; i++) {
      expect(primaryOrders[i]).toBeGreaterThan(primaryOrders[i - 1]);
    }
  });
});
