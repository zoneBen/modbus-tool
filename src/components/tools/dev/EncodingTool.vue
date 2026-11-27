<script setup lang="ts">
import { ArrowLeftRight, Check, Copy, Languages } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { TextConvertInput, TextConvertResult } from "@/types/tools";

const ENCODINGS = [
  { value: "utf-8", label: "UTF-8" },
  { value: "gbk", label: "GBK" },
  { value: "gb18030", label: "GB18030" },
  { value: "utf-16le", label: "UTF-16 LE" },
  { value: "utf-16be", label: "UTF-16 BE" },
  { value: "big5", label: "Big5" },
  { value: "shift_jis", label: "Shift_JIS" },
  { value: "iso-8859-1", label: "ISO-8859-1" },
];

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const text = toolbox.toolPref("dev.encoding.text", "中文编码测试");
const inputMode = toolbox.toolPref<"text" | "hex">("dev.encoding.mode", "text");
const fromEncoding = toolbox.toolPref("dev.encoding.from", "utf-8");
const toEncoding = toolbox.toolPref("dev.encoding.to", "gbk");
const result = ref<TextConvertResult | null>(null);
const error = ref<string | null>(null);
const busy = ref(false);

const modeOptions = computed(() => [
  { value: "text" as const, label: t("devTools.encoding.textMode") },
  { value: "hex" as const, label: t("devTools.encoding.hexMode") },
]);

async function convert(): Promise<void> {
  if (!text.value) {
    result.value = null;
    error.value = null;
    return;
  }
  busy.value = true;
  try {
    const input: TextConvertInput = {
      text: text.value,
      inputMode: inputMode.value,
      fromEncoding: fromEncoding.value,
      toEncoding: toEncoding.value,
    };
    result.value = await ipc.toolTextConvert(input);
    error.value = null;
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}

function swap(): void {
  const previous = fromEncoding.value;
  fromEncoding.value = toEncoding.value;
  toEncoding.value = previous;
  if (result.value) {
    text.value = result.value.text;
    inputMode.value = "text";
  }
  void convert();
}

let timer: ReturnType<typeof setTimeout> | null = null;
watch([text, inputMode, fromEncoding, toEncoding], () => {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void convert(), 220);
});
void convert();
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.encoding.title')"
    :description="t('devTools.tools.encoding.description')"
    :icon="Languages"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.encoding.mode") }}</Label>
          <div class="w-32">
            <FormSelect v-model="inputMode" :options="modeOptions" />
          </div>
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.encoding.from") }}</Label>
          <div class="w-40">
            <FormSelect v-model="fromEncoding" :options="ENCODINGS" />
          </div>
        </div>
        <Button
          variant="outline"
          size="icon"
          class="size-9 border-hairline-strong bg-transparent"
          @click="swap"
        >
          <ArrowLeftRight class="size-4" />
        </Button>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.encoding.to") }}</Label>
          <div class="w-40">
            <FormSelect v-model="toEncoding" :options="ENCODINGS" />
          </div>
        </div>
        <span class="numeric ml-auto pb-2 text-[10px] text-muted-foreground">
          {{ busy ? t("tools.running") : "" }}
        </span>
      </div>

      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <Label class="text-[11px] text-muted-foreground">
            {{
              inputMode === "hex"
                ? t("devTools.encoding.hexInput")
                : t("devTools.encoding.textInput")
            }}
          </Label>
        </div>
        <textarea
          v-model="text"
          class="scrollbar-thin numeric h-[140px] w-full resize-y rounded-xl border border-hairline bg-well px-3 py-2.5 text-[12px] leading-relaxed outline-none focus:border-signal/50"
          :placeholder="inputMode === 'hex' ? 'D6 D0 CE C4' : '中文编码测试'"
        />
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="grid gap-3 lg:grid-cols-2">
        <div class="rounded-xl border border-hairline">
          <div class="flex items-center gap-2 border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">{{ t("devTools.encoding.decoded") }}</span>
            <span class="numeric text-[9.5px] text-muted-foreground">{{ fromEncoding }}</span>
            <Button
              variant="ghost"
              size="sm"
              class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
              @click="copy('decoded', result.text)"
            >
              <Check v-if="isCopied('decoded')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("decoded") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
          <pre
            class="scrollbar-thin numeric max-h-[240px] min-h-[120px] overflow-auto whitespace-pre-wrap break-all px-3 py-2 text-[12px]"
            >{{ result.text }}</pre>
        </div>

        <div class="rounded-xl border border-hairline">
          <div class="flex items-center gap-2 border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">{{ t("devTools.encoding.encoded") }}</span>
            <span class="numeric text-[9.5px] text-muted-foreground">
              {{ toEncoding }} · {{ result.bytes }} B
            </span>
            <Button
              variant="ghost"
              size="sm"
              class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
              @click="copy('hex', result.hex)"
            >
              <Check v-if="isCopied('hex')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("hex") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
          <pre
            class="scrollbar-thin numeric max-h-[240px] min-h-[120px] overflow-auto whitespace-pre-wrap break-all px-3 py-2 text-[12px]"
            >{{ result.hex }}</pre>
        </div>
      </div>

      <p v-if="result?.lossy" class="text-[10.5px] text-warn">{{ t("devTools.encoding.lossy") }}</p>
    </div>
  </ToolPanelFrame>
</template>
