<script setup lang="ts">
import {
  ArrowDownToLine,
  ArrowLeftRight,
  CircleAlert,
  Download,
  Eraser,
  LoaderCircle,
  Network,
  Play,
  Radio,
  Send,
  Server,
  Square,
} from "@lucide/vue";
import type { Component } from "vue";
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { formatTime } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { formatDebugBytes, rawByteLength } from "@/lib/raw-send";
import { useNetworkStore } from "@/stores/network";
import type { NetworkMode, RawSendMode } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const network = useNetworkStore();
const { t } = useI18n();

interface ModeOption {
  value: NetworkMode;
  label: string;
  hint: string;
  icon: Component;
}

const modeOptions = computed<ModeOption[]>(() => [
  {
    value: "tcp-server",
    label: t("network.modes.tcpServer"),
    hint: t("network.modeHints.tcpServer"),
    icon: Server,
  },
  {
    value: "tcp-client",
    label: t("network.modes.tcpClient"),
    hint: t("network.modeHints.tcpClient"),
    icon: ArrowLeftRight,
  },
  {
    value: "udp-server",
    label: t("network.modes.udpServer"),
    hint: t("network.modeHints.udpServer"),
    icon: Radio,
  },
  {
    value: "udp-client",
    label: t("network.modes.udpClient"),
    hint: t("network.modeHints.udpClient"),
    icon: Send,
  },
  {
    value: "tcp-relay",
    label: t("network.modes.tcpRelay"),
    hint: t("network.modeHints.tcpRelay"),
    icon: Network,
  },
]);

const currentMode = computed(
  () => modeOptions.value.find((option) => option.value === network.mode) ?? modeOptions.value[0],
);

const displayMode = ref<"ascii" | "hex">("ascii");
const sendMode = ref<RawSendMode>("ascii");
const formats: Array<"ascii" | "hex"> = ["ascii", "hex"];
const payload = ref("");
const appendCr = ref(false);
const appendLf = ref(false);
const sending = ref(false);
const busy = ref(false);
const formError = ref("");
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
    network.active &&
    !sending.value &&
    preview.value.valid &&
    preview.value.bytes !== null &&
    preview.value.bytes > 0,
);
const placeholder = computed(() =>
  network.active
    ? sendMode.value === "ascii"
      ? t("network.inputPlaceholder")
      : t("network.inputPlaceholderHex")
    : t("network.configLocked"),
);

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
  if (ERROR_CODES.has(message)) {
    const key = message === "notRunning" ? "notRunning" : message;
    return t(`network.errors.${key}`);
  }
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
  () => network.entries.length,
  () => {
    if (follow.value) void scrollToEnd();
  },
);

watch(open, async (isOpen) => {
  if (!isOpen) return;
  formError.value = "";
  network.resetError();
  await scrollToEnd();
});

async function start(): Promise<void> {
  if (busy.value || network.active) return;
  busy.value = true;
  formError.value = "";
  try {
    await network.start();
    formError.value = "";
    await scrollToEnd();
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    formError.value = errorText(message);
    toast.error(t("network.startFailed"), {
      description: formError.value,
    });
  } finally {
    busy.value = false;
  }
}

async function stop(): Promise<void> {
  if (busy.value || !network.active) return;
  busy.value = true;
  try {
    await network.stop();
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    toast.error(t("network.stopFailed"), { description: errorText(message) });
  } finally {
    busy.value = false;
  }
}

