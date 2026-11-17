<script setup lang="ts">
import { ArrowDownToLine, CircleAlert, Eraser, LoaderCircle, Send, Terminal } from "@lucide/vue";
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
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
import { encodeRawPayload, formatDebugBytes } from "@/lib/raw-send";
import { useDebugStore } from "@/stores/debug";
import { useSessionsStore } from "@/stores/sessions";
import type { RawSendMode } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const sessions = useSessionsStore();
const debug = useDebugStore();
const { t } = useI18n();

const sendMode = ref<RawSendMode>("ascii");
const displayMode = ref<"ascii" | "hex">("ascii");
const formats: Array<"ascii" | "hex"> = ["ascii", "hex"];
const payload = ref("");
const appendCr = ref(false);
const appendLf = ref(false);
const sending = ref(false);
const follow = ref(true);
const listEl = ref<HTMLElement | null>(null);

const session = computed(() => sessions.activeSession);
const preview = computed<{ bytes: number | null; valid: boolean }>(() => {
  if (!payload.value.length && !appendCr.value && !appendLf.value) {
    return { bytes: 0, valid: true };
  }
  try {
    return {
      bytes: encodeRawPayload(payload.value, sendMode.value, {
        appendCr: appendCr.value,
        appendLf: appendLf.value,
      }).length,
      valid: true,
    };
  } catch {
    return { bytes: null, valid: false };
  }
});
const canSend = computed(
  () =>
    debug.active &&
    !sending.value &&
    preview.value.valid &&
    preview.value.bytes !== null &&
    preview.value.bytes > 0,
);
const rxBytes = computed(() =>
  debug.entries
    .filter((entry) => entry.direction === "rx")
    .reduce((total, entry) => total + entry.data.length, 0),
);
const txBytes = computed(() =>
  debug.entries
    .filter((entry) => entry.direction === "tx")
    .reduce((total, entry) => total + entry.data.length, 0),
);
const placeholder = computed(() => {
  if (!debug.active) return t("debug.inputDisabled");
  return sendMode.value === "ascii" ? t("debug.inputPlaceholder") : t("debug.inputPlaceholderHex");
});

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
  () => debug.entries.length,
  () => {
    if (follow.value) void scrollToEnd();
  },
);

