<script setup lang="ts">
import { Copy, Terminal } from "@lucide/vue";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import type { TelnetResult } from "@/types/tools";

const { t } = useI18n();
const host = ref("127.0.0.1");
const port = ref("23");
const timeoutMs = ref("3000");
const readTimeoutMs = ref("800");
const payload = ref("");
const payloadMode = ref<"ascii" | "hex">("ascii");
const appendCr = ref(true);
const appendLf = ref(false);
const busy = ref(false);
const result = ref<TelnetResult | null>(null);
const error = ref<string | null>(null);

const modeOptions = [
  { label: "ASCII", value: "ascii" as const },
  { label: "HEX", value: "hex" as const },
];

async function run(): Promise<void> {
  if (busy.value) return;
  if (!host.value.trim()) {
    error.value = t("tools.errors.invalidHost");
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    result.value = await ipc.toolTelnetTest({
      host: host.value.trim(),
      port: Math.max(1, Math.min(65_535, Number(port.value) || 23)),
      timeoutMs: Math.min(30_000, Math.max(100, Number(timeoutMs.value) || 3000)),
      readTimeoutMs: Math.min(10_000, Math.max(50, Number(readTimeoutMs.value) || 800)),
      payload: payload.value.trim() || undefined,
      payloadMode: payloadMode.value,
      appendCr: appendCr.value,
      appendLf: appendLf.value,
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}

async function copy(value: string): Promise<void> {
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    toast.success(t("common.copied"));
  } catch {
    toast.error(t("tools.copyFailed"));
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.telnet.title')"
    :description="t('networkTools.tools.telnet.description')"
    :icon="Terminal"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div
        class="grid grid-cols-2 gap-3 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-6"
      >
        <div class="col-span-2 space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.telnet.host")
          }}</Label>
          <Input
            v-model="host"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            @keydown.enter="run"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.telnet.port")
          }}</Label>
          <Input
            v-model="port"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.telnet.timeout")
          }}</Label>
          <Input
            v-model="timeoutMs"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.telnet.readTimeout")
          }}</Label>
          <Input
            v-model="readTimeoutMs"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="flex items-end">
          <Button size="sm" class="h-9 w-full gap-1.5 text-[12px]" :disabled="busy" @click="run">
            <Terminal class="size-3.5" />
            {{ busy ? t("tools.running") : t("networkTools.telnet.test") }}
          </Button>
        </div>

        <div class="col-span-3 space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.telnet.payload")
          }}</Label>
          <Input
            v-model="payload"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            :placeholder="t('networkTools.telnet.payloadPlaceholder')"
            @keydown.enter="run"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.telnet.mode")
          }}</Label>
          <FormSelect v-model="payloadMode" :options="modeOptions" />
        </div>
        <div class="col-span-2 flex items-end gap-3">
          <label class="flex items-center gap-2 text-[11px] text-muted-foreground">
            <Switch v-model="appendCr" />
            CR
          </label>
          <label class="flex items-center gap-2 text-[11px] text-muted-foreground">
            <Switch v-model="appendLf" />
            LF
          </label>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="space-y-3">
        <div
          class="flex flex-wrap items-center gap-2 rounded-xl border border-hairline bg-well-soft px-3 py-2.5 text-[11px]"
        >
          <span class="rounded border border-signal/25 bg-signal/10 px-1.5 py-0.5 text-signal">
            {{ t("networkTools.telnet.connected") }}
          </span>
          <span class="numeric text-muted-foreground">
            {{ result.host }}:{{ result.port }} · {{ result.latencyMs }} ms ·
            {{ t("networkTools.telnet.bytes", { count: result.bytesRead }) }}
          </span>
        </div>

        <div class="grid gap-3 lg:grid-cols-2">
          <div class="rounded-xl border border-hairline">
            <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
              <span class="text-[11px] font-medium">ASCII</span>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 text-[11px] text-muted-foreground"
                :disabled="!result.text"
                @click="copy(result.text)"
              >
                <Copy class="size-3" />
                {{ t("tools.copy") }}
              </Button>
            </div>
            <pre
              class="scrollbar-thin max-h-[320px] overflow-auto whitespace-pre-wrap break-all px-3 py-2.5 text-[11px] text-muted-foreground"
              >{{ result.text || "—" }}</pre>
          </div>
          <div class="rounded-xl border border-hairline">
            <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
              <span class="text-[11px] font-medium">HEX</span>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 text-[11px] text-muted-foreground"
                :disabled="!result.hex"
                @click="copy(result.hex)"
              >
                <Copy class="size-3" />
                {{ t("tools.copy") }}
              </Button>
            </div>
            <pre
              class="scrollbar-thin max-h-[320px] overflow-auto whitespace-pre-wrap break-all px-3 py-2.5 text-[11px] text-muted-foreground"
              >{{ result.hex || "—" }}</pre>
          </div>
        </div>
      </div>

      <div
        v-else-if="!busy"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.telnet.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
