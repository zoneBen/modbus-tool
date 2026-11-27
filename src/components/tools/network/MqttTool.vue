<script setup lang="ts">
import {
  ArrowDownLeft,
  ArrowUpRight,
  Copy,
  FolderOpen,
  Plus,
  Radio,
  Send,
  Trash2,
  X,
} from "@lucide/vue";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useMqttStore } from "@/stores/mqtt";
import type { MqttBrokerConfig } from "@/stores/mqtt";
import type { MqttProtocol, MqttStatus } from "@/types/tools";

const { t } = useI18n();
const mqtt = useMqttStore();
const busy = ref(false);
const subscriptionTopic = ref("");
const subscriptionQos = ref(0);
const publishTopic = ref("");
const publishPayload = ref("");
const publishMode = ref<"text" | "hex">("text");
const publishQos = ref(0);
const publishRetain = ref(false);
const logMode = ref<"text" | "hex">("text");
const messageBox = ref<HTMLElement | null>(null);

const active = computed(() => mqtt.brokers.find((broker) => broker.id === mqtt.activeId) ?? null);
const status = computed<MqttStatus>(() =>
  active.value ? (mqtt.statuses[active.value.id] ?? "disconnected") : "disconnected",
);
const detail = computed(() => (active.value ? (mqtt.details[active.value.id] ?? null) : null));
const running = computed(() => (active.value ? mqtt.isRunning(active.value.id) : false));
const entries = computed(() => (active.value ? mqtt.logEntries(active.value.id) : []));
const visibleEntries = computed(() => entries.value);
const brokerSubscriptions = computed(() =>
  active.value ? (mqtt.subscriptions[active.value.id] ?? []) : [],
);

function field<K extends keyof MqttBrokerConfig>(key: K) {
  return computed<MqttBrokerConfig[K]>({
    get: () => active.value?.[key] as MqttBrokerConfig[K],
    set: (value) => {
      const broker = active.value;
      if (!broker) return;
      mqtt.updateBroker(broker.id, { [key]: value } as Partial<MqttBrokerConfig>);
    },
  });
}

const name = field("name");
const host = field("host");
const protocol = field("protocol");
const clientId = field("clientId");
const username = field("username");
const tls = field("tls");
const caPath = field("caPath");
const cleanSession = field("cleanSession");
const port = computed({
  get: () => String(active.value?.port ?? 1883),
  set: (value: string) => {
    const broker = active.value;
    if (!broker) return;
    mqtt.updateBroker(broker.id, {
      port: Math.min(65_535, Math.max(1, Number(value) || 1883)),
    });
  },
});
const keepAlive = computed({
  get: () => String(active.value?.keepAlive ?? 60),
  set: (value: string) => {
    const broker = active.value;
    if (!broker) return;
    mqtt.updateBroker(broker.id, {
      keepAlive: Math.min(600, Math.max(5, Number(value) || 60)),
    });
  },
});
const password = computed({
  get: () => (active.value ? (mqtt.passwords[active.value.id] ?? "") : ""),
  set: (value: string) => {
    if (active.value) mqtt.passwords[active.value.id] = value;
  },
});

const protocolOptions: Array<{ label: string; value: MqttProtocol }> = [
  { label: "MQTT 3.1.1", value: "3.1.1" },
  { label: "MQTT 5", value: "5" },
];
const qosOptions = [
  { label: "QoS 0", value: 0 },
  { label: "QoS 1", value: 1 },
  { label: "QoS 2", value: 2 },
];
const modeOptions = computed(() => [
  { label: t("networkTools.mqtt.text"), value: "text" as const },
  { label: t("networkTools.mqtt.hex"), value: "hex" as const },
]);

const statusTone: Record<MqttStatus, string> = {
  connected: "text-signal",
  connecting: "text-warn",
  reconnecting: "text-warn",
  disconnected: "text-muted-foreground",
  error: "text-error",
};

onMounted(async () => {
  await mqtt.init();
});

watch(
  () => visibleEntries.value.length,
  async () => {
    await nextTick();
    if (messageBox.value) messageBox.value.scrollTop = 0;
  },
);

async function toggleConnection(): Promise<void> {
  const broker = active.value;
  if (!broker || busy.value) return;
  busy.value = true;
  try {
    if (running.value) {
      await mqtt.stop(broker.id);
    } else {
      await mqtt.start(broker.id);
    }
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  } finally {
    busy.value = false;
  }
}

function addBroker(): void {
  mqtt.addBroker();
}

async function removeBroker(): Promise<void> {
  const broker = active.value;
  if (!broker) return;
  if (!window.confirm(t("networkTools.mqtt.deleteConfirm", { name: broker.name }))) return;
  await mqtt.removeBroker(broker.id).catch(() => undefined);
}

