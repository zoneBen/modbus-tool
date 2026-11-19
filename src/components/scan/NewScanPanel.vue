<script setup lang="ts">
import { Radar, X } from "@lucide/vue";
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSerialPorts } from "@/composables/useSerialPorts";
import { MAX_SCAN_PORTS, parsePortList } from "@/lib/port-list";
import { useScansStore } from "@/stores/scans";
import type { Framing, FunctionCode, ScanKind, ScanLinkInput } from "@/types/models";

const emit = defineEmits<{
  (event: "close"): void;
  (event: "started", taskId: string): void;
}>();

const scans = useScansStore();
const { t } = useI18n();
const { ports: serialPorts, refresh: refreshSerialPorts } = useSerialPorts();

const activeKind = ref<ScanKind>("slaveId");
const linkModes = ["serial", "network"] as const;
const linkMode = ref<"network" | "serial">("serial");
const transport = ref<"tcp" | "udp">("tcp");
const framing = ref<Framing>("rtu");
const host = ref("192.168.1.10");
const port = ref("502");
const serialPort = ref("");
const baudRate = ref("9600");
const dataBits = ref("8");
const parity = ref<"none" | "even" | "odd">("none");
const stopBits = ref("1");
const timeoutMs = ref("200");
const starting = ref(false);

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
const netPorts = ref("502");
const netTimeout = ref("400");
const netConcurrency = ref("64");
const probeModbus = ref(true);
const grabBanner = ref(false);
const netSlaveId = ref("1");

const kindTabs = computed<Array<{ kind: ScanKind; label: string }>>(() => [
  { kind: "slaveId", label: t("scan.kindLabels.slaveId") },
  { kind: "address", label: t("scan.kindLabels.address") },
  { kind: "baud", label: t("scan.kindLabels.baud") },
  { kind: "network", label: t("scan.kindLabels.network") },
]);

const portParse = computed(() => parsePortList(netPorts.value));

const portPresets = computed(() => [
  { label: t("scan.presets.modbus"), value: "502" },
  { label: t("scan.presets.common"), value: "22, 80, 443, 502, 8080" },
  { label: t("scan.presets.web"), value: "80, 443, 8000, 8080, 8443" },
]);

function portErrorText(): string | null {
  const error = portParse.value.error;
  if (!error) return null;
  return t(`scan.portErrors.${error}`, { max: MAX_SCAN_PORTS });
}

function applyPortPreset(value: string): void {
  netPorts.value = value;
}

