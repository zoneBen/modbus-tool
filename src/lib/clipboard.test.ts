import { afterEach, describe, expect, it, vi } from "vitest";
import { copyText } from "./clipboard";

describe("clipboard helper", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("uses the async clipboard when available", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    await expect(copyText("hello")).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith("hello");
  });

  it("falls back when the async clipboard fails", async () => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
    });
    const exec = vi.fn().mockReturnValue(true);
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: exec,
    });
    await expect(copyText("fallback")).resolves.toBe(true);
    expect(exec).toHaveBeenCalledWith("copy");
  });
});
