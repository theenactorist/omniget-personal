import { describe, expect, it } from "vitest";
import { downloadLocation } from "./download-location";

describe("recorded download locations", () => {
  it.each([undefined, null, "", "   "])("does not invent a location for %s", (path) => {
    expect(downloadLocation(path)).toBeNull();
  });

  it("uses the actual saved name, including spaces and Unicode", () => {
    const path = "/Users/Me/Downloads/Reels/A summer walk — final.mp4";
    expect(downloadLocation(path)).toEqual({
      path, name: "A summer walk — final.mp4", parent: "/Users/Me/Downloads/Reels",
    });
  });

  it("keeps a backslash inside a POSIX filename", () => {
    const path = "/Users/Me/Downloads/part\\two.mp4";
    expect(downloadLocation(path)).toEqual({ path, name: "part\\two.mp4", parent: "/Users/Me/Downloads" });
  });

  it("preserves the original Windows drive path for reveal", () => {
    const path = "C:\\Users\\Me\\Downloads\\Reel.mp4";
    expect(downloadLocation(path)).toEqual({ path, name: "Reel.mp4", parent: "C:\\Users\\Me\\Downloads" });
  });

  it("preserves a Windows drive root in the parent", () => {
    expect(downloadLocation("C:\\Reel.mp4")).toEqual({ path: "C:\\Reel.mp4", name: "Reel.mp4", parent: "C:\\" });
  });

  it("handles Windows network shares", () => {
    const path = "\\\\server\\media\\Reels\\Reel.mp4";
    expect(downloadLocation(path)).toEqual({ path, name: "Reel.mp4", parent: "\\\\server\\media\\Reels" });
  });

  it.each(["/", "\\", "C:\\", "\\\\server\\media\\"])("shows root %s without inventing a parent", (path) => {
    expect(downloadLocation(path)).toEqual({ path, name: path, parent: null });
  });

  it("keeps a recorded multi-file output directory neutral", () => {
    const path = "/Users/Me/Downloads/Torrent collection/";
    expect(downloadLocation(path)).toEqual({ path, name: "Torrent collection", parent: "/Users/Me/Downloads" });
  });

  it("handles a file directly below the POSIX root", () => {
    expect(downloadLocation("/video.mp4")).toEqual({ path: "/video.mp4", name: "video.mp4", parent: "/" });
  });

  it("does not add a default folder to a recorded relative path", () => {
    expect(downloadLocation("video.mp4")).toEqual({ path: "video.mp4", name: "video.mp4", parent: null });
  });

  it("preserves meaningful leading and trailing filename whitespace", () => {
    const path = "/Users/Me/Downloads/  final reel.mp4  ";
    expect(downloadLocation(path)).toEqual({ path, name: "  final reel.mp4  ", parent: "/Users/Me/Downloads" });
  });

  it("leaves long filenames and paths intact for selection and wrapping", () => {
    const name = `${"long title ".repeat(25)}.mp4`;
    const path = `/Users/Me/Downloads/${"long folder/".repeat(10)}${name}`;
    expect(downloadLocation(path)?.name).toBe(name);
    expect(downloadLocation(path)?.path).toBe(path);
    expect(downloadLocation(path)?.parent).toBe(path.slice(0, -(name.length + 1)));
  });
});
