<script setup lang="ts">
import { Braces, Copy, Eraser, Play, RefreshCw } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useToolboxStore } from "@/stores/toolbox";
import { formatText } from "@/lib/tool-formatter";
import type { FormatLanguage } from "@/lib/tool-formatter";

const { t } = useI18n();
const toolbox = useToolboxStore();
const MAX_INPUT_BYTES = 2 * 1024 * 1024;
const language = toolbox.toolPref<FormatLanguage>("dev.formatter.language", "json");
const indent = toolbox.toolPref<"2" | "4">("dev.formatter.indent", "2");
const input = ref("");
const output = ref("");
const busy = ref(false);
const error = ref<string | null>(null);

const languageOptions: Array<{ label: string; value: FormatLanguage }> = [
  { label: "JSON", value: "json" },
  { label: "JavaScript", value: "javascript" },
  { label: "TypeScript", value: "typescript" },
  { label: "HTML", value: "html" },
  { label: "CSS", value: "css" },
  { label: "XML", value: "xml" },
];

const indentOptions = [
  { label: "2", value: "2" },
  { label: "4", value: "4" },
];

const inputBytes = computed(() => new TextEncoder().encode(input.value).length);

async function format(): Promise<void> {
  if (busy.value) return;
  if (!input.value.trim()) {
    error.value = t("devTools.formatter.empty");
    return;
  }
  if (inputBytes.value > MAX_INPUT_BYTES) {
    error.value = t("devTools.formatter.tooLarge", { max: 2 });
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    output.value = await formatText(input.value, language.value, Number(indent.value) as 2 | 4);
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : String(cause);
    output.value = "";
  } finally {
    busy.value = false;
  }
}

function swap(): void {
  const previous = input.value;
  input.value = output.value;
  output.value = previous;
}

function clear(): void {
  input.value = "";
  output.value = "";
  error.value = null;
}

async function copy(): Promise<void> {
  if (!output.value) return;
  try {
    await navigator.clipboard.writeText(output.value);
    toast.success(t("common.copied"));
  } catch {
    toast.error(t("tools.copyFailed"));
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.formatter.title')"
    :description="t('devTools.tools.formatter.description')"
    :icon="Braces"
  >
    <div class="mx-auto flex h-full max-w-[1280px] min-h-[520px] flex-col gap-3">
      <div
        class="flex flex-wrap items-center gap-3 rounded-xl border border-hairline bg-well-soft px-3 py-2"
      >
        <div class="flex items-center gap-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.formatter.language")
          }}</Label>
          <div class="w-[150px]">
            <FormSelect v-model="language" :options="languageOptions" />
          </div>
        </div>
        <div class="flex items-center gap-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.formatter.indent")
          }}</Label>
          <div class="w-[76px]">
            <FormSelect v-model="indent" :options="indentOptions" />
          </div>
        </div>
        <span class="numeric text-[10.5px] text-muted-foreground"> {{ inputBytes }} B / 2 MB </span>
        <div class="ml-auto flex items-center gap-2">
          <Button size="sm" class="h-8 gap-1.5 text-[12px]" :disabled="busy" @click="format">
            <Play class="size-3.5" />
            {{ busy ? t("devTools.formatter.formatting") : t("devTools.formatter.format") }}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1 text-[11px] text-muted-foreground"
            :disabled="!output"
            @click="swap"
          >
            <RefreshCw class="size-3" />
            {{ t("devTools.formatter.swap") }}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1 text-[11px] text-muted-foreground"
            :disabled="!input && !output"
            @click="clear"
          >
            <Eraser class="size-3" />
            {{ t("common.clear") }}
          </Button>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div class="grid min-h-[420px] flex-1 gap-3 lg:grid-cols-2">
        <div class="flex min-h-[320px] flex-col rounded-xl border border-hairline bg-well">
          <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">{{ t("devTools.formatter.input") }}</span>
            <span class="text-[10px] text-muted-foreground">{{
              t("devTools.formatter.inputHint")
            }}</span>
          </div>
          <textarea
            v-model="input"
            spellcheck="false"
            class="numeric scrollbar-thin min-h-0 flex-1 resize-none bg-transparent px-3 py-2.5 text-[11.5px] leading-relaxed text-foreground outline-none"
            :placeholder="t('devTools.formatter.placeholder')"
          />
        </div>
        <div class="flex min-h-[320px] flex-col rounded-xl border border-hairline bg-well">
          <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">{{ t("devTools.formatter.output") }}</span>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
              :disabled="!output"
              @click="copy"
            >
              <Copy class="size-3" />
              {{ t("tools.copy") }}
            </Button>
          </div>
          <textarea
            :value="output"
            readonly
            spellcheck="false"
            class="numeric scrollbar-thin min-h-0 flex-1 resize-none bg-transparent px-3 py-2.5 text-[11.5px] leading-relaxed text-muted-foreground outline-none"
            :placeholder="t('devTools.formatter.outputPlaceholder')"
          />
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
