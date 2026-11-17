<script setup lang="ts">
import { Cpu, Network, Server, Usb } from "@lucide/vue";
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useSerialPorts } from "@/composables/useSerialPorts";
import { useSessionsStore } from "@/stores/sessions";
import type { Framing, SessionRole, TransportKind } from "@/types/models";
import SerialPortField from "./SerialPortField.vue";

const open = defineModel<boolean>("open", { default: false });

const sessions = useSessionsStore();

const roleOptions = ["master", "slave"] as const;
const name = ref("");
const role = ref<SessionRole>("master");
const linkMode = ref<"network" | "serial">("serial");
const transport = ref<"tcp" | "udp">("tcp");
const framing = ref<Framing>("rtu");
const host = ref("192.168.1.10");
const port = ref("502");
const slaveId = ref("1");
const serialPort = ref("");
const baudRate = ref(9600);
const parity = ref("none");
const dataBits = ref(8);
const stopBits = ref(1);
const {
  ports: serialPorts,
  loading: serialPortsLoading,
  error: serialPortsError,
  refresh: refreshSerialPorts,
} = useSerialPorts();
const busy = ref(false);

const framingOptions = computed(() =>
  linkMode.value === "serial"
    ? [
        { label: "RTU", value: "rtu" as const },
        { label: "ASCII", value: "ascii" as const },
      ]
    : [
        { label: "MBAP", value: "mbap" as const },
        { label: "RTU over TCP/UDP", value: "rtu" as const },
        { label: "ASCII over TCP/UDP", value: "ascii" as const },
      ],
);

const defaultName = computed(() =>
  role.value === "slave"
    ? `模拟器 · 从站 ${(sessions.slaveCount + 1).toString().padStart(2, "0")}`
    : `${linkMode.value === "serial" ? "串口" : "网络"} 会话 ${(sessions.masterCount + 1).toString().padStart(2, "0")}`,
);

const endpoint = computed(() => {
  if (linkMode.value === "serial") {
    const port = serialPort.value.trim() || "未选择串口";
    return `${port} · ${baudRate.value} ${dataBits.value}${
      parity.value === "none" ? "N" : parity.value === "even" ? "E" : "O"
    }${stopBits.value}`;
  }
  const targetHost = role.value === "slave" ? "0.0.0.0" : host.value;
  return `${targetHost}:${port.value}`;
});

watch(open, (isOpen) => {
  if (isOpen) void refreshSerialPorts();
});

watch(
  serialPorts,
  (ports) => {
    if (ports.length && !ports.some((port) => port.path === serialPort.value)) {
      serialPort.value = ports[0]?.path ?? "";
    }
  },
  { immediate: true },
);

watch(role, (value) => {
  if (value === "slave") {
    linkMode.value = "network";
    framing.value = "mbap";
    transport.value = "tcp";
    host.value = "0.0.0.0";
    if (port.value === "502") port.value = "5020";
  } else {
    if (host.value === "0.0.0.0") host.value = "192.168.1.10";
    if (port.value === "5020") port.value = "502";
  }
});

watch(linkMode, (value) => {
  framing.value = value === "serial" ? "rtu" : "mbap";
});

async function submit(): Promise<void> {
  if (busy.value) return;
  if (role.value === "slave" && linkMode.value === "network" && transport.value === "udp") {
    toast.error("从站模拟暂不支持 UDP，请选择 TCP");
    return;
  }
  if (linkMode.value === "network" && role.value === "master" && !host.value.trim()) {
    toast.error("请输入主机地址");
    return;
  }
  if (linkMode.value === "serial" && !serialPort.value.trim()) {
    toast.error("请选择或输入串口端口");
    return;
  }
  busy.value = true;
  try {
    const session = await sessions.createSession({
      name: name.value.trim() || defaultName.value,
      role: role.value,
      transport: (linkMode.value === "serial" ? "serial" : transport.value) as TransportKind,
      framing: framing.value,
      endpoint: endpoint.value,
      host:
        linkMode.value === "network"
          ? role.value === "slave"
            ? "0.0.0.0"
            : host.value
          : undefined,
      port: linkMode.value === "network" ? Number(port.value) || 502 : undefined,
      serialPort: linkMode.value === "serial" ? serialPort.value.trim() : undefined,
      baudRate: baudRate.value,
      dataBits: dataBits.value,
      parity: parity.value as "none" | "even" | "odd",
      stopBits: stopBits.value,
      slaveId: role.value === "slave" ? Math.max(1, Number(slaveId.value) || 1) : undefined,
    });
    toast.success(`已创建会话「${session.name}」`, {
      description:
        role.value === "slave"
          ? "从站模拟器已启动监听。"
          : sessions.demo
            ? "演示模式已自动建立连接。"
            : "正在连接设备…",
    });
    name.value = "";
    open.value = false;
  } catch (error: unknown) {
    toast.error("创建会话失败", {
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
      <DialogHeader class="relative border-b border-hairline px-5 py-4">
        <div
          class="pointer-events-none absolute inset-0 bg-gradient-to-r from-signal/[0.07] to-transparent"
        />
        <DialogTitle class="relative flex items-center gap-2 text-[14px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <Network class="size-3.5" />
          </span>
          新建会话
        </DialogTitle>
        <DialogDescription class="relative text-[11.5px] text-muted-foreground">
          {{
            role === "slave"
              ? "从站模拟器会监听所选链路，响应主站的读写请求。"
              : "配置链路参数并连接目标设备。"
          }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4 px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">会话名称</Label>
            <Input
              v-model="name"
              :placeholder="defaultName"
              class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">角色</Label>
            <div class="grid grid-cols-2 gap-1 rounded-lg border border-hairline bg-well p-0.5">
              <button
                v-for="option in roleOptions"
                :key="option"
                type="button"
                class="flex h-8 items-center justify-center gap-1.5 rounded-md text-[11.5px] transition-colors"
                :class="
                  role === option
                    ? option === 'slave'
                      ? 'bg-sim/15 text-sim'
                      : 'bg-signal/15 text-signal'
                    : 'text-muted-foreground hover:text-foreground'
                "
                @click="role = option"
              >
                <Cpu v-if="option === 'master'" class="size-3.5" />
                <Server v-else class="size-3.5" />
                {{ option === "master" ? "主站" : "从站" }}
              </button>
            </div>
          </div>
        </div>

        <div class="space-y-2">
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

          <div v-if="role === 'slave'" class="grid grid-cols-3 gap-3">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">从站 ID</Label>
              <Input
                v-model="slaveId"
                inputmode="numeric"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div
              class="col-span-2 flex items-center rounded-lg border border-sim/20 bg-sim/5 px-3 text-[11px] leading-relaxed text-muted-foreground"
            >
              模拟映射：保持/输入寄存器 0–15、线圈/离散输入 0–7；主站写入会实时回显。
            </div>
          </div>

          <div v-if="linkMode === 'network'" class="grid grid-cols-3 gap-3">
            <div class="col-span-1 space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">传输层</Label>
              <FormSelect
                v-model="transport"
                :options="
                  role === 'slave'
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
                :disabled="role === 'slave'"
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
                class="numeric flex h-9 items-center truncate rounded-lg border border-hairline bg-overlay-1 px-3 text-[11.5px] text-muted-foreground"
              >
                {{ endpoint }}
              </div>
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
        <Button size="sm" class="h-8 gap-1.5 text-[12px]" :disabled="busy" @click="submit">
          <Network class="size-3.5" />
          {{ busy ? "连接中…" : "连接会话" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
