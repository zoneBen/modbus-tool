<script setup lang="ts">
import { Binary, Check, Copy } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useToolboxStore } from "@/stores/toolbox";
import { describeBase, parseInteger } from "@/lib/converters";
import type { BaseRadix } from "@/lib/converters";

const { t } = useI18n();
const toolbox = useToolboxStore();
const input = ref("FF");
const radix = toolbox.toolPref<BaseRadix>("dev.base.radix", 16);

const radixOptions: Array<{ label: string; value: BaseRadix }> = [
  { label: "十六进制 (16)", value: 16 },
  { label: "十进制 (10)", value: 10 },
  { label: "八进制 (8)", value: 8 },
  { label: "二进制 (2)", value: 2 },
];

const parsed = computed(() => parseInteger(input.value, radix.value));
const outputs = computed(() => (parsed.value.ok ? describeBase(parsed.value.value) : null));
const errorText = computed(() => {
  if (parsed.value.ok) return "";
  return t(`devTools.base.errors.${parsed.value.error}`);
});

const baseRows = computed(() =>
  outputs.value
    ? [
        { label: "HEX", value: outputs.value.hex },
        { label: "DEC", value: outputs.value.decimal },
        { label: "OCT", value: outputs.value.octal },
        { label: "BIN", value: outputs.value.binary },
      ]
    : [],
);

const widthRows = computed(() =>
  outputs.value
    ? [
        { label: "UINT8", value: outputs.value.unsigned8 },
        { label: "INT8", value: outputs.value.signed8 },
        { label: "UINT16", value: outputs.value.unsigned16 },
        { label: "INT16", value: outputs.value.signed16 },
        { label: "UINT32", value: outputs.value.unsigned32 },
        { label: "INT32", value: outputs.value.signed32 },
        { label: "UINT64", value: outputs.value.unsigned64 },
        { label: "INT64", value: outputs.value.signed64 },
      ]
    : [],
);

const { copy, isCopied } = useCopyFeedback();
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.base.title')"
    :description="t('devTools.tools.base.description')"
    :icon="Binary"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div
        class="grid grid-cols-[minmax(0,1fr)_170px] gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.base.input") }}</Label>
          <Input
            v-model="input"
            class="numeric h-10 rounded-lg border-hairline-strong bg-well text-[15px] tracking-tight"
            :placeholder="t('devTools.base.placeholder')"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.base.inputRadix")
          }}</Label>
          <FormSelect v-model="radix" :options="radixOptions" />
        </div>
      </div>

      <p
        v-if="errorText"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ errorText }}
      </p>

      <div v-if="outputs" class="grid grid-cols-2 gap-2 lg:grid-cols-4">
        <div
          v-for="row in baseRows"
          :key="row.label"
          class="min-w-0 rounded-xl border border-hairline bg-well-soft p-3"
        >
          <div class="flex items-center justify-between">
            <span class="text-[10px] font-medium tracking-wider text-muted-foreground">{{
              row.label
            }}</span>
            <button
              type="button"
              class="flex items-center gap-1 text-muted-foreground hover:text-foreground"
              @click="copy(row.label, row.value)"
            >
              <Check v-if="isCopied(row.label)" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              <span v-if="isCopied(row.label)" class="text-[9px] text-signal">{{
                t("common.copiedShort")
              }}</span>
            </button>
          </div>
          <p class="numeric mt-1 break-all text-[13px] text-foreground">{{ row.value }}</p>
        </div>
      </div>

      <div v-if="outputs" class="grid gap-3 lg:grid-cols-2">
        <div class="rounded-xl border border-hairline bg-well-soft p-3">
          <p class="mb-2 text-[11px] font-medium">{{ t("devTools.base.widthTitle") }}</p>
          <div class="grid grid-cols-2 gap-1">
            <div
              v-for="row in widthRows"
              :key="row.label"
              class="flex items-center gap-2 rounded-md bg-overlay-1 px-2 py-1"
            >
              <span class="w-14 shrink-0 text-[9.5px] text-muted-foreground">{{ row.label }}</span>
              <span class="numeric min-w-0 flex-1 truncate text-[11px]">{{ row.value }}</span>
              <button
                type="button"
                class="flex items-center gap-1 text-muted-foreground hover:text-foreground"
                @click="copy(row.label, row.value)"
              >
                <Check v-if="isCopied(row.label)" class="size-3 text-signal" />
                <Copy v-else class="size-3" />
                <span v-if="isCopied(row.label)" class="text-[9px] text-signal">{{
                  t("common.copiedShort")
                }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
          <div>
            <p class="mb-2 text-[11px] font-medium">{{ t("devTools.base.floatTitle") }}</p>
            <div class="grid grid-cols-2 gap-2">
              <div class="rounded-md bg-overlay-1 px-2 py-2">
                <p class="text-[9.5px] text-muted-foreground">FLOAT32</p>
                <p class="numeric mt-0.5 truncate text-[12px]">{{ outputs.float32 }}</p>
              </div>
              <div class="rounded-md bg-overlay-1 px-2 py-2">
                <p class="text-[9.5px] text-muted-foreground">FLOAT64</p>
                <p class="numeric mt-0.5 truncate text-[12px]">{{ outputs.float64 }}</p>
              </div>
            </div>
          </div>
          <p class="text-[10.5px] leading-relaxed text-muted-foreground">
            {{ t("devTools.base.hint") }}
          </p>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
