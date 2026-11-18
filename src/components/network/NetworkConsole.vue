<script setup lang="ts">
import {
  ArrowDownToLine,
  CircleAlert,
  Download,
  Eraser,
  LoaderCircle,
  Network,
  Play,
  Send,
  Square,
} from "@lucide/vue";
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatTime } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { formatDebugBytes, rawByteLength } from "@/lib/raw-send";
import type { NetworkSession } from "@/stores/network";
import { useNetworkStore } from "@/stores/network";
import type { NetworkMode, RawSendMode } from "@/types/models";

const props = defineProps<{ session: NetworkSession }>();
const network = useNetworkStore();
const { t } = useI18n();

const modeKeys: Record<NetworkMode, string> = {
  "tcp-server": "tcpServer",
  "tcp-client": "tcpClient",
  "udp-server": "udpServer",
  "udp-client": "udpClient",
  "tcp-relay": "tcpRelay",
};

const isServer = computed(
  () => props.session.mode === "tcp-server" || props.session.mode === "udp-server",
);
const isClient = computed(
  () => props.session.mode === "tcp-client" || props.session.mode === "udp-client",
);
const isRelay = computed(() => props.session.mode === "tcp-relay");
const modeLabel = computed(() => t(`network.modes.${modeKeys[props.session.mode]}`));
const modeHint = computed(() => {
  if (isServer.value) return t("network.serverHint");
  if (isRelay.value) return t("network.relayHint");
  return t("network.clientHint");
});
const locked = computed(() => props.session.active || props.session.busy);

const name = computed({
  get: () => props.session.name,
  set: (value: string) => network.updateConfig(props.session.id, { name: value }),
});
const localHost = computed({
  get: () => props.session.localHost,
  set: (value: string) => network.updateConfig(props.session.id, { localHost: value }),
});
const localPort = computed({
  get: () => props.session.localPort,
  set: (value: number) => network.updateConfig(props.session.id, { localPort: value }),
});
const remoteHost = computed({
  get: () => props.session.remoteHost,
  set: (value: string) => network.updateConfig(props.session.id, { remoteHost: value }),
});
const remotePort = computed({
  get: () => props.session.remotePort,
  set: (value: number) => network.updateConfig(props.session.id, { remotePort: value }),
});

const displayMode = ref<"ascii" | "hex">("ascii");
const sendMode = ref<RawSendMode>("ascii");
const formats: Array<"ascii" | "hex"> = ["ascii", "hex"];
const payload = ref("");
const appendCr = ref(false);
const appendLf = ref(false);
const sending = ref(false);
const follow = ref(true);
const listEl = ref<HTMLElement | null>(null);

const preview = computed<{ bytes: number | null; valid: boolean }>(() => {
  if (!payload.value.length && !appendCr.value && !appendLf.value) {
    return { bytes: 0, valid: true };
  }
  const bytes = rawByteLength(payload.value, sendMode.value, {
    appendCr: appendCr.value,
    appendLf: appendLf.value,
  });
  return { bytes, valid: bytes !== null };
});
const canSend = computed(
  () =>
    props.session.active &&
    !sending.value &&
    preview.value.valid &&
    preview.value.bytes !== null &&
    preview.value.bytes > 0,
);
const placeholder = computed(() =>
  props.session.active
    ? sendMode.value === "ascii"
      ? t("network.inputPlaceholder")
      : t("network.inputPlaceholderHex")
    : t("network.configLocked"),
);
const rxBytes = computed(() => network.rxBytes(props.session));
const txBytes = computed(() => network.txBytes(props.session));

const ERROR_CODES = new Set([
  "listenPortRange",
  "remoteHostRequired",
  "remotePortRange",
  "notRunning",
  "startFailed",
  "stopFailed",
]);

function errorText(message: string): string {
  if (!message) return "";
  if (message === "networkNotRunning") return t("network.notRunning");
  if (ERROR_CODES.has(message)) return t(`network.errors.${message}`);
  return message;
}

