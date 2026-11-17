import { describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  listSerialPorts: vi.fn(),
}));

vi.mock("@/lib/ipc", () => ({
  ipc: { listSerialPorts: mocks.listSerialPorts },
}));

describe("useSerialPorts", () => {
  it("loads ports returned by the backend", async () => {
    mocks.listSerialPorts.mockResolvedValue([
      {
        path: "COM21",
        label: "USB-SERIAL CH340 (COM21)",
        manufacturer: "wch.cn",
      },
    ]);
    const { useSerialPorts } = await import("./useSerialPorts");
    const { ports, loading, error, refresh } = useSerialPorts();

    const pending = refresh();
    expect(loading.value).toBe(true);
    await pending;

    expect(loading.value).toBe(false);
    expect(error.value).toBe("");
    expect(ports.value.map((port) => port.path)).toEqual(["COM21"]);
  });

  it("surfaces enumeration errors instead of fake ports", async () => {
    mocks.listSerialPorts.mockRejectedValue("拒绝访问");
    const { useSerialPorts } = await import("./useSerialPorts");
    const { ports, loading, error, refresh } = useSerialPorts();

    await refresh();

    expect(loading.value).toBe(false);
    expect(ports.value).toEqual([]);
    expect(error.value).toBe("拒绝访问");
  });
});