watch(open, async (isOpen) => {
  if (isOpen) {
    const current = session.value;
    if (!current) {
      open.value = false;
      return;
    }
    payload.value = "";
    follow.value = true;
    try {
      await debug.start(current.id);
      await scrollToEnd();
    } catch (cause: unknown) {
      toast.error(t("debug.startFailed"), {
        description: cause instanceof Error ? cause.message : String(cause),
      });
      open.value = false;
    }
    return;
  }
  try {
    await debug.stop();
  } catch (cause: unknown) {
    toast.error(t("debug.stopFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  }
});

onBeforeUnmount(() => {
  if (debug.active) void debug.stop();
});

async function send(): Promise<void> {
  if (!canSend.value || sending.value) return;
  sending.value = true;
  try {
    const count = await debug.send({
      data: payload.value,
      mode: sendMode.value,
      appendCr: appendCr.value,
      appendLf: appendLf.value,
    });
    toast.success(t("debug.sent", { count }), {
      description: t("debug.sentDescription"),
    });
  } catch (cause: unknown) {
    toast.error(t("debug.sendFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  } finally {
    sending.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="flex h-[min(780px,88vh)] flex-col gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/30 backdrop-blur-2xl sm:max-w-[920px]"
    >
      <DialogHeader class="shrink-0 border-b border-hairline px-5 py-4">
        <div class="flex items-start gap-3 pr-7">
          <span
            class="flex size-9 shrink-0 items-center justify-center rounded-xl border border-signal/25 bg-signal/10 text-signal"
          >
            <Terminal class="size-4" />
          </span>
          <div class="min-w-0 flex-1">
            <DialogTitle class="text-[14px]">{{ t("debug.title") }}</DialogTitle>
            <DialogDescription class="mt-0.5 truncate text-[11px] text-muted-foreground">
              {{ session?.name ?? "—" }} · {{ session?.endpoint ?? "—" }} ·
              {{ t("debug.subtitle") }}
            </DialogDescription>
          </div>
          <span
            class="inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2 py-1 text-[10.5px] font-medium"
            :class="
              debug.active
                ? 'border-signal/25 bg-signal/10 text-signal'
                : 'border-hairline bg-overlay-1 text-muted-foreground'
            "
          >
            <span
              class="status-dot scale-75"
              :class="debug.active ? 'status-dot-live text-signal' : 'opacity-40'"
            />
            {{ debug.active ? t("debug.active") : t("debug.inactive") }}
          </span>
        </div>
      </DialogHeader>

      <div
        class="flex shrink-0 flex-wrap items-center gap-2 border-b border-hairline bg-overlay-1/45 px-4 py-2"
      >
        <span class="numeric text-[10.5px] text-muted-foreground">
          <span class="text-rx">RX</span> {{ rxBytes }} B
        </span>
        <span class="text-foreground/15">/</span>
        <span class="numeric text-[10.5px] text-muted-foreground">
          <span class="text-signal">TX</span> {{ txBytes }} B
        </span>
        <span class="text-foreground/15">/</span>
        <span class="text-[10.5px] text-muted-foreground">
          {{ t("debug.entries", { count: debug.entries.length }) }}
        </span>

        <div class="ml-auto flex items-center gap-1.5">
          <span class="text-[10.5px] text-muted-foreground">{{ t("debug.display") }}</span>
          <div
            class="flex rounded-lg border border-hairline bg-well p-0.5"
            role="group"
            :aria-label="t('debug.display')"
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
            :title="t('debug.follow')"
            @click="toggleFollow"
          >
            <ArrowDownToLine class="size-3" />
            <span class="hidden sm:inline">{{ t("debug.follow") }}</span>
          </button>
          <button
            type="button"
            class="flex items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] text-muted-foreground transition-colors hover:text-foreground"
            :title="t('debug.clear')"
            @click="debug.clear()"
          >
            <Eraser class="size-3" />
            <span class="hidden sm:inline">{{ t("debug.clear") }}</span>
          </button>
        </div>
      </div>

      <div
        v-if="debug.error"
        class="flex shrink-0 items-start gap-2 border-b border-error/25 bg-error/10 px-4 py-2 text-[11px] leading-relaxed text-error"
      >
        <CircleAlert class="mt-0.5 size-3.5 shrink-0" />
        <span>{{ t("debug.linkError") }}: {{ debug.error }}</span>
      </div>

      <div
        ref="listEl"
        class="scrollbar-thin min-h-[260px] flex-1 overflow-auto bg-well/45 px-4 py-3"
        @scroll="onScroll"
      >
        <div
          v-if="!debug.entries.length"
          class="flex h-full min-h-[220px] flex-col items-center justify-center gap-2 text-center"
        >
          <span class="status-dot status-dot-live text-signal" />
          <p class="text-[12px] text-muted-foreground">{{ t("debug.empty") }}</p>
          <p class="max-w-md text-[10.5px] leading-relaxed text-muted-foreground/80">
            {{ t("debug.emptyHint") }}
          </p>
        </div>

        <div v-else class="space-y-0.5 font-mono">
          <div
            v-for="entry in debug.entries"
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
            <pre
              class="min-w-0 flex-1 whitespace-pre-wrap break-all font-mono text-[11.5px] leading-relaxed"
              :class="entry.direction === 'tx' ? 'text-signal/90' : 'text-foreground/90'"
              >{{ formatDebugBytes(entry.data, displayMode) }}</pre>
          </div>
        </div>
      </div>

      <div class="shrink-0 border-t border-hairline bg-overlay-1/40 px-4 py-3">
        <div class="mb-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10.5px]">
          <span class="flex items-center gap-1.5 text-muted-foreground">
            <span
              class="status-dot scale-75"
              :class="debug.active ? 'text-signal' : 'opacity-40'"
            />
            {{ debug.active ? t("debug.active") : t("debug.inactive") }}
          </span>
          <span class="text-foreground/15">/</span>
          <span class="text-muted-foreground">{{ t("debug.pollingOff") }}</span>
          <span class="ml-auto text-right text-muted-foreground/80">{{ t("debug.note") }}</span>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <span class="numeric shrink-0 text-[13px] text-signal/70">›</span>
          <Input
            v-model="payload"
            :placeholder="placeholder"
            :disabled="!debug.active || sending"
            spellcheck="false"
            autocomplete="off"
            :aria-label="t('debug.inputLabel')"
            class="numeric h-8 min-w-[220px] flex-1 rounded-lg border-hairline bg-well px-2.5 text-[11.5px] placeholder:font-sans placeholder:text-muted-foreground/80 focus-visible:border-signal/40 focus-visible:ring-signal/10"
            :class="preview.valid ? '' : 'border-error/45 focus-visible:border-error/60'"
            @keydown.enter.prevent="send"
          />

          <span class="numeric hidden shrink-0 text-[10.5px] text-muted-foreground md:inline">
            {{ preview.bytes === null ? "ERR" : `${preview.bytes} B` }}
          </span>

          <div
            class="flex shrink-0 items-center gap-2 rounded-lg border border-hairline bg-well px-2 py-1"
            :aria-label="t('debug.suffix')"
          >
            <span class="text-[10px] text-muted-foreground">{{ t("debug.suffix") }}</span>
            <label class="flex cursor-pointer items-center gap-1 text-[10px] text-muted-foreground">
              <input
                v-model="appendCr"
                type="checkbox"
                :disabled="!debug.active || sending"
                class="size-3 accent-[var(--signal)]"
              />
              <span class="numeric">\r</span>
            </label>
            <label class="flex cursor-pointer items-center gap-1 text-[10px] text-muted-foreground">
              <input
                v-model="appendLf"
                type="checkbox"
                :disabled="!debug.active || sending"
                class="size-3 accent-[var(--signal)]"
              />
              <span class="numeric">\n</span>
            </label>
          </div>

          <div
            class="flex shrink-0 rounded-lg border border-hairline bg-well p-0.5"
            role="group"
            :aria-label="t('debug.sendFormat')"
          >
            <button
              v-for="option in formats"
              :key="option"
              type="button"
              :disabled="!debug.active || sending"
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
            class="h-8 shrink-0 gap-1.5 border-signal/25 bg-signal/10 px-3 text-[11.5px] text-signal hover:bg-signal/15 disabled:border-hairline disabled:bg-overlay-1 disabled:text-muted-foreground"
            :disabled="!canSend"
            :title="preview.valid ? t('debug.send') : t('debug.invalidPayload')"
            @click="send"
          >
            <LoaderCircle v-if="sending" class="size-3.5 animate-spin" />
            <Send v-else class="size-3.5" />
            {{ sending ? t("debug.sending") : t("debug.send") }}
          </Button>
        </div>

        <p v-if="!preview.valid" class="mt-1.5 text-[10.5px] leading-relaxed text-error">
          {{ t("debug.invalidPayload") }}
        </p>
      </div>
    </DialogContent>
  </Dialog>
</template>