async function scrollToEnd(): Promise<void> {
  await nextTick();
  if (listEl.value) listEl.value.scrollTop = listEl.value.scrollHeight;
}

function onScroll(): void {
  const element = listEl.value;
  if (!element) return;
  follow.value = element.scrollHeight - element.scrollTop - element.clientHeight < 24;
}

async function toggleFollow(): Promise<void> {
  follow.value = true;
  await scrollToEnd();
}

watch(
  () => props.session.entries.length,
  () => {
    if (follow.value) void scrollToEnd();
  },
);

watch(
  () => props.session.id,
  async () => {
    payload.value = "";
    follow.value = true;
    network.resetError(props.session.id);
    await scrollToEnd();
  },
);

async function start(): Promise<void> {
  try {
    await network.start(props.session.id);
    await scrollToEnd();
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    toast.error(t("network.startFailed"), { description: errorText(message) });
  }
}

async function stop(): Promise<void> {
  try {
    await network.stop(props.session.id);
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    toast.error(t("network.stopFailed"), { description: errorText(message) });
  }
}

async function send(): Promise<void> {
  if (!canSend.value || sending.value) return;
  sending.value = true;
  try {
    const count = await network.send(props.session.id, {
      data: payload.value,
      mode: sendMode.value,
      appendCr: appendCr.value,
      appendLf: appendLf.value,
    });
    toast.success(t("network.sent", { count }));
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    toast.error(t("network.sendFailed"), { description: errorText(message) });
  } finally {
    sending.value = false;
  }
}

function csvCell(value: unknown): string {
  return `"${String(value ?? "").replace(/"/g, '""')}"`;
}

