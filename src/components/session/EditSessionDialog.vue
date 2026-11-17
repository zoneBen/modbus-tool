<script setup lang="ts">
import { SlidersHorizontal, Usb, Network } from "@lucide/vue";
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useSerialPorts } from "@/composables/useSerialPorts";
import { useSessionsStore } from "@/stores/sessions";
import type { Framing, UpdateSessionInput } from "@/types/models";
import SerialPortField from "./SerialPortField.vue";

const open = defineModel<boolean>("open", { default: false });
const sessions = useSessionsStore();
const session = computed(() => sessions.activeSession);

const name = ref("");
const linkMode = ref<"network" | "serial">("serial");
const transport = ref<"tcp" | "udp">("tcp");
const framing = ref<Framing>("rtu");
const host = ref("127.0.0.1");
const port = ref("502");
const serialPort = ref("");
const baudRate = ref(9600);
const parity = ref("none");
const dataBits = ref(8);
const stopBits = ref(1);
const timeoutMs = ref("1000");
const retries = ref("1");
const autoReconnect = ref(true);
const {
  ports: serialPorts,
  loading: serialPortsLoading,
  error: serialPortsError,
  refresh: refreshSerialPorts,
} = useSerialPorts();
const busy = ref(false);

const isSlave = computed(() => session.value?.role === "slave");

const framingOptions = computed(() =>
  linkMode.value === "serial"
    ? [
        { label: "RTU", value: "rtu" as const },
        { label: "ASCII", value: "ascii" as const },
      ]
    : [
        { label: "MBAP", value: "mbap" as const },
        { label: "RTU over TCP", value: "rtu" as const },
        { label: "ASCII over TCP", value: "ascii" as const },
      ],
);

const endpoint = computed(() => {
  if (linkMode.value === "serial") {
    const port = serialPort.value.trim() || "未选择串口";
    return `${port} · ${baudRate.value} ${dataBits.value}${
      parity.value === "none" ? "N" : parity.value === "even" ? "E" : "O"
    }${stopBits.value}`;
  }
  return `${isSlave.value ? "0.0.0.0" : host.value}:${port.value}`;
});

watch(open, async (isOpen) => {
  if (!isOpen || !session.value) return;
  const current = session.value;
  name.value = current.name;
  linkMode.value = current.transport === "serial" ? "serial" : "network";
  transport.value = current.transport === "udp" ? "udp" : "tcp";
  framing.value = current.framing;
  timeoutMs.value = String(current.timeoutMs ?? 1000);
  retries.value = String(current.retries ?? 1);
  autoReconnect.value = current.autoReconnect ?? true;

  if (current.transport === "serial") {
    serialPort.value = current.serialPort ?? current.endpoint.split("·")[0]?.trim() ?? "";
    baudRate.value = current.baudRate ?? 9600;
    dataBits.value = current.dataBits ?? 8;
    parity.value = current.parity ?? "none";
    stopBits.value = current.stopBits ?? 1;
  } else {
    const [endpointHost, endpointPort] = current.endpoint.split(":");
    host.value = current.host ?? endpointHost ?? "127.0.0.1";
    port.value = String(current.port ?? (Number(endpointPort) || 502));
  }
  if (isSlave.value) host.value = "0.0.0.0";

  await refreshSerialPorts();
});

watch(linkMode, (value) => {
  if (value === "serial" && framing.value === "mbap") framing.value = "rtu";
});

watch(isSlave, (value) => {
  if (value) {
    transport.value = "tcp";
    host.value = "0.0.0.0";
  }
});

