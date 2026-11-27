<script setup lang="ts">
import { Check, Clock, Copy } from "@lucide/vue";
import { useNow } from "@vueuse/core";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useToolboxStore } from "@/stores/toolbox";
import { describeEpoch, parseDateText, parseEpoch } from "@/lib/converters";
import type { EpochUnit } from "@/lib/converters";

const { t } = useI18n();
const toolbox = useToolboxStore();
const now = useNow();
const epochInput = ref(String(Math.floor(Date.now() / 1000)));
const epochUnit = toolbox.toolPref<EpochUnit>("dev.timestamp.unit", "auto");
const dateInput = ref(new Date().toISOString());

const unitOptions: Array<{ label: string; value: EpochUnit }> = [
  { label: t("devTools.timestamp.units.auto"), value: "auto" },
  { label: "s", value: "s" },
  { label: "ms", value: "ms" },
  { label: "µs", value: "us" },
  { label: "ns", value: "ns" },
];

const nowOutputs = computed(() => describeEpoch(now.value.getTime()));

const epochResult = computed(() => {
  const parsed = parseEpoch(epochInput.value, epochUnit.value);
  if (!parsed.ok) return null;
  return { ...describeEpoch(parsed.value.ms), unit: parsed.value.unit };
});

const dateResult = computed(() => {
  const parsed = parseDateText(dateInput.value);
  if (!parsed.ok) return null;
  return describeEpoch(parsed.value);
});

const epochError = computed(() => {
  const parsed = parseEpoch(epochInput.value, epochUnit.value);
  if (parsed.ok) return "";
  return t(`devTools.timestamp.errors.${parsed.error}`);
});

const dateError = computed(() => {
  const parsed = parseDateText(dateInput.value);
  if (parsed.ok) return "";
  return t(`devTools.timestamp.errors.${parsed.error}`);
});

function setEpochFromNow(): void {
  epochInput.value = String(Math.floor(now.value.getTime() / 1000));
  epochUnit.value = "s";
}

const { copy: copyFeedback, isCopied } = useCopyFeedback();

async function copy(key: string, value: string): Promise<void> {
  if (!value) return;
  await copyFeedback(key, value);
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.timestamp.title')"
    :description="t('devTools.tools.timestamp.description')"
    :icon="Clock"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div class="rounded-xl border border-hairline bg-well-soft p-3">
        <div class="mb-2 flex items-center gap-2">
          <span class="status-dot status-dot-live text-signal" />
          <span class="text-[11px] font-medium">{{ t("devTools.timestamp.current") }}</span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-7 text-[10.5px]"
            @click="setEpochFromNow"
          >
            {{ t("devTools.timestamp.useNow") }}
          </Button>
        </div>
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
          <div
            v-for="item in [
              { label: t('devTools.timestamp.local'), value: nowOutputs.local },
              { label: 'UTC', value: nowOutputs.utc },
              { label: 'ISO 8601', value: nowOutputs.iso },
              { label: 'Epoch (s)', value: nowOutputs.seconds },
            ]"
            :key="item.label"
            class="min-w-0 rounded-lg border border-hairline bg-well px-2.5 py-2"
          >
            <p class="text-[10px] text-muted-foreground">{{ item.label }}</p>
            <div class="mt-0.5 flex items-center gap-1">
              <span class="numeric min-w-0 flex-1 truncate text-[11px]">{{ item.value }}</span>
              <button
                type="button"
                class="flex shrink-0 items-center gap-1 text-muted-foreground hover:text-foreground"
                :aria-label="t('tools.copy')"
                @click="copy(item.label, item.value)"
              >
                <Check v-if="isCopied(item.label)" class="size-3 text-signal" />
                <Copy v-else class="size-3" />
                <span v-if="isCopied(item.label)" class="text-[9px] text-signal">{{
                  t("common.copiedShort")
                }}</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="grid gap-3 lg:grid-cols-2">
        <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
          <p class="text-[11px] font-medium">{{ t("devTools.timestamp.epochTitle") }}</p>
          <div class="grid grid-cols-[minmax(0,1fr)_112px] gap-2">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{
                t("devTools.timestamp.epochInput")
              }}</Label>
              <Input
                v-model="epochInput"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{
                t("devTools.timestamp.unit")
              }}</Label>
              <FormSelect v-model="epochUnit" :options="unitOptions" />
            </div>
          </div>
          <p v-if="epochError" class="text-[10.5px] text-error">{{ epochError }}</p>
          <div v-if="epochResult" class="space-y-1">
            <div
              v-for="item in [
                { label: t('devTools.timestamp.local'), value: epochResult.local },
                { label: 'UTC', value: epochResult.utc },
                { label: 'ISO 8601', value: epochResult.iso },
                { label: 'Epoch (s)', value: epochResult.seconds },
                { label: 'Epoch (ms)', value: epochResult.milliseconds },
              ]"
              :key="item.label"
              class="flex items-center gap-2 rounded-md bg-overlay-1 px-2 py-1"
            >
              <span class="w-24 shrink-0 text-[10px] text-muted-foreground">{{ item.label }}</span>
              <span class="numeric min-w-0 flex-1 truncate text-[11px]">{{ item.value }}</span>
              <button
                type="button"
                class="flex shrink-0 items-center gap-1 text-muted-foreground hover:text-foreground"
                :aria-label="t('tools.copy')"
                @click="copy(item.label, item.value)"
              >
                <Check v-if="isCopied(item.label)" class="size-3 text-signal" />
                <Copy v-else class="size-3" />
                <span v-if="isCopied(item.label)" class="text-[9px] text-signal">{{
                  t("common.copiedShort")
                }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
          <p class="text-[11px] font-medium">{{ t("devTools.timestamp.dateTitle") }}</p>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.timestamp.dateInput")
            }}</Label>
            <Input
              v-model="dateInput"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              placeholder="2026-01-01T00:00:00Z"
            />
          </div>
          <p v-if="dateError" class="text-[10.5px] text-error">{{ dateError }}</p>
          <div v-if="dateResult" class="space-y-1">
            <div
              v-for="item in [
                { label: 'Epoch (s)', value: dateResult.seconds },
                { label: 'Epoch (ms)', value: dateResult.milliseconds },
                { label: t('devTools.timestamp.local'), value: dateResult.local },
                { label: 'UTC', value: dateResult.utc },
                { label: 'ISO 8601', value: dateResult.iso },
              ]"
              :key="item.label"
              class="flex items-center gap-2 rounded-md bg-overlay-1 px-2 py-1"
            >
              <span class="w-24 shrink-0 text-[10px] text-muted-foreground">{{ item.label }}</span>
              <span class="numeric min-w-0 flex-1 truncate text-[11px]">{{ item.value }}</span>
              <button
                type="button"
                class="flex shrink-0 items-center gap-1 text-muted-foreground hover:text-foreground"
                :aria-label="t('tools.copy')"
                @click="copy(item.label, item.value)"
              >
                <Check v-if="isCopied(item.label)" class="size-3 text-signal" />
                <Copy v-else class="size-3" />
                <span v-if="isCopied(item.label)" class="text-[9px] text-signal">{{
                  t("common.copiedShort")
                }}</span>
              </button>
            </div>
          </div>
          <p class="text-[10.5px] leading-relaxed text-muted-foreground">
            {{ t("devTools.timestamp.dateHint") }}
          </p>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