async function exportLogs(): Promise<void> {
  if (!props.session.entries.length) {
    toast.info(t("network.exportEmpty"));
    return;
  }
  const header = ["time", "direction", "connection", "bytes", "ascii", "hex"];
  const lines = [header.map(csvCell).join(",")];
  for (const entry of props.session.entries) {
    lines.push(
      [
        new Date(entry.timestamp).toISOString(),
        entry.direction.toUpperCase(),
        entry.connection,
        entry.data.length,
        formatDebugBytes(entry.data, "ascii"),
        formatDebugBytes(entry.data, "hex"),
      ]
        .map(csvCell)
        .join(","),
    );
  }
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const scope = props.session.name.replace(/[\\/:*?"<>|\s]+/g, "_");
  try {
    const saved = await ipc.saveTextFile(
      `modbus-network-${scope}-${stamp}.csv`,
      `${String.fromCharCode(0xfeff)}${lines.join("\r\n")}`,
    );
    if (saved) {
      toast.success(t("network.exported", { count: props.session.entries.length }));
    }
  } catch (cause: unknown) {
    toast.error(t("network.exportFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  }
}
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <header class="flex h-[54px] shrink-0 items-center gap-3 border-b border-hairline px-4">
      <span
        class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-rx/25 bg-rx/10 text-rx"
      >
        <Network class="size-4" />
      </span>
      <div class="min-w-0 flex-1">
        <div class="flex min-w-0 items-center gap-2">
          <span class="truncate text-[13px] font-semibold">{{ session.name }}</span>
          <span class="rounded border border-rx/20 bg-rx/10 px-1.5 py-0.5 text-[10.5px] text-rx">
            {{ modeLabel }}
          </span>
          <span
            class="inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10.5px]"
            :class="
              session.active
                ? 'border-signal/25 bg-signal/10 text-signal'
                : 'border-hairline bg-overlay-1 text-muted-foreground'
            "
          >
            <span
              class="status-dot scale-75"
              :class="session.active ? 'status-dot-live text-signal' : 'opacity-40'"
            />
            {{ session.active ? t("network.running") : t("network.inactive") }}
          </span>
        </div>
        <div class="mt-0.5 flex min-w-0 items-center gap-2 text-[10.5px] text-muted-foreground">
          <span class="numeric truncate">{{ session.endpoint ?? t("network.noEndpoint") }}</span>
          <span class="text-foreground/15">/</span>
          <span>{{ t("network.clients", { count: session.clients }) }}</span>
        </div>
      </div>
      <Button
        v-if="!session.active"
        size="sm"
        class="h-8 shrink-0 gap-1.5 border-signal/30 bg-signal/15 text-[11.5px] text-signal hover:bg-signal/20"
        :disabled="session.busy"
        @click="start"
      >
        <LoaderCircle v-if="session.busy" class="size-3.5 animate-spin" />
        <Play v-else class="size-3.5" />
        {{ session.busy ? t("network.starting") : t("network.start") }}
      </Button>
      <Button
        v-else
        variant="outline"
        size="sm"
        class="h-8 shrink-0 gap-1.5 border-error/25 bg-error/10 text-[11.5px] text-error hover:bg-error/15"
        :disabled="session.busy"
        @click="stop"
      >
        <LoaderCircle v-if="session.busy" class="size-3.5 animate-spin" />
        <Square v-else class="size-3.5" />
        {{ t("network.stop") }}
      </Button>
    </header>

    <div class="shrink-0 border-b border-hairline bg-overlay-1/35 px-4 py-3">
      <div class="mb-2 flex items-center gap-2">
        <span class="text-[10px] font-medium uppercase tracking-wide text-muted-foreground/70">
          {{ t("network.config") }}
        </span>
        <span class="text-[10.5px] text-muted-foreground/80">
          {{ session.active ? t("network.configLocked") : t("network.configHint") }}
        </span>
      </div>
      <div class="grid gap-2 md:grid-cols-4">
        <div class="space-y-1">
          <Label class="text-[10px] text-muted-foreground">{{ t("network.sessionName") }}</Label>
          <Input
            v-model="name"
            :disabled="locked"
            class="h-8 rounded-lg border-hairline bg-well text-[11.5px]"
          />
        </div>
        <template v-if="isServer">
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.listenHost") }}</Label>
            <Input
              v-model="localHost"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.listenPort") }}</Label>
            <Input
              v-model.number="localPort"
              type="number"
              min="1"
              max="65535"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
        </template>
        <template v-else-if="isClient">
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.targetHost") }}</Label>
            <Input
              v-model="remoteHost"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.targetPort") }}</Label>
            <Input
              v-model.number="remotePort"
              type="number"
              min="1"
              max="65535"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
        </template>
        <template v-else>
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.listenHost") }}</Label>
            <Input
              v-model="localHost"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.listenPort") }}</Label>
            <Input
              v-model.number="localPort"
              type="number"
              min="1"
              max="65535"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.upstreamHost") }}</Label>
            <Input
              v-model="remoteHost"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ t("network.upstreamPort") }}</Label>
            <Input
              v-model.number="remotePort"
              type="number"
              min="1"
              max="65535"
              :disabled="locked"
              class="numeric h-8 rounded-lg border-hairline bg-well text-[11.5px]"
            />
          </div>
        </template>
      </div>
      <p class="mt-1.5 text-[10px] leading-relaxed text-muted-foreground/70">{{ modeHint }}</p>
    </div>

    <div
      v-if="session.error"
      class="flex shrink-0 items-start gap-2 border-b border-error/25 bg-error/10 px-4 py-2 text-[11px] leading-relaxed text-error"
    >
      <CircleAlert class="mt-0.5 size-3.5 shrink-0" />
      <span>{{ t("network.linkError") }}: {{ errorText(session.error) }}</span>
    </div>

    <div
      class="flex shrink-0 flex-wrap items-center gap-2 border-b border-hairline bg-overlay-1/45 px-4 py-2"
    >
      <span class="text-[10.5px] font-medium text-foreground/90">{{ t("network.log") }}</span>
      <span class="text-foreground/15">/</span>
      <span class="numeric text-[10.5px] text-muted-foreground">
        <span class="text-rx">RX</span> {{ rxBytes }} B
      </span>
      <span class="text-foreground/15">/</span>
      <span class="numeric text-[10.5px] text-muted-foreground">
        <span class="text-signal">TX</span> {{ txBytes }} B
      </span>
      <span class="text-foreground/15">/</span>
      <span class="text-[10.5px] text-muted-foreground">
        {{ t("network.entries", { count: session.entries.length }) }}
      </span>

      <div class="ml-auto flex items-center gap-1.5">
        <span class="text-[10.5px] text-muted-foreground">{{ t("network.display") }}</span>
        <div class="flex rounded-lg border border-hairline bg-well p-0.5" role="group">
          <button
            v-for="option in formats"
            :key="option"
            type="button"
            class="numeric rounded-md px-1.5 py-0.5 text-[10px] uppercase transition-colors"
            :class="
              displayMode === option
                ? option === 'hex'
                  ? 'bg-rx/15 text-rx'
                  : 'bg-signal/15 text-signal'
                : 'text-muted-foreground hover:text-foreground'
            "
            :aria-pressed="displayMode === option"
            @click="displayMode = option"
          >
            {{ option }}
          </button>
        </div>
        <button
          type="button"
          class="flex items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] transition-colors"
          :class="
            follow ? 'bg-overlay-2 text-foreground' : 'text-muted-foreground hover:text-foreground'
          "
          :title="t('network.follow')"
          @click="toggleFollow"
        >
          <ArrowDownToLine class="size-3" />
          <span class="hidden sm:inline">{{ t("network.follow") }}</span>
        </button>
        <button
          type="button"
          class="flex items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] text-muted-foreground transition-colors hover:text-foreground"
          :title="t('network.export')"
          @click="exportLogs"
        >
          <Download class="size-3" />
          <span class="hidden sm:inline">{{ t("network.export") }}</span>
        </button>
        <button
          type="button"
          class="flex items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] text-muted-foreground transition-colors hover:text-foreground"
          :title="t('network.clear')"
          @click="network.clear(session.id)"
        >
          <Eraser class="size-3" />
          <span class="hidden sm:inline">{{ t("network.clear") }}</span>
        </button>
      </div>
    </div>

    <div
      ref="listEl"
      class="scrollbar-thin min-h-0 flex-1 overflow-auto bg-well/45 px-4 py-3"
      @scroll="onScroll"
    >
      <div
        v-if="!session.entries.length"
        class="flex h-full min-h-[220px] flex-col items-center justify-center gap-2 text-center"
      >
        <span class="status-dot status-dot-live text-rx" />
        <p class="text-[12px] text-muted-foreground">{{ t("network.empty") }}</p>
        <p class="max-w-sm text-[10.5px] leading-relaxed text-muted-foreground/80">
          {{ t("network.emptyHint") }}
        </p>
      </div>

      <div v-else class="space-y-0.5 font-mono">
        <div
          v-for="entry in session.entries"
          :key="entry.id"
          class="flex items-start gap-2 rounded px-1 py-0.5 text-[11.5px] hover:bg-overlay-1"
        >
          <span class="numeric mt-[2px] shrink-0 text-[10px] text-muted-foreground/70">
            {{ formatTime(entry.timestamp) }}
          </span>
          <span
            class="mt-[1px] inline-flex h-[15px] shrink-0 items-center justify-center rounded border px-1 text-[9.5px] font-medium"
            :class="
              entry.direction === 'tx'
                ? 'border-tx/25 bg-tx/10 text-tx'
                : 'border-rx/25 bg-rx/10 text-rx'
            "
          >
            {{ entry.direction.toUpperCase() }}
          </span>
          <span
            class="numeric mt-[2px] w-10 shrink-0 text-right text-[9.5px] text-muted-foreground/60"
          >
            {{ entry.data.length }}B
          </span>
          <span
            class="mt-[2px] max-w-[120px] shrink-0 truncate font-sans text-[9.5px] text-muted-foreground/70"
            :title="entry.connection"
          >
            {{ entry.connection }}
          </span>
          <pre
            class="min-w-0 flex-1 whitespace-pre-wrap break-all font-mono text-[11.5px] leading-relaxed"
            :class="entry.direction === 'tx' ? 'text-signal/90' : 'text-foreground/90'"
            >{{ formatDebugBytes(entry.data, displayMode) }}</pre>
        </div>
      </div>
    </div>

    <div class="shrink-0 border-t border-hairline bg-overlay-1/40 px-4 py-3">
      <div class="flex flex-wrap items-center gap-2">
        <span class="numeric shrink-0 text-[13px] text-rx/70">›</span>
        <Input
          v-model="payload"
          :placeholder="placeholder"
          :disabled="!session.active || sending"
          spellcheck="false"
          autocomplete="off"
          :aria-label="t('network.inputLabel')"
          class="numeric h-8 min-w-[220px] flex-1 rounded-lg border-hairline bg-well px-2.5 text-[11.5px] placeholder:font-sans placeholder:text-muted-foreground/80 focus-visible:border-rx/40 focus-visible:ring-rx/10"
          :class="preview.valid ? '' : 'border-error/45 focus-visible:border-error/60'"
          @keydown.enter.prevent="send"
        />

        <span class="numeric hidden shrink-0 text-[10.5px] text-muted-foreground md:inline">
          {{ preview.bytes === null ? "ERR" : `${preview.bytes} B` }}
        </span>

        <div
          class="flex shrink-0 items-center gap-2 rounded-lg border border-hairline bg-well px-2 py-1"
          role="group"
          :aria-label="t('network.suffix')"
        >
          <span class="text-[10px] text-muted-foreground">{{ t("network.suffix") }}</span>
          <label class="flex cursor-pointer items-center gap-1 text-[10px] text-muted-foreground">
            <input
              v-model="appendCr"
              type="checkbox"
              :disabled="!session.active || sending"
              class="size-3 accent-[var(--signal)]"
            />
            <span class="numeric">\r</span>
          </label>
          <label class="flex cursor-pointer items-center gap-1 text-[10px] text-muted-foreground">
            <input
              v-model="appendLf"
              type="checkbox"
              :disabled="!session.active || sending"
              class="size-3 accent-[var(--signal)]"
            />
            <span class="numeric">\n</span>
          </label>
        </div>

        <div class="flex shrink-0 rounded-lg border border-hairline bg-well p-0.5" role="group">
          <button
            v-for="option in formats"
            :key="option"
            type="button"
            :disabled="!session.active || sending"
            class="numeric rounded-md px-1.5 py-0.5 text-[10px] uppercase transition-colors disabled:opacity-50"
            :class="
              sendMode === option
                ? option === 'hex'
                  ? 'bg-rx/15 text-rx'
                  : 'bg-signal/15 text-signal'
                : 'text-muted-foreground hover:text-foreground'
            "
            :aria-pressed="sendMode === option"
            @click="sendMode = option"
          >
            {{ option }}
          </button>
        </div>

        <Button
          variant="outline"
          size="sm"
          class="h-8 shrink-0 gap-1.5 border-rx/25 bg-rx/10 px-3 text-[11.5px] text-rx hover:bg-rx/15 disabled:border-hairline disabled:bg-overlay-1 disabled:text-muted-foreground"
          :disabled="!canSend"
          @click="send"
        >
          <LoaderCircle v-if="sending" class="size-3.5 animate-spin" />
          <Send v-else class="size-3.5" />
          {{ sending ? t("network.sending") : t("network.send") }}
        </Button>
      </div>

      <p v-if="!preview.valid" class="mt-1.5 text-[10.5px] leading-relaxed text-error">
        {{ t("network.invalidPayload") }}
      </p>
    </div>
  </div>
</template>
