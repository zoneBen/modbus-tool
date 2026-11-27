<script setup lang="ts">
import { ArrowLeftRight, Check, Copy, Binary } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import {
  base64ToHex,
  decodeBase64,
  decodeUrl,
  encodeBase64,
  encodeUrl,
  hexToBase64,
  hexToText,
  textToHex,
} from "@/lib/codec";
import { useToolboxStore } from "@/stores/toolbox";

type Operation = "b64e" | "b64d" | "urle" | "urld" | "hexe" | "hexd" | "b64hex" | "hexb64";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const operation = toolbox.toolPref<Operation>("dev.codec.operation", "b64e");
const urlSafe = toolbox.toolPref("dev.codec.urlSafe", false);
const input = toolbox.toolPref("dev.codec.input", "");

const operations = computed<Array<{ value: Operation; label: string }>>(() => [
  { value: "b64e", label: t("devTools.codec.ops.b64e") },
  { value: "b64d", label: t("devTools.codec.ops.b64d") },
  { value: "urle", label: t("devTools.codec.ops.urle") },
  { value: "urld", label: t("devTools.codec.ops.urld") },
  { value: "hexe", label: t("devTools.codec.ops.hexe") },
  { value: "hexd", label: t("devTools.codec.ops.hexd") },
  { value: "b64hex", label: t("devTools.codec.ops.b64hex") },
  { value: "hexb64", label: t("devTools.codec.ops.hexb64") },
]);

const output = computed(() => {
  const value = input.value;
  if (!value) return "";
  try {
    switch (operation.value) {
      case "b64e":
        return encodeBase64(value, urlSafe.value);
      case "b64d":
        return decodeBase64(value);
      case "urle":
        return encodeUrl(value);
      case "urld":
        return decodeUrl(value);
      case "hexe":
        return textToHex(value, " ");
      case "hexd":
        return hexToText(value);
      case "b64hex":
        return base64ToHex(value);
      case "hexb64":
        return hexToBase64(value);
      default:
        return "";
    }
  } catch (error: unknown) {
    return error instanceof Error && error.message === "hexOddLength"
      ? t("devTools.codec.hexOdd")
      : t("devTools.codec.invalid");
  }
});

function swap(): void {
  const next = output.value;
  input.value = next;
  const inverse: Record<Operation, Operation> = {
    b64e: "b64d",
    b64d: "b64e",
    urle: "urld",
    urld: "urle",
    hexe: "hexd",
    hexd: "hexe",
    b64hex: "hexb64",
    hexb64: "b64hex",
  };
  operation.value = inverse[operation.value];
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.codec.title')"
    :description="t('devTools.tools.codec.description')"
    :icon="Binary"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="min-w-[240px] flex-1 space-y-1.5">
          <p class="text-[11px] text-muted-foreground">{{ t("devTools.codec.operation") }}</p>
          <FormSelect v-model="operation" :options="operations" />
        </div>
        <label v-if="operation === 'b64e'" class="flex items-center gap-2 pb-2 text-[11px]">
          <input v-model="urlSafe" type="checkbox" class="size-3.5 accent-signal" />
          {{ t("devTools.codec.urlSafe") }}
        </label>
        <Button
          variant="outline"
          size="sm"
          class="h-9 gap-1.5 border-hairline-strong bg-transparent text-[11px]"
          @click="swap"
        >
          <ArrowLeftRight class="size-3.5" />
          {{ t("devTools.codec.swap") }}
        </Button>
      </div>

      <div class="grid gap-3 lg:grid-cols-2">
        <div class="space-y-1.5">
          <p class="text-[11px] font-medium">{{ t("devTools.codec.input") }}</p>
          <textarea
            v-model="input"
            class="scrollbar-thin numeric h-[320px] w-full resize-none rounded-xl border border-hairline bg-well px-3 py-2.5 text-[12px] leading-relaxed outline-none focus:border-signal/50"
            :placeholder="t('devTools.codec.inputPlaceholder')"
          />
        </div>
        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <p class="text-[11px] font-medium">{{ t("devTools.codec.output") }}</p>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
              :disabled="!output"
              @click="copy('output', output)"
            >
              <Check v-if="isCopied('output')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("output") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
          <textarea
            :value="output"
            readonly
            class="scrollbar-thin numeric h-[320px] w-full resize-none rounded-xl border border-hairline bg-well-soft px-3 py-2.5 text-[12px] leading-relaxed text-foreground outline-none"
            placeholder="—"
          />
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