async function browseCa(): Promise<void> {
  const path = await ipc.pickFile([
    { name: t("networkTools.http.certificateFiles"), extensions: ["pem", "crt", "cer", "der"] },
  ]);
  if (path) caPath.value = path;
}

async function addSubscription(): Promise<void> {
  const broker = active.value;
  const topic = subscriptionTopic.value.trim();
  if (!broker || !topic) return;
  try {
    await mqtt.subscribe(broker.id, topic, subscriptionQos.value);
    subscriptionTopic.value = "";
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

async function removeSubscription(topic: string): Promise<void> {
  const broker = active.value;
  if (!broker) return;
  try {
    await mqtt.unsubscribe(broker.id, topic);
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

async function publish(): Promise<void> {
  const broker = active.value;
  if (!broker || !publishTopic.value.trim()) return;
  try {
    await mqtt.publish(
      broker.id,
      publishTopic.value.trim(),
      publishPayload.value,
      publishMode.value,
      publishQos.value,
      publishRetain.value,
    );
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

const { copy: copyFeedback, isCopied } = useCopyFeedback();

async function copy(key: string, value: string): Promise<void> {
  if (!value) return;
  await copyFeedback(key, value);
}

function formatTime(timestampMs: number): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(timestampMs));
}

function entryPayload(entry: { payloadText: string; payloadHex: string }): string {
  return logMode.value === "hex" ? entry.payloadHex : entry.payloadText;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.mqtt.title')"
    :description="t('networkTools.tools.mqtt.description')"
    :icon="Radio"
  >
    <div class="mx-auto flex h-full max-w-[1420px] min-h-[560px] gap-3">
      <aside
        class="flex w-[218px] shrink-0 flex-col rounded-xl border border-hairline bg-well-soft"
      >
        <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
          <span class="text-[11px] font-medium">{{ t("networkTools.mqtt.brokers") }}</span>
          <Button
            variant="ghost"
            size="sm"
            class="h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
            @click="addBroker"
          >
            <Plus class="size-3" />
            {{ t("networkTools.mqtt.addBroker") }}
          </Button>
        </div>
        <div class="scrollbar-thin min-h-0 flex-1 space-y-1 overflow-y-auto p-2">
          <button
            v-for="broker in mqtt.brokers"
            :key="broker.id"
            type="button"
            class="w-full rounded-lg border px-2.5 py-2 text-left transition-colors"
            :class="
              broker.id === mqtt.activeId
                ? 'border-rx/30 bg-rx/10'
                : 'border-transparent hover:border-hairline hover:bg-overlay-1'
            "
            @click="mqtt.activeId = broker.id"
          >
            <div class="flex items-center gap-1.5">
              <span
                class="size-1.5 shrink-0 rounded-full"
                :class="{
                  'bg-signal': mqtt.statuses[broker.id] === 'connected',
                  'bg-warn animate-pulse':
                    mqtt.statuses[broker.id] === 'connecting' ||
                    mqtt.statuses[broker.id] === 'reconnecting',
                  'bg-error': mqtt.statuses[broker.id] === 'error',
                  'bg-muted-foreground/40': !mqtt.statuses[broker.id],
                }"
              />
              <span class="min-w-0 flex-1 truncate text-[11.5px] font-medium">{{
                broker.name
              }}</span>
            </div>
            <p class="numeric mt-0.5 truncate text-[10px] text-muted-foreground">
              {{ broker.host }}:{{ broker.port }}
            </p>
          </button>
        </div>
        <div class="border-t border-hairline px-2.5 py-2 text-[10px] text-muted-foreground">
          {{ t("networkTools.mqtt.parallelHint") }}
        </div>
      </aside>

      <div v-if="active" class="min-w-0 flex-1 space-y-3">
        <div class="rounded-xl border border-hairline bg-well-soft p-3">
          <div class="mb-2 flex flex-wrap items-center gap-2">
            <span class="flex items-center gap-1.5 text-[11px]" :class="statusTone[status]">
              <span
                class="status-dot scale-75"
                :class="
                  status === 'connected'
                    ? 'status-dot-live'
                    : status === 'connecting' || status === 'reconnecting'
                      ? 'status-dot-live'
                      : ''
                "
                aria-hidden="true"
              />
              {{ t(`networkTools.mqtt.statuses.${status}`) }}
            </span>
            <span
              v-if="detail"
              class="min-w-0 truncate text-[10px] text-muted-foreground"
              :title="detail"
            >
              {{ detail }}
            </span>
            <div class="ml-auto flex items-center gap-1.5">
              <Button
                v-if="
                  status === 'connected' || status === 'connecting' || status === 'reconnecting'
                "
                variant="outline"
                size="sm"
                class="h-7 gap-1 border-warn/25 bg-warn/10 text-[10.5px] text-warn"
                :disabled="busy"
                @click="toggleConnection"
              >
                {{ t("networkTools.mqtt.stop") }}
              </Button>
              <Button
                v-else
                size="sm"
                class="h-7 gap-1 text-[10.5px]"
                :disabled="busy"
                @click="toggleConnection"
              >
                {{ busy ? t("tools.running") : t("networkTools.mqtt.connect") }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                @click="removeBroker"
              >
                <Trash2 class="size-3" />
                {{ t("networkTools.mqtt.deleteBroker") }}
              </Button>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.name")
              }}</Label>
              <Input
                v-model="name"
                class="h-8 rounded-lg border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.host")
              }}</Label>
              <Input
                v-model="host"
                class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.port")
              }}</Label>
              <Input
                v-model="port"
                class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.protocol")
              }}</Label>
              <FormSelect v-model="protocol" :options="protocolOptions" class="h-8" />
            </div>
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.clientId")
              }}</Label>
              <Input
                v-model="clientId"
                class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11px]"
                :placeholder="t('networkTools.mqtt.clientIdPlaceholder')"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.username")
              }}</Label>
              <Input
                v-model="username"
                class="h-8 rounded-lg border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.password")
              }}</Label>
              <Input
                v-model="password"
                type="password"
                class="h-8 rounded-lg border-hairline-strong bg-well text-[11px]"
                :placeholder="t('networkTools.mqtt.passwordHint')"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10.5px] text-muted-foreground">{{
                t("networkTools.mqtt.keepAlive")
              }}</Label>
              <Input
                v-model="keepAlive"
                class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11px]"
              />
            </div>
          </div>

          <div class="mt-2 flex flex-wrap items-center gap-4">
            <label class="flex items-center gap-2 text-[10.5px] text-muted-foreground">
              <Switch v-model="tls" />
              {{ t("networkTools.mqtt.tls") }}
            </label>
            <label class="flex items-center gap-2 text-[10.5px] text-muted-foreground">
              <Switch v-model="cleanSession" />
              {{ t("networkTools.mqtt.cleanSession") }}
            </label>
            <div v-if="tls" class="flex min-w-0 flex-1 items-center gap-2">
              <Input
                v-model="caPath"
                class="numeric h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[11px]"
                :placeholder="t('networkTools.mqtt.caPlaceholder')"
              />
              <Button
                variant="outline"
                size="sm"
                class="h-8 shrink-0 gap-1 border-hairline-strong bg-transparent text-[10.5px]"
                @click="browseCa"
              >
                <FolderOpen class="size-3" />
                {{ t("networkTools.http.browse") }}
              </Button>
            </div>
          </div>
        </div>

        <div class="grid gap-3 xl:grid-cols-2">
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="mb-2 text-[11px] font-medium">{{ t("networkTools.mqtt.subscriptions") }}</p>
            <div class="flex gap-1.5">
              <Input
                v-model="subscriptionTopic"
                class="numeric h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[11px]"
                placeholder="demo/#"
                @keydown.enter="addSubscription"
              />
              <div class="w-[92px] shrink-0">
                <FormSelect v-model="subscriptionQos" :options="qosOptions" />
              </div>
              <Button size="sm" class="h-8 shrink-0 gap-1 text-[11px]" @click="addSubscription">
                <Plus class="size-3" />
                {{ t("networkTools.mqtt.subscribe") }}
              </Button>
            </div>
            <div class="mt-2 flex flex-wrap gap-1.5">
              <span
                v-for="subscription in brokerSubscriptions"
                :key="subscription.topic"
                class="numeric flex items-center gap-1.5 rounded-full border border-hairline-strong bg-well px-2 py-1 text-[10.5px] text-muted-foreground"
              >
                {{ subscription.topic }}
                <span class="text-[9px] text-muted-foreground/70">Q{{ subscription.qos }}</span>
                <button
                  type="button"
                  class="text-muted-foreground transition-colors hover:text-error"
                  :aria-label="t('networkTools.mqtt.unsubscribe')"
                  @click="removeSubscription(subscription.topic)"
                >
                  <X class="size-3" />
                </button>
              </span>
              <span v-if="!brokerSubscriptions.length" class="text-[10px] text-muted-foreground">
                {{ t("networkTools.mqtt.noSubscriptions") }}
              </span>
            </div>
          </div>

          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="mb-2 text-[11px] font-medium">{{ t("networkTools.mqtt.publishTitle") }}</p>
            <div class="flex gap-1.5">
              <Input
                v-model="publishTopic"
                class="numeric h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[11px]"
                placeholder="demo/command"
                @keydown.enter="publish"
              />
              <div class="w-[88px] shrink-0">
                <FormSelect v-model="publishQos" :options="qosOptions" />
              </div>
              <div class="w-[88px] shrink-0">
                <FormSelect v-model="publishMode" :options="modeOptions" />
              </div>
            </div>
            <textarea
              v-model="publishPayload"
              rows="3"
              class="numeric scrollbar-thin mt-1.5 w-full resize-y rounded-lg border border-hairline-strong bg-well px-2.5 py-1.5 text-[11px] text-foreground outline-none focus:border-signal/50"
              :placeholder="t('networkTools.mqtt.payloadPlaceholder')"
            />
            <div class="mt-1.5 flex items-center gap-3">
              <label class="flex items-center gap-1.5 text-[10.5px] text-muted-foreground">
                <Switch v-model="publishRetain" />
                Retain
              </label>
              <span class="text-[10px] text-muted-foreground">{{
                t("networkTools.mqtt.publishHint")
              }}</span>
              <Button size="sm" class="ml-auto h-8 gap-1 text-[11px]" @click="publish">
                <Send class="size-3" />
                {{ t("networkTools.mqtt.publish") }}
              </Button>
            </div>
          </div>
        </div>

        <div class="rounded-xl border border-hairline bg-well-soft">
          <div class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">{{ t("networkTools.mqtt.messages") }}</span>
            <span class="numeric text-[10px] text-muted-foreground">
              {{ entries.length }}
            </span>
            <div class="ml-auto flex items-center gap-1">
              <button
                type="button"
                class="rounded px-1.5 py-0.5 text-[10px] transition-colors"
                :class="
                  logMode === 'text'
                    ? 'bg-overlay-2 text-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                "
                @click="logMode = 'text'"
              >
                {{ t("networkTools.mqtt.text") }}
              </button>
              <button
                type="button"
                class="rounded px-1.5 py-0.5 text-[10px] transition-colors"
                :class="
                  logMode === 'hex'
                    ? 'bg-overlay-2 text-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                "
                @click="logMode = 'hex'"
              >
                {{ t("networkTools.mqtt.hex") }}
              </button>
              <Button
                variant="ghost"
                size="sm"
                class="h-6 text-[10px] text-muted-foreground"
                :disabled="!entries.length"
                @click="mqtt.clearMessages(active.id)"
              >
                {{ t("common.clear") }}
              </Button>
            </div>
          </div>
          <div
            ref="messageBox"
            class="scrollbar-thin max-h-[320px] min-h-[120px] overflow-auto px-2 py-1.5"
          >
            <div
              v-for="entry in visibleEntries"
              :key="entry.id"
              class="mb-1 rounded-lg border border-hairline px-2 py-1.5"
              :class="entry.direction === 'in' ? 'bg-well' : 'bg-rx/[0.04]'"
            >
              <div class="flex items-center gap-1.5 text-[10px] text-muted-foreground">
                <ArrowDownLeft v-if="entry.direction === 'in'" class="size-3 text-signal" />
                <ArrowUpRight v-else class="size-3 text-rx" />
                <span class="numeric text-foreground">{{ entry.topic }}</span>
                <span class="rounded border border-hairline px-1 py-0.5 text-[9px]"
                  >Q{{ entry.qos }}</span
                >
                <span
                  v-if="entry.retain"
                  class="rounded border border-warn/25 px-1 py-0.5 text-[9px] text-warn"
                >
                  retain
                </span>
                <span class="ml-auto numeric">{{ formatTime(entry.timestampMs) }}</span>
                <button
                  type="button"
                  class="flex items-center gap-1 text-muted-foreground transition-colors hover:text-foreground"
                  :aria-label="t('tools.copy')"
                  @click="copy(entry.id, entryPayload(entry))"
                >
                  <Check v-if="isCopied(entry.id)" class="size-3 text-signal" />
                  <Copy v-else class="size-3" />
                  <span v-if="isCopied(entry.id)" class="text-[9px] text-signal">{{
                    t("common.copiedShort")
                  }}</span>
                </button>
              </div>
              <pre
                class="scrollbar-thin numeric mt-1 max-h-[72px] overflow-auto whitespace-pre-wrap break-all text-[10.5px] leading-relaxed text-foreground"
                >{{ entryPayload(entry) || "—" }}</pre>
              <div class="mt-0.5 flex items-center gap-2 text-[9px] text-muted-foreground">
                <span>{{ entry.payloadBytes }} B</span>
              </div>
            </div>
            <p
              v-if="!visibleEntries.length"
              class="py-8 text-center text-[11px] text-muted-foreground"
            >
              {{ t("networkTools.mqtt.noMessages") }}
            </p>
          </div>
        </div>
      </div>

      <div
        v-else
        class="flex flex-1 flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-hairline text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.mqtt.noBrokers") }}
        <Button size="sm" class="h-8 gap-1.5 text-[11px]" @click="addBroker">
          <Plus class="size-3" />
          {{ t("networkTools.mqtt.addBroker") }}
        </Button>
      </div>
    </div>
  </ToolPanelFrame>
</template>
