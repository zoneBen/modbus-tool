import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { useScansStore } from "@/stores/scans";

const link = {
  transport: "tcp" as const,
  framing: "mbap" as const,
  host: "127.0.0.1",
  port: 502,
  timeoutMs: 200,
};

describe("scans store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("keeps concurrently started scan tasks independent", async () => {
    const scans = useScansStore();
    const firstId = await scans.startSlaveIds(
      {
        link,
        functionCode: 3,
        address: 0,
        quantity: 1,
        idStart: 1,
        idEnd: 4,
      },
      "IDs",
      "127.0.0.1:502 · 1–4",
    );
    const secondId = await scans.startNetwork(
      {
        cidr: "192.168.1.0/30",
        port: 502,
        timeoutMs: 200,
        concurrency: 8,
        probeModbus: false,
        slaveId: 1,
      },
      "LAN",
      "192.168.1.0/30:502",
    );

    expect(scans.tasks).toHaveLength(2);
    expect(scans.runningCount).toBe(2);
    expect(scans.activeTaskId).toBe(secondId);

    const first = scans.tasks.find((task) => task.id === firstId);
    const second = scans.tasks.find((task) => task.id === secondId);
    expect(first?.backendId).toBeTruthy();
    expect(second?.backendId).toBeTruthy();

    scans.applyProgress({
      scanId: first?.backendId ?? "",
      kind: "slaveId",
      done: 2,
      total: 4,
      found: 1,
      phase: "probe",
      finished: false,
    });
    scans.applyResult({
      scanId: first?.backendId ?? "",
      kind: "slaveId",
      label: "ID 1",
      status: "ok",
      detail: "found",
      latencyMs: 4,
    });
    scans.applyDone({
      scanId: first?.backendId ?? "",
      kind: "slaveId",
      found: 1,
    });

    expect(scans.tasks.find((task) => task.id === firstId)?.status).toBe("done");
    expect(scans.tasks.find((task) => task.id === firstId)?.results).toHaveLength(1);
    expect(scans.tasks.find((task) => task.id === secondId)?.status).toBe("running");
    expect(scans.tasks.find((task) => task.id === secondId)?.results).toHaveLength(0);

    await scans.removeTask(secondId);
    expect(scans.tasks).toHaveLength(1);
  });
});