async function submit(): Promise<void> {
  const current = session.value;
  if (!current || busy.value) return;
  if (linkMode.value === "network" && !isSlave.value && !host.value.trim()) {
    toast.error("请输入主机地址");
    return;
  }
  if (linkMode.value === "serial" && !serialPort.value.trim()) {
    toast.error("请选择或输入串口端口");
    return;
  }
  const input: UpdateSessionInput = {
    name: name.value.trim() || current.name,
    transport: linkMode.value === "serial" ? "serial" : transport.value,
    framing: framing.value,
    endpoint: endpoint.value,
    host: linkMode.value === "network" ? (isSlave.value ? "0.0.0.0" : host.value) : undefined,
    port: linkMode.value === "network" ? Number(port.value) || 502 : undefined,
    serialPort: linkMode.value === "serial" ? serialPort.value.trim() : undefined,
    baudRate: baudRate.value,
    dataBits: dataBits.value,
    parity: parity.value as "none" | "even" | "odd",
    stopBits: stopBits.value,
    timeoutMs: Math.max(50, Number(timeoutMs.value) || 1000),
    retries: Math.max(0, Math.min(10, Number(retries.value) || 0)),
    autoReconnect: autoReconnect.value,
  };
  busy.value = true;
  try {
    await sessions.updateSession(current.id, input);
    toast.success("会话参数已更新", {
      description: current.status === "connected" ? "链路已按新参数重启。" : "下次启动时生效。",
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error("更新会话失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[580px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <SlidersHorizontal class="size-3.5" />
          </span>
          会话参数
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ session?.name ?? "未选择会话" }} ·
          {{ session?.role === "slave" ? "从站模拟器" : "主站" }}；
          保存后如果会话正在运行，会用新参数重启链路。
        </DialogDescription>
      </DialogHeader>

      <div class="scrollbar-thin max-h-[68vh] space-y-4 overflow-y-auto px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">会话名称</Label>
            <Input
              v-model="name"
              class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">会话状态</Label>
            <div
              class="numeric flex h-9 items-center rounded-lg border border-hairline bg-well-soft px-3 text-[12px] text-muted-foreground"
            >
              {{ session?.status ?? "—" }}
            </div>
          </div>
        </div>

        <Tabs v-model="linkMode">
          <TabsList class="grid w-full grid-cols-2 border border-hairline bg-well">
            <TabsTrigger value="serial" class="gap-1.5 text-[12px]">
              <Usb class="size-3.5" />
              串口
            </TabsTrigger>
            <TabsTrigger value="network" class="gap-1.5 text-[12px]">
              <Network class="size-3.5" />
              网络
            </TabsTrigger>
          </TabsList>
        </Tabs>

        <div v-if="linkMode === 'network'" class="grid grid-cols-3 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">传输层</Label>
            <FormSelect
              v-model="transport"
              :options="
                isSlave
                  ? [{ label: 'TCP', value: 'tcp' }]
                  : [
                      { label: 'TCP', value: 'tcp' },
                      { label: 'UDP', value: 'udp' },
                    ]
              "
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">主机地址</Label>
            <Input
              v-model="host"
              :disabled="isSlave"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">端口</Label>
            <Input
              v-model="port"
              type="number"
              min="1"
              max="65535"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
        </div>

        <div v-else class="grid grid-cols-2 gap-3">
          <SerialPortField
            v-model="serialPort"
            class="col-span-2"
            :ports="serialPorts"
            :loading="serialPortsLoading"
            :error="serialPortsError"
            @refresh="refreshSerialPorts"
          />
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">波特率</Label>
            <FormSelect
              v-model="baudRate"
              :options="
                [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200].map((value) => ({
                  label: value.toString(),
                  value,
                }))
              "
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">校验 / 数据位 / 停止位</Label>
            <div class="grid grid-cols-3 gap-1.5">
              <FormSelect
                v-model="parity"
                :options="[
                  { label: 'N', value: 'none' },
                  { label: 'E', value: 'even' },
                  { label: 'O', value: 'odd' },
                ]"
              />
              <FormSelect
                v-model="dataBits"
                :options="[7, 8].map((value) => ({ label: value.toString(), value }))"
              />
              <FormSelect
                v-model="stopBits"
                :options="[1, 2].map((value) => ({ label: value.toString(), value }))"
              />
            </div>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">帧格式</Label>
            <FormSelect v-model="framing" :options="framingOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">链路端点</Label>
            <div
              class="numeric flex h-9 items-center truncate rounded-lg border border-hairline bg-well-soft px-3 text-[11px] text-muted-foreground"
            >
              {{ endpoint }}
            </div>
          </div>
        </div>

        <div class="grid grid-cols-3 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">超时 (ms)</Label>
            <Input
              v-model="timeoutMs"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">重试次数</Label>
            <Input
              v-model="retries"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">自动重连</Label>
            <div
              class="flex h-9 items-center justify-between rounded-lg border border-hairline bg-well px-3"
            >
              <span class="text-[11px] text-muted-foreground">
                {{ autoReconnect ? "开启" : "关闭" }}
              </span>
              <Switch v-model="autoReconnect" />
            </div>
          </div>
        </div>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-hairline-strong bg-transparent text-[12px]"
          >
            取消
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 min-w-[84px] text-[12px]" :disabled="busy" @click="submit">
          {{ busy ? "保存中…" : "保存参数" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
