<script setup lang="ts">
import { LoaderCircle, Send, Terminal } from "@lucide/vue";
import { computed, ref } from "vue";
import { toast } from "vue-sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { encodeRawPayload } from "@/lib/raw-send";
import { useSessionsStore } from "@/stores/sessions";
import type { RawSendMode } from "@/types/models";

const sessions = useSessionsStore();
const mode = ref<RawSendMode>("ascii");
const modes: RawSendMode[] = ["ascii", "hex"];
const payload = ref("");
const busy = ref(false);

const session = computed(() => sessions.activeSession);
const isSlave = computed(() => session.value?.role === "slave");
const connected = computed(() => session.value?.status === "connected");

const preview = computed<{ bytes: number | null; error: string }>(() => {
  if (!payload.value.length) return { bytes: 0, error: "" };
  try {
    return { bytes: encodeRawPayload(payload.value, mode.value).length, error: "" };
  } catch (cause: unknown) {
    return {
      bytes: null,
      error: cause instanceof Error ? cause.message : String(cause),
    };
  }
});

const canSend = computed(
  () =>
    Boolean(session.value) &&
    !isSlave.value &&
    connected.value &&
    payload.value.length > 0 &&
    preview.value.bytes !== null &&
    !busy.value,
);

const disabledReason = computed(() => {
  if (!session.value) return "请先选择会话";
  if (isSlave.value) return "从站模拟会话不支持手动发送";
  if (!connected.value) return "请先启动会话链路";
  if (!payload.value.length) return "请输入要发送的数据";
  if (preview.value.error) return preview.value.error;
  return "";
});

const placeholder = computed(() => {
  if (isSlave.value) return "从站模拟会话不支持手动发送";
  if (!connected.value) return "连接会话后可发送原始数据";
  return mode.value === "ascii"
    ? "输入要发送的字符，按 Enter 发送"
    : "输入十六进制字节，例如 01 03 00 00 00 0A";
});

async function send(): Promise<void> {
  if (!canSend.value || busy.value) return;
  busy.value = true;
  try {
    const result = await sessions.sendRaw({
      data: payload.value,
      mode: mode.value,
    });
    toast.success(`已发送 ${result.bytesSent} 字节`, {
      description:
        result.bytesReceived > 0
          ? `收到 ${result.bytesReceived} 字节 · ${result.latencyMs.toFixed(1)} ms`
          : "未收到响应",
    });
  } catch (cause: unknown) {
    toast.error("发送原始数据失败", {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div
    class="flex h-10 shrink-0 items-center gap-2 border-b border-hairline bg-[color:var(--surface)]/72 px-3"
  >
    <span
      class="hidden shrink-0 items-center gap-1.5 text-[10.5px] font-medium tracking-wide text-muted-foreground sm:flex"
    >
      <Terminal class="size-3.5 text-signal/80" />
      手动发送
    </span>
    <span class="numeric shrink-0 text-[13px] text-signal/70">›</span>

    <Input
      v-model="payload"
      :placeholder="placeholder"
      :disabled="busy || isSlave"
      spellcheck="false"
      autocomplete="off"
      aria-label="手动发送数据"
      class="numeric h-7 min-w-0 flex-1 rounded-lg border-hairline bg-well px-2.5 text-[11.5px] placeholder:font-sans placeholder:text-muted-foreground/80 focus-visible:border-signal/40 focus-visible:ring-signal/10"
      :class="preview.error ? 'border-error/45 focus-visible:border-error/60' : ''"
      @keydown.enter.prevent="send"
    />

    <span
      class="numeric hidden shrink-0 text-[10.5px] text-muted-foreground md:inline"
      :class="preview.error ? 'text-error' : ''"
    >
      {{ preview.bytes === null ? "ERR" : `${preview.bytes} B` }}
    </span>
    <span
      v-if="preview.error"
      class="hidden max-w-[180px] shrink-0 truncate text-[10.5px] text-error xl:inline"
      :title="preview.error"
    >
      {{ preview.error }}
    </span>

    <div
      class="flex shrink-0 rounded-lg border border-hairline bg-well p-0.5"
      role="group"
      aria-label="发送格式"
    >
      <button
        v-for="option in modes"
        :key="option"
        type="button"
        :disabled="busy"
        class="numeric rounded-md px-1.5 py-0.5 text-[10.5px] uppercase transition-colors disabled:opacity-50"
        :class="
          mode === option
            ? option === 'hex'
              ? 'bg-rx/15 text-rx'
              : 'bg-signal/15 text-signal'
            : 'text-muted-foreground hover:text-foreground'
        "
        :aria-pressed="mode === option"
        @click="mode = option"
      >
        {{ option }}
      </button>
    </div>

    <Button
      variant="outline"
      size="sm"
      class="h-7 shrink-0 gap-1.5 border-signal/25 bg-signal/10 px-2.5 text-[11.5px] text-signal hover:bg-signal/15 disabled:border-hairline disabled:bg-overlay-1 disabled:text-muted-foreground"
      :disabled="!canSend"
      :title="disabledReason || '发送原始数据'"
      aria-label="发送原始数据"
      @click="send"
    >
      <LoaderCircle v-if="busy" class="size-3.5 animate-spin" />
      <Send v-else class="size-3.5" />
      <span class="hidden sm:inline">{{ busy ? "发送中" : "发送" }}</span>
    </Button>
  </div>
</template>