async function send(): Promise<void> {
  if (!canSend.value || sending.value) return;
  sending.value = true;
  try {
    const count = await network.send({
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
  if (!network.entries.length) {
    toast.info(t("network.exportEmpty"));
    return;
  }
  const header = ["time", "direction", "connection", "bytes", "ascii", "hex"];
  const lines = [header.map(csvCell).join(",")];
  for (const entry of network.entries) {
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
  try {
    const saved = await ipc.saveTextFile(
      `modbus-network-${stamp}.csv`,
      `${String.fromCharCode(0xfeff)}${lines.join("\r\n")}`,
    );
    if (saved) {
      toast.success(t("network.exported", { count: network.entries.length }));
    }
  } catch (cause: unknown) {
    toast.error(t("network.exportFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="flex h-[min(800px,90vh)] flex-col gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/30 backdrop-blur-2xl sm:max-w-[1040px]"
    >
      <DialogHeader class="shrink-0 border-b border-hairline px-5 py-4">
        <div class="flex items-start gap-3 pr-7">
          <span
            class="flex size-9 shrink-0 items-center justify-center rounded-xl border border-rx/25 bg-rx/10 text-rx"
          >
            <Network class="size-4" />
          </span>
          <div class="min-w-0 flex-1">
            <DialogTitle class="text-[14px]">{{ t("network.title") }}</DialogTitle>
            <DialogDescription class="mt-0.5 truncate text-[11px] text-muted-foreground">
              {{ t("network.subtitle") }} · {{ currentMode?.hint }}
            </DialogDescription>
          </div>
          <span
            class="inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2 py-1 text-[10.5px] font-medium"
            :class="
              network.active
                ? 'border-signal/25 bg-signal/10 text-signal'
                : 'border-hairline bg-overlay-1 text-muted-foreground'
            "
          >
            <span
              class="status-dot scale-75"
              :class="network.active ? 'status-dot-live text-signal' : 'opacity-40'"
            />
            {{ network.active ? t("network.running") : t("network.inactive") }}
          </span>
        </div>
      </DialogHeader>

      <div class="flex min-h-0 flex-1">
        <aside
          class="scrollbar-thin flex w-[310px] shrink-0 flex-col overflow-y-auto border-r border-hairline bg-overlay-1/35 p-4"
        >
          <div class="mb-3 flex items-center justify-between">
            <h3 class="text-[11px] font-medium tracking-wide text-muted-foreground">
              {{ t("network.config") }}
            </h3>
            <span class="numeric text-[9.5px] text-muted-foreground/60">
              {{ t("network.mode") }}
            </span>
          </div>

          <div class="grid grid-cols-2 gap-1.5" role="group" :aria-label="t('network.mode')">
            <button
              v-for="option in modeOptions"
              :key="option.value"
              type="button"
              class="group flex items-center gap-2 rounded-lg border px-2.5 py-2 text-left transition-colors"
              :class="[
                option.value === 'tcp-relay' ? 'col-span-2' : '',
                network.mode === option.value
                  ? 'border-rx/35 bg-rx/10 text-rx'
                  : 'border-hairline bg-well text-muted-foreground hover:border-hairline-strong hover:text-foreground',
              ]"
              :disabled="network.active"
              :aria-pressed="network.mode === option.value"
              @click="network.mode = option.value"
            >
              <component :is="option.icon" class="size-3.5 shrink-0" />
              <span class="truncate text-[11px] font-medium">{{ option.label }}</span>
            </button>
          </div>

          <p class="mt-2 min-h-[32px] text-[10.5px] leading-relaxed text-muted-foreground/80">
            {{ currentMode?.hint }}
          </p>

          <div class="my-3 h-px bg-hairline" />

          <div class="space-y-2.5">
            <label class="block">
              <span class="mb-1 block text-[10px] font-medium text-muted-foreground">
                {{ t("network.localHost") }}
              </span>
              <Input
                v-model="network.localHost"
                :placeholder="t('network.localHostPlaceholder')"
                :disabled="network.active"
                spellcheck="false"
                autocomplete="off"
                class="numeric h-8 rounded-lg border-hairline bg-well px-2.5 text-[11.5px]"
              />
            </label>

            <label class="block">
              <span class="mb-1 block text-[10px] font-medium text-muted-foreground">
                {{ t("network.localPort") }}
              </span>
              <Input
                v-model.number="network.localPort"
                type="number"
                min="0"
                max="65535"
                :disabled="network.active"
                class="numeric h-8 rounded-lg border-hairline bg-well px-2.5 text-[11.5px]"
              />
              <span class="mt-1 block text-[9.5px] text-muted-foreground/70">
                {{ t("network.localPortHint") }}
              </span>
            </label>

            <template v-if="network.isClient || network.isRelay">
              <label class="block">
                <span class="mb-1 block text-[10px] font-medium text-muted-foreground">
                  {{ t("network.remoteHost") }}
                </span>
                <Input
                  v-model="network.remoteHost"
                  :placeholder="t('network.remoteHostPlaceholder')"
                  :disabled="network.active"
                  spellcheck="false"
                  autocomplete="off"
                  class="numeric h-8 rounded-lg border-hairline bg-well px-2.5 text-[11.5px]"
                />
              </label>

              <label class="block">
                <span class="mb-1 block text-[10px] font-medium text-muted-foreground">
                  {{ t("network.remotePort") }}
                </span>
                <Input
                  v-model.number="network.remotePort"
                  type="number"
                  min="1"
                  max="65535"
                  :disabled="network.active"
                  class="numeric h-8 rounded-lg border-hairline bg-well px-2.5 text-[11.5px]"
                />
              </label>
            </template>
          </div>

          <div
            v-if="formError"
            class="mt-3 flex items-start gap-1.5 rounded-lg border border-error/25 bg-error/10 px-2.5 py-2 text-[10.5px] leading-relaxed text-error"
          >
            <CircleAlert class="mt-0.5 size-3 shrink-0" />
            <span>{{ formError }}</span>
          </div>

          <div class="mt-3 rounded-xl border border-hairline bg-well/70 p-2.5">
            <p class="text-[9.5px] uppercase tracking-wide text-muted-foreground/60">
              {{ t("network.endpoint") }}
            </p>
            <p class="numeric mt-1 break-all text-[11px] text-foreground/90">
              {{ network.endpoint ?? t("network.noEndpoint") }}
            </p>
            <div class="mt-2 flex items-center gap-2 text-[10px] text-muted-foreground">
              <span
                class="status-dot scale-75"
                :class="network.active ? 'text-signal' : 'opacity-40'"
              />
              <span>{{ network.active ? t("network.running") : t("network.inactive") }}</span>
              <span class="text-foreground/15">/</span>
              <span>{{ t("network.clients", { count: network.clients }) }}</span>
            </div>
          </div>

          <Button
            v-if="!network.active"
            size="sm"
            class="mt-3 h-9 w-full gap-1.5 border-signal/30 bg-signal/15 text-[11.5px] text-signal hover:bg-signal/20"
            :disabled="busy"
            @click="start"
          >
            <LoaderCircle v-if="busy" class="size-3.5 animate-spin" />
            <Play v-else class="size-3.5" />
            {{ busy ? t("network.starting") : t("network.start") }}
          </Button>
          <Button
            v-else
            variant="outline"
            size="sm"
            class="mt-3 h-9 w-full gap-1.5 border-error/25 bg-error/10 text-[11.5px] text-error hover:bg-error/15"
            :disabled="busy"
            @click="stop"
          >
            <LoaderCircle v-if="busy" class="size-3.5 animate-spin" />
            <Square v-else class="size-3.5" />
            {{ t("network.stop") }}
          </Button>

          <p class="mt-2 text-[9.5px] leading-relaxed text-muted-foreground/70">
            {{ network.active ? t("network.configLocked") : t("network.note") }}
          </p>
        </aside>

        <section class="flex min-w-0 flex-1 flex-col">
          <div
            class="flex shrink-0 flex-wrap items-center gap-2 border-b border-hairline bg-overlay-1/45 px-4 py-2"
          >
            <span class="text-[10.5px] font-medium text-foreground/90">
              {{ t("network.log") }}
            </span>
            <span class="text-foreground/15">/</span>
            <span class="numeric text-[10.5px] text-muted-foreground">
              <span class="text-rx">RX</span> {{ network.rxBytes }} B
            </span>
            <span class="text-foreground/15">/</span>
            <span class="numeric text-[10.5px] text-muted-foreground">
              <span class="text-signal">TX</span> {{ network.txBytes }} B
            </span>
            <span class="text-foreground/15">/</span>
            <span class="text-[10.5px] text-muted-foreground">
              {{ t("network.entries", { count: network.entries.length }) }}
            </span>

            <div class="ml-auto flex items-center gap-1.5">
              <span class="text-[10.5px] text-muted-foreground">{{ t("network.display") }}</span>
              <div
                class="flex rounded-lg border border-hairline bg-well p-0.5"
                role="group"
                :aria-label="t('network.display')"
              >
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
                  follow
                    ? 'bg-overlay-2 text-foreground'
                    : 'text-muted-foreground hover:text-foreground'
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
                @click="network.clear()"
              >
                <Eraser class="size-3" />
                <span class="hidden sm:inline">{{ t("network.clear") }}</span>
              </button>
            </div>
          </div>

          <div
            v-if="network.error"
            class="flex shrink-0 items-start gap-2 border-b border-error/25 bg-error/10 px-4 py-2 text-[11px] leading-relaxed text-error"
          >
            <CircleAlert class="mt-0.5 size-3.5 shrink-0" />
            <span>{{ t("network.linkError") }}: {{ network.error }}</span>
          </div>

          <div
            ref="listEl"
            class="scrollbar-thin min-h-0 flex-1 overflow-auto bg-well/45 px-4 py-3"
            @scroll="onScroll"
          >
            <div
              v-if="!network.entries.length"
              class="flex h-full min-h-[240px] flex-col items-center justify-center gap-2 text-center"
            >
              <span class="status-dot status-dot-live text-rx" />
              <p class="text-[12px] text-muted-foreground">{{ t("network.empty") }}</p>
              <p class="max-w-sm text-[10.5px] leading-relaxed text-muted-foreground/80">
                {{ t("network.emptyHint") }}
              </p>
            </div>

            <div v-else class="space-y-0.5 font-mono">
              <div
                v-for="entry in network.entries"
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
                :disabled="!network.active || sending"
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
                <label
                  class="flex cursor-pointer items-center gap-1 text-[10px] text-muted-foreground"
                >
                  <input
                    v-model="appendCr"
                    type="checkbox"
                    :disabled="!network.active || sending"
                    class="size-3 accent-[var(--signal)]"
                  />
                  <span class="numeric">\r</span>
                </label>
                <label
                  class="flex cursor-pointer items-center gap-1 text-[10px] text-muted-foreground"
                >
                  <input
                    v-model="appendLf"
                    type="checkbox"
                    :disabled="!network.active || sending"
                    class="size-3 accent-[var(--signal)]"
                  />
                  <span class="numeric">\n</span>
                </label>
              </div>

              <div
                class="flex shrink-0 rounded-lg border border-hairline bg-well p-0.5"
                role="group"
                :aria-label="t('network.send')"
              >
                <button
                  v-for="option in formats"
                  :key="option"
                  type="button"
                  :disabled="!network.active || sending"
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
                :title="preview.valid ? t('network.send') : t('network.invalidPayload')"
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
        </section>
      </div>
    </DialogContent>
  </Dialog>
</template>
