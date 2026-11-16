<script setup lang="ts">
import { Radar, Square, Trash2 } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { formatLatency } from "@/lib/format";
import { useScansStore } from "@/stores/scans";
import type { Framing, FunctionCode, ScanKind, ScanLinkInput } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const scans = useScansStore();

const tabs: Array<{ kind: ScanKind; label: string }> = [
  { kind: "slaveId", label: "从站 ID" },
  { kind: "address", label: "寄存器地址" },
  { kind: "baud", label: "波特率" },
  { kind: "network", label: "局域网 IP" },
];

const activeKind = ref<ScanKind>("slaveId");
const linkModes = ["network", "serial"] as const;
const linkMode = ref<"network" | "serial">("network");
const transport = ref<"tcp" | "udp">("tcp");
const framing = ref<Framing>("mbap");
const host = ref("192.168.1.10");
const port = ref("502");
const serialPort = ref("COM3");
const baudRate = ref("9600");
const dataBits = ref("8");
const parity = ref<"none" | "even" | "odd">("none");
const stopBits = ref("1");
const timeoutMs = ref("200");
const serialPorts = ref<Array<{ path: string; label: string }>>([
  { path: "COM3", label: "USB-SERIAL CH340 (COM3)" },
  { path: "COM4", label: "FTDI FT232R (COM4)" },
]);

const idFunction = ref("3");
const idAddress = ref("0");
const idQuantity = ref("1");
const idStart = ref("1");
const idEnd = ref("247");

const addressFunction = ref("3");
const addressSlaveId = ref("1");
const addressStart = ref("0");
const addressEnd = ref("99");
const blockSize = ref("10");
const nonZeroOnly = ref(false);

const baudFraming = ref<"rtu" | "ascii">("rtu");
const baudRateList = ref("1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200");
const baudSlaveId = ref("1");
const baudFunction = ref("3");
const baudAddress = ref("0");
const baudQuantity = ref("1");
const baudTimeout = ref("300");

const cidr = ref("192.168.1.0/24");
const netPort = ref("502");
const netTimeout = ref("400");
const netConcurrency = ref("64");
const probeModbus = ref(true);
const netSlaveId = ref("1");

const functionOptions = [
  { label: "FC01 线圈", value: "1" },
  { label: "FC02 离散输入", value: "2" },
  { label: "FC03 保持寄存器", value: "3" },
  { label: "FC04 输入寄存器", value: "4" },
];

const framingOptions = computed(() =>
  linkMode.value === "serial"
    ? [
        { label: "RTU", value: "rtu" as Framing },
        { label: "ASCII", value: "ascii" as Framing },
      ]
    : [
        { label: "MBAP", value: "mbap" as Framing },
        { label: "RTU over TCP/UDP", value: "rtu" as Framing },
        { label: "ASCII over TCP/UDP", value: "ascii" as Framing },
      ],
);

const statusTone: Record<string, string> = {
  ok: "border-signal/25 bg-signal/10 text-signal",
  exception: "border-warn/25 bg-warn/10 text-warn",
  timeout: "border-white/[0.08] bg-white/[0.03] text-muted-foreground/55",
  error: "border-error/25 bg-error/10 text-error",
  open: "border-rx/25 bg-rx/10 text-rx",
};

const running = computed(() => scans.running);

watch(open, async (isOpen) => {
  if (!isOpen) return;
  try {
    const ports = await ipc.listSerialPorts();
    if (ports.length) {
      serialPorts.value = ports.map((item) => ({ path: item.path, label: item.label }));
      if (!serialPorts.value.some((item) => item.path === serialPort.value)) {
        serialPort.value = serialPorts.value[0]?.path ?? "COM3";
      }
    }
  } catch {
    // Keep the demo fallback list.
  }
});

watch(linkMode, (mode) => {
  if (mode === "serial" && framing.value === "mbap") framing.value = "rtu";
});