const functionOptions = [
  { label: "FC01", value: "1" },
  { label: "FC02", value: "2" },
  { label: "FC03", value: "3" },
  { label: "FC04", value: "4" },
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

const serialOptions = computed(() =>
  serialPorts.value.map((item) => ({ label: item.label, value: item.path })),
);

onMounted(async () => {
  await refreshSerialPorts();
  if (!serialPort.value) serialPort.value = serialPorts.value[0]?.path ?? "COM3";
});

function kindLabel(kind: ScanKind): string {
  return t(`scan.kindLabels.${kind}`);
}

function taskName(): string {
  const index = scans.tasks.filter((task) => task.kind === activeKind.value).length + 1;
  return `${kindLabel(activeKind.value)} ${String(index).padStart(2, "0")}`;
}

function linkSummary(): string {
  if (linkMode.value === "serial") {
    return `${serialPort.value || "COM"} · ${baudRate.value} ${dataBits.value}${parity.value === "none" ? "N" : parity.value === "even" ? "E" : "O"}${stopBits.value}`;
  }
  return `${transport.value.toUpperCase()} ${host.value}:${port.value}`;
}

function buildLink(): ScanLinkInput {
  const serial = linkMode.value === "serial";
  return {
    transport: serial ? "serial" : transport.value,
    framing: serial ? (framing.value === "mbap" ? "rtu" : framing.value) : framing.value,
    host: serial ? undefined : host.value.trim(),
    port: serial ? undefined : Math.max(1, Number(port.value) || 502),
    serialPort: serial ? serialPort.value || "COM3" : undefined,
    baudRate: serial ? Math.max(1, Number(baudRate.value) || 9600) : undefined,
    dataBits: Number(dataBits.value) || 8,
    parity: parity.value,
    stopBits: Number(stopBits.value) || 1,
    timeoutMs: Math.max(50, Number(timeoutMs.value) || 200),
  };
}

async function start(): Promise<void> {
  if (starting.value) return;
  starting.value = true;
  try {
    let taskId: string;
    if (activeKind.value === "slaveId") {
      taskId = await scans.startSlaveIds(
        {
          link: buildLink(),
          functionCode: Number(idFunction.value) as FunctionCode,
          address: Math.max(0, Number(idAddress.value) || 0),
          quantity: Math.max(1, Number(idQuantity.value) || 1),
          idStart: Math.min(255, Math.max(0, Number(idStart.value) || 0)),
          idEnd: Math.min(255, Math.max(0, Number(idEnd.value) || 0)),
        },
        taskName(),
        `${linkSummary()} · ${idStart.value}–${idEnd.value}`,
      );
    } else if (activeKind.value === "address") {
      taskId = await scans.startAddresses(
        {
          link: buildLink(),
          functionCode: Number(addressFunction.value) as FunctionCode,
          slaveId: Math.max(1, Number(addressSlaveId.value) || 1),
          startAddress: Math.max(0, Number(addressStart.value) || 0),
          endAddress: Math.max(0, Number(addressEnd.value) || 0),
          blockSize: Math.max(1, Number(blockSize.value) || 10),
          nonZeroOnly: nonZeroOnly.value,
        },
        taskName(),
        `${linkSummary()} · ${addressStart.value}–${addressEnd.value}`,
      );
    } else if (activeKind.value === "baud") {
      const rates = baudRateList.value
        .split(/[\s,;]+/)
        .map((entry) => Number(entry))
        .filter((entry) => Number.isFinite(entry) && entry > 0);
      if (!rates.length) {
        toast.error(t("scan.noBaudRates"));
        return;
      }
      taskId = await scans.startBaudRates(
        {
          serialPort: serialPort.value || "COM3",
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
        },
        taskName(),
        `${serialPort.value || "COM3"} · ${rates.length} ${t("scan.baudRate")}`,
      );
    } else {
      if (!cidr.value.trim()) {
        toast.error(t("scan.cidrRequired"));
        return;
      }
      const parsedPorts = parsePortList(netPorts.value);
      if (parsedPorts.error) {
        toast.error(t(`scan.portErrors.${parsedPorts.error}`, { max: MAX_SCAN_PORTS }));
        return;
      }
      taskId = await scans.startNetwork(
        {
          cidr: cidr.value.trim(),
          port: parsedPorts.ports[0],
          ports: parsedPorts.ports,
          timeoutMs: Math.max(100, Number(netTimeout.value) || 400),
          concurrency: Math.min(128, Math.max(1, Number(netConcurrency.value) || 64)),
          probeModbus: probeModbus.value,
          grabBanner: grabBanner.value,
          slaveId: Math.max(1, Number(netSlaveId.value) || 1),
        },
        taskName(),
        `${cidr.value.trim()} · ${parsedPorts.ports.length} ${t("scan.ports")}`,
      );
    }
    emit("started", taskId);
    emit("close");
  } catch (error: unknown) {
    toast.error(t("scan.startFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    starting.value = false;
  }
}
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <header class="flex h-[54px] shrink-0 items-center gap-3 border-b border-hairline px-4">
      <span
        class="flex size-8 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
      >
        <Radar class="size-4" />
      </span>
      <div class="min-w-0 flex-1">
        <p class="text-[13px] font-semibold">{{ t("scan.newScan") }}</p>
        <p class="mt-0.5 truncate text-[10.5px] text-muted-foreground">
          {{ t("scan.subtitle") }}
        </p>
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        class="rounded-lg text-muted-foreground hover:text-foreground"
        :aria-label="t('common.cancel')"
        @click="emit('close')"
      >
        <X class="size-4" />
      </Button>
    </header>

    <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-4 py-4">
      <div class="mx-auto max-w-[960px] space-y-4">
        <div class="grid grid-cols-4 gap-1 rounded-xl border border-hairline bg-overlay-1 p-1">
          <button
            v-for="tab in kindTabs"
            :key="tab.kind"
            type="button"
            class="rounded-lg px-2 py-2 text-[11.5px] transition-colors"
            :class="
              activeKind === tab.kind
                ? 'bg-overlay-3 text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            "
            @click="activeKind = tab.kind"
          >
            {{ tab.label }}
          </button>
        </div>

        <div
          v-if="activeKind === 'slaveId' || activeKind === 'address'"
          class="rounded-xl border border-hairline bg-well-soft p-3"
        >
          <div class="mb-3 flex items-center justify-between">
            <p class="text-[11px] font-medium text-foreground">{{ t("scan.link") }}</p>
            <div class="flex rounded-lg border border-hairline bg-well p-0.5">
              <button
                v-for="mode in linkModes"
                :key="mode"
                type="button"
                class="rounded-md px-2 py-0.5 text-[11px] transition-colors"
                :class="
                  linkMode === mode
                    ? 'bg-overlay-3 text-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                "
                @click="linkMode = mode"
              >
                {{ mode === "network" ? t("scan.linkNetwork") : t("scan.linkSerial") }}
              </button>
            </div>
          </div>

          <div v-if="linkMode === 'network'" class="grid grid-cols-4 gap-2">
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.transport") }}</Label>
              <FormSelect
                v-model="transport"
                :options="[
                  { label: 'TCP', value: 'tcp' },
                  { label: 'UDP', value: 'udp' },
                ]"
              />
            </div>
            <div class="col-span-2 space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.host") }}</Label>
              <Input
                v-model="host"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.port") }}</Label>
              <Input
                v-model="port"
                inputmode="numeric"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>
          <div v-else class="grid grid-cols-4 gap-2">
            <div class="col-span-2 space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.serialPort") }}</Label>
              <FormSelect v-model="serialPort" :options="serialOptions" />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.baudRate") }}</Label>
              <Input
                v-model="baudRate"
                inputmode="numeric"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.parity") }}</Label>
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
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.dataBits") }}</Label>
              <FormSelect
                v-model="dataBits"
                :options="['7', '8'].map((value) => ({ label: value, value }))"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.stopBits") }}</Label>
              <FormSelect
                v-model="stopBits"
                :options="['1', '2'].map((value) => ({ label: value, value }))"
              />
            </div>
          </div>

          <div class="mt-2 grid grid-cols-3 gap-2">
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.framing") }}</Label>
              <FormSelect v-model="framing" :options="framingOptions" />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.timeout") }}</Label>
              <Input
                v-model="timeoutMs"
                inputmode="numeric"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>
        </div>

        <div v-if="activeKind === 'slaveId'" class="grid grid-cols-5 gap-2">
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.function") }}</Label>
            <FormSelect v-model="idFunction" :options="functionOptions" />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.address") }}</Label>
            <Input
              v-model="idAddress"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.quantity") }}</Label>
            <Input
              v-model="idQuantity"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.idStart") }}</Label>
            <Input
              v-model="idStart"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.idEnd") }}</Label>
            <Input
              v-model="idEnd"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
        </div>

        <div v-if="activeKind === 'address'" class="grid grid-cols-6 gap-2">
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.function") }}</Label>
            <FormSelect v-model="addressFunction" :options="functionOptions" />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.slaveId") }}</Label>
            <Input
              v-model="addressSlaveId"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.startAddress") }}</Label>
            <Input
              v-model="addressStart"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.endAddress") }}</Label>
            <Input
              v-model="addressEnd"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.blockSize") }}</Label>
            <Input
              v-model="blockSize"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.nonZeroOnly") }}</Label>
            <div
              class="flex h-9 items-center rounded-lg border border-hairline-strong bg-well px-3"
            >
              <Switch v-model="nonZeroOnly" class="scale-90" />
            </div>
          </div>
        </div>

        <div
          v-if="activeKind === 'baud'"
          class="grid grid-cols-3 gap-2 rounded-xl border border-hairline bg-well-soft p-3"
        >
          <div class="col-span-2 space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.serialPort") }}</Label>
            <FormSelect v-model="serialPort" :options="serialOptions" />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.framing") }}</Label>
            <FormSelect
              v-model="baudFraming"
              :options="[
                { label: 'RTU', value: 'rtu' },
                { label: 'ASCII', value: 'ascii' },
              ]"
            />
          </div>
          <div class="col-span-3 space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.baudCandidates") }}</Label>
            <Input
              v-model="baudRateList"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.slaveId") }}</Label>
            <Input
              v-model="baudSlaveId"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.function") }}</Label>
            <FormSelect v-model="baudFunction" :options="functionOptions" />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground"
              >{{ t("scan.address") }} / {{ t("scan.quantity") }}</Label
            >
            <div class="grid grid-cols-2 gap-1">
              <Input
                v-model="baudAddress"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
              <Input
                v-model="baudQuantity"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{ t("scan.timeout") }}</Label>
            <Input
              v-model="baudTimeout"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="grid grid-cols-3 gap-1">
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.dataBits") }}</Label>
              <FormSelect
                v-model="dataBits"
                :options="['7', '8'].map((value) => ({ label: value, value }))"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.parity") }}</Label>
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
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.stopBits") }}</Label>
              <FormSelect
                v-model="stopBits"
                :options="['1', '2'].map((value) => ({ label: value, value }))"
              />
            </div>
          </div>
        </div>

        <div
          v-if="activeKind === 'network'"
          class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3"
        >
          <div class="grid grid-cols-3 gap-2">
            <div class="col-span-2 space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.cidr") }}</Label>
              <Input
                v-model="cidr"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
                placeholder="192.168.1.0/24"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.timeout") }}</Label>
              <Input
                v-model="netTimeout"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>

          <div class="space-y-1.5">
            <div class="flex flex-wrap items-center justify-between gap-2">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.ports") }}</Label>
              <div class="flex flex-wrap items-center gap-1">
                <button
                  v-for="preset in portPresets"
                  :key="preset.value"
                  type="button"
                  class="rounded-md border border-hairline bg-overlay-1 px-1.5 py-0.5 text-[10px] text-muted-foreground transition-colors hover:text-foreground"
                  @click="applyPortPreset(preset.value)"
                >
                  {{ preset.label }}
                </button>
              </div>
            </div>
            <Input
              v-model="netPorts"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              :class="portParse.error ? 'border-error/50' : ''"
              placeholder="22, 80, 443, 502, 8000-8100"
            />
            <p
              class="text-[10.5px] leading-relaxed"
              :class="portParse.error ? 'text-error' : 'text-muted-foreground'"
            >
              {{
                portErrorText() ??
                t("scan.portsHint", {
                  count: portParse.ports.length,
                  max: MAX_SCAN_PORTS,
                })
              }}
            </p>
          </div>

          <div class="grid grid-cols-2 gap-2">
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.concurrency") }}</Label>
              <Input
                v-model="netConcurrency"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t("scan.probeSlaveId") }}</Label>
              <Input
                v-model="netSlaveId"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-2">
            <div
              class="flex items-center justify-between gap-3 rounded-lg border border-hairline px-3 py-2"
            >
              <div>
                <p class="text-[11px] text-foreground">{{ t("scan.probeModbus") }}</p>
                <p class="text-[10.5px] text-muted-foreground">{{ t("scan.probeModbusHint") }}</p>
              </div>
              <Switch v-model="probeModbus" />
            </div>
            <div
              class="flex items-center justify-between gap-3 rounded-lg border border-hairline px-3 py-2"
            >
              <div>
                <p class="text-[11px] text-foreground">{{ t("scan.grabBanner") }}</p>
                <p class="text-[10.5px] text-muted-foreground">{{ t("scan.grabBannerHint") }}</p>
              </div>
              <Switch v-model="grabBanner" />
            </div>
          </div>
        </div>
      </div>
    </div>

    <footer
      class="flex shrink-0 items-center justify-end gap-2 border-t border-hairline bg-well-soft px-4 py-3"
    >
      <Button
        variant="outline"
        size="sm"
        class="h-8 border-hairline-strong text-[12px]"
        @click="emit('close')"
      >
        {{ t("common.cancel") }}
      </Button>
      <Button size="sm" class="h-8 gap-1.5 text-[12px]" :disabled="starting" @click="start">
        <Radar class="size-3.5" />
        {{ starting ? t("scan.starting") : t("scan.start") }}
      </Button>
    </footer>
  </div>
</template>