function buildLink(): ScanLinkInput {
  const serial = linkMode.value === "serial";
  return {
    transport: serial ? "serial" : transport.value,
    framing: serial ? (framing.value === "mbap" ? "rtu" : framing.value) : framing.value,
    host: serial ? undefined : host.value.trim(),
    port: serial ? undefined : Math.max(1, Number(port.value) || 502),
    serialPort: serial ? serialPort.value : undefined,
    baudRate: serial ? Math.max(1, Number(baudRate.value) || 9600) : undefined,
    dataBits: Number(dataBits.value) || 8,
    parity: parity.value,
    stopBits: Number(stopBits.value) || 1,
    timeoutMs: Math.max(50, Number(timeoutMs.value) || 200),
  };
}

async function startSlaveIdScan(): Promise<void> {
  try {
    await scans.startSlaveIds({
      link: buildLink(),
      functionCode: Number(idFunction.value) as FunctionCode,
      address: Math.max(0, Number(idAddress.value) || 0),
      quantity: Math.max(1, Number(idQuantity.value) || 1),
      idStart: Math.min(255, Math.max(0, Number(idStart.value) || 0)),
      idEnd: Math.min(255, Math.max(0, Number(idEnd.value) || 0)),
    });
  } catch (error: unknown) {
    toast.error("启动扫描失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}

async function startAddressScan(): Promise<void> {
  try {
    await scans.startAddresses({
      link: buildLink(),
      functionCode: Number(addressFunction.value) as FunctionCode,
      slaveId: Math.max(1, Number(addressSlaveId.value) || 1),
      startAddress: Math.max(0, Number(addressStart.value) || 0),
      endAddress: Math.max(0, Number(addressEnd.value) || 0),
      blockSize: Math.max(1, Number(blockSize.value) || 10),
      nonZeroOnly: nonZeroOnly.value,
    });
  } catch (error: unknown) {
    toast.error("启动扫描失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}

async function startBaudScan(): Promise<void> {
  const rates = baudRateList.value
    .split(/[\s,;]+/)
    .map((entry) => Number(entry))
    .filter((entry) => Number.isFinite(entry) && entry > 0);
  if (!rates.length) {
    toast.error("请填写至少一个波特率");
    return;
  }
  try {
    await scans.startBaudRates({
      serialPort: serialPort.value,
      framing: baudFraming.value,
      dataBits: Number(dataBits.value) || 8,
      parity: parity.value,
      stopBits: Number(stopBits.value) || 1,
      baudRates: rates,
      slaveId: Math.max(1, Number(baudSlaveId.value) || 1),
      functionCode: Number(baudFunction.value) as FunctionCode,
      address: Math.max(0, Number(baudAddress.value) || 0),
      quantity: Math.max(1, Number(baudQuantity.value) || 1),
      timeoutMs: Math.max(50, Number(baudTimeout.value) || 300),
    });
  } catch (error: unknown) {
    toast.error("启动扫描失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}

async function startNetworkScan(): Promise<void> {
  if (!cidr.value.trim()) {
    toast.error("请输入 CIDR 网段");
    return;
  }
  try {
    await scans.startNetwork({
      cidr: cidr.value.trim(),
      port: Math.max(1, Number(netPort.value) || 502),
      timeoutMs: Math.max(100, Number(netTimeout.value) || 400),
      concurrency: Math.min(128, Math.max(1, Number(netConcurrency.value) || 64)),
      probeModbus: probeModbus.value,
      slaveId: Math.max(1, Number(netSlaveId.value) || 1),
    });
  } catch (error: unknown) {
    toast.error("启动扫描失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}

function startCurrent(): void {
  if (running.value) return;
  switch (activeKind.value) {
    case "slaveId":
      void startSlaveIdScan();
      break;
    case "address":
      void startAddressScan();
      break;
    case "baud":
      void startBaudScan();
      break;
    case "network":
      void startNetworkScan();
      break;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-white/[0.08] bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/60 backdrop-blur-2xl sm:max-w-[720px]"
    >
      <DialogHeader class="border-b border-white/[0.06] px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <Radar class="size-3.5" />
          </span>
          设备扫描
        </DialogTitle>
        <DialogDescription class="text-[10.5px] text-muted-foreground/65">
          扫描报文不写入当前轮询会话；串口扫描要求端口未被占用。
        </DialogDescription>
      </DialogHeader>

      <div class="grid grid-cols-4 gap-1 border-b border-white/[0.05] bg-black/10 px-3 py-2">
        <button
          v-for="tab in tabs"
          :key="tab.kind"
          type="button"
          class="rounded-md px-2 py-1.5 text-[10.5px] transition-colors"
          :class="
            activeKind === tab.kind
              ? 'bg-white/[0.07] text-foreground'
              : 'text-muted-foreground/55 hover:text-foreground'
          "
          @click="activeKind = tab.kind"
        >
          {{ tab.label }}
        </button>
      </div>

      <div class="scrollbar-thin max-h-[62vh] space-y-4 overflow-y-auto px-5 py-4">
        <template v-if="activeKind === 'slaveId' || activeKind === 'address'">
          <div class="space-y-3 rounded-xl border border-white/[0.05] bg-black/10 p-3">
            <div class="flex items-center justify-between">
              <p class="text-[11px] font-medium text-foreground/85">扫描链路</p>
              <div class="flex rounded-lg border border-white/[0.07] bg-black/20 p-0.5">
                <button
                  v-for="mode in linkModes"
                  :key="mode"
                  type="button"
                  class="rounded-md px-2 py-0.5 text-[9.5px] transition-colors"
                  :class="
                    linkMode === mode
                      ? 'bg-white/[0.08] text-foreground'
                      : 'text-muted-foreground/55 hover:text-foreground'
                  "
                  @click="linkMode = mode"
                >
                  {{ mode === "network" ? "网络" : "串口" }}
                </button>
              </div>
            </div>

            <div v-if="linkMode === 'network'" class="grid grid-cols-4 gap-2">
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">传输层</Label>
                <FormSelect
                  v-model="transport"
                  :options="[
                    { label: 'TCP', value: 'tcp' },
                    { label: 'UDP', value: 'udp' },
                  ]"
                />
              </div>
              <div class="col-span-2 space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">主机</Label>
                <Input
                  v-model="host"
                  class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
                />
              </div>
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">端口</Label>
                <Input
                  v-model="port"
                  inputmode="numeric"
                  class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
                />
              </div>
            </div>
            <div v-else class="grid grid-cols-4 gap-2">
              <div class="col-span-2 space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">串口</Label>
                <FormSelect
                  v-model="serialPort"
                  :options="serialPorts.map((item) => ({ label: item.label, value: item.path }))"
                />
              </div>
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">波特率</Label>
                <FormSelect
                  v-model="baudRate"
                  :options="
                    ['1200', '2400', '4800', '9600', '19200', '38400', '57600', '115200'].map(
                      (value) => ({ label: value, value }),
                    )
                  "
                />
              </div>
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">校验</Label>
                <FormSelect
                  v-model="parity"
                  :options="[
                    { label: 'N', value: 'none' },
                    { label: 'E', value: 'even' },
                    { label: 'O', value: 'odd' },
                  ]"
                />
              </div>
            </div>

            <div class="grid grid-cols-3 gap-2">
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">帧格式</Label>
                <FormSelect v-model="framing" :options="framingOptions" />
              </div>
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">超时 (ms)</Label>
                <Input
                  v-model="timeoutMs"
                  inputmode="numeric"
                  class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
                />
              </div>
              <div v-if="linkMode === 'serial'" class="grid grid-cols-2 gap-2">
                <div class="space-y-1">
                  <Label class="text-[9px] text-muted-foreground/60">数据位</Label>
                  <FormSelect
                    v-model="dataBits"
                    :options="['7', '8'].map((value) => ({ label: value, value }))"
                  />
                </div>
                <div class="space-y-1">
                  <Label class="text-[9px] text-muted-foreground/60">停止位</Label>
                  <FormSelect
                    v-model="stopBits"
                    :options="['1', '2'].map((value) => ({ label: value, value }))"
                  />
                </div>
              </div>
            </div>
          </div>
        </template>

        <div
          v-if="activeKind === 'slaveId'"
          class="grid grid-cols-[1.25fr_1fr_0.75fr_0.9fr_0.9fr] gap-2"
        >
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">功能码</Label>
            <FormSelect v-model="idFunction" :options="functionOptions" />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">起始地址</Label>
            <Input
              v-model="idAddress"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">数量</Label>
            <Input
              v-model="idQuantity"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">起始 ID</Label>
            <Input
              v-model="idStart"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">结束 ID</Label>
            <Input
              v-model="idEnd"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
        </div>

        <div v-if="activeKind === 'address'" class="grid grid-cols-6 gap-2">
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">功能码</Label>
            <FormSelect v-model="addressFunction" :options="functionOptions" />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">从站 ID</Label>
            <Input
              v-model="addressSlaveId"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">起始地址</Label>
            <Input
              v-model="addressStart"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">结束地址</Label>
            <Input
              v-model="addressEnd"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">块大小</Label>
            <Input
              v-model="blockSize"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">仅非零</Label>
            <div
              class="flex h-9 items-center rounded-lg border border-white/[0.08] bg-black/20 px-3"
            >
              <Switch v-model="nonZeroOnly" class="scale-90" />
            </div>
          </div>
        </div>

        <template v-if="activeKind === 'baud'">
          <div class="grid grid-cols-3 gap-2 rounded-xl border border-white/[0.05] bg-black/10 p-3">
            <div class="col-span-2 space-y-1">
              <Label class="text-[9px] text-muted-foreground/60">串口</Label>
              <FormSelect
                v-model="serialPort"
                :options="serialPorts.map((item) => ({ label: item.label, value: item.path }))"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[9px] text-muted-foreground/60">帧格式</Label>
              <FormSelect
                v-model="baudFraming"
                :options="[
                  { label: 'RTU', value: 'rtu' },
                  { label: 'ASCII', value: 'ascii' },
                ]"
              />
            </div>
            <div class="col-span-3 space-y-1">
              <Label class="text-[9px] text-muted-foreground/60">候选波特率（逗号分隔）</Label>
              <Input
                v-model="baudRateList"
                class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[9px] text-muted-foreground/60">从站 ID</Label>
              <Input
                v-model="baudSlaveId"
                class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[9px] text-muted-foreground/60">功能码</Label>
              <FormSelect v-model="baudFunction" :options="functionOptions" />
            </div>
            <div class="space-y-1">
              <Label class="text-[9px] text-muted-foreground/60">地址 / 数量</Label>
              <div class="grid grid-cols-2 gap-1">
                <Input
                  v-model="baudAddress"
                  class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
                />
                <Input
                  v-model="baudQuantity"
                  class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
                />
              </div>
            </div>
            <div class="space-y-1">
              <Label class="text-[9px] text-muted-foreground/60">超时 (ms)</Label>
              <Input
                v-model="baudTimeout"
                class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
              />
            </div>
            <div class="grid grid-cols-3 gap-1">
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">数据位</Label>
                <FormSelect
                  v-model="dataBits"
                  :options="['7', '8'].map((value) => ({ label: value, value }))"
                />
              </div>
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">校验</Label>
                <FormSelect
                  v-model="parity"
                  :options="[
                    { label: 'N', value: 'none' },
                    { label: 'E', value: 'even' },
                    { label: 'O', value: 'odd' },
                  ]"
                />
              </div>
              <div class="space-y-1">
                <Label class="text-[9px] text-muted-foreground/60">停止位</Label>
                <FormSelect
                  v-model="stopBits"
                  :options="['1', '2'].map((value) => ({ label: value, value }))"
                />
              </div>
            </div>
          </div>
        </template>

        <div
          v-if="activeKind === 'network'"
          class="grid grid-cols-3 gap-2 rounded-xl border border-white/[0.05] bg-black/10 p-3"
        >
          <div class="col-span-2 space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">CIDR 网段</Label>
            <Input
              v-model="cidr"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
              placeholder="192.168.1.0/24"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">端口</Label>
            <Input
              v-model="netPort"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">超时 (ms)</Label>
            <Input
              v-model="netTimeout"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">并发</Label>
            <Input
              v-model="netConcurrency"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[9px] text-muted-foreground/60">探测从站 ID</Label>
            <Input
              v-model="netSlaveId"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11px]"
            />
          </div>
          <div
            class="col-span-3 flex items-center justify-between rounded-lg border border-white/[0.06] px-3 py-2"
          >
            <div>
              <p class="text-[10px] text-foreground/80">验证 Modbus 响应</p>
              <p class="text-[8.5px] text-muted-foreground/45">关闭时仅检测 TCP 端口是否开放</p>
            </div>
            <Switch v-model="probeModbus" />
          </div>
        </div>

        <div class="rounded-xl border border-white/[0.05] bg-black/15">
          <div class="flex items-center gap-2 border-b border-white/[0.04] px-3 py-2">
            <span class="text-[10px] font-medium text-foreground/80">扫描进度</span>
            <span v-if="scans.progress" class="numeric text-[9px] text-muted-foreground/55">
              {{ scans.progress.done }}/{{ scans.progress.total }} · 发现 {{ scans.progress.found }}
            </span>
            <span
              v-if="scans.error"
              class="ml-auto rounded border border-error/20 bg-error/10 px-1.5 py-0.5 text-[8.5px] text-error"
            >
              {{ scans.error }}
            </span>
          </div>
          <div class="h-1 bg-white/[0.04]">
            <div
              class="h-full bg-gradient-to-r from-signal to-rx transition-[width] duration-200"
              :style="{ width: `${scans.percent}%` }"
            />
          </div>

          <div v-if="scans.results.length" class="scrollbar-thin max-h-[190px] overflow-y-auto">
            <table class="w-full border-collapse text-[9.5px]">
              <thead class="sticky top-0 bg-[#151a22]">
                <tr class="border-b border-white/[0.05]">
                  <th class="px-3 py-1.5 text-left font-medium text-muted-foreground/50">目标</th>
                  <th class="px-2 py-1.5 text-left font-medium text-muted-foreground/50">状态</th>
                  <th class="px-2 py-1.5 text-left font-medium text-muted-foreground/50">详情</th>
                  <th class="px-3 py-1.5 text-right font-medium text-muted-foreground/50">延时</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(result, index) in scans.results"
                  :key="`${result.label}-${index}`"
                  class="border-b border-white/[0.025]"
                >
                  <td class="numeric px-3 py-1 text-foreground/85">{{ result.label }}</td>
                  <td class="px-2 py-1">
                    <span
                      class="rounded border px-1 py-0.5 text-[8px] uppercase"
                      :class="statusTone[result.status]"
                    >
                      {{ result.status }}
                    </span>
                  </td>
                  <td class="truncate px-2 py-1 text-muted-foreground/65">{{ result.detail }}</td>
                  <td class="numeric px-3 py-1 text-right text-muted-foreground/55">
                    {{ result.latencyMs !== undefined ? formatLatency(result.latencyMs) : "—" }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div
            v-else-if="scans.progress"
            class="flex h-16 items-center justify-center gap-2 text-[10px] text-muted-foreground/55"
          >
            <span class="status-dot status-dot-live text-signal" />
            {{ scans.running ? "扫描中，等待响应…" : "扫描完成，未发现设备" }}
          </div>
          <div
            v-else
            class="flex h-16 items-center justify-center text-[10px] text-muted-foreground/45"
          >
            配置参数后开始扫描
          </div>
        </div>
      </div>

      <DialogFooter class="border-t border-white/[0.06] bg-black/10 px-5 py-3">
        <Button
          v-if="scans.results.length && !running"
          variant="ghost"
          size="sm"
          class="mr-auto h-8 gap-1.5 text-[10.5px] text-muted-foreground/60"
          @click="scans.reset()"
        >
          <Trash2 class="size-3" />
          清空结果
        </Button>
        <Button
          v-if="running"
          variant="outline"
          size="sm"
          class="h-8 gap-1.5 border-warn/25 bg-warn/10 text-[10.5px] text-warn"
          @click="scans.cancel()"
        >
          <Square class="size-3" />
          取消扫描
        </Button>
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-white/[0.08] bg-transparent text-[11px]"
          >
            关闭
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 gap-1.5 text-[11px]" :disabled="running" @click="startCurrent">
          <Radar class="size-3.5" />
          {{ running ? "扫描中…" : "开始扫描" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
