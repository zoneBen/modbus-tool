<script setup lang="ts">
import { CalendarClock, Check, Copy } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { nextRuns, validateCron } from "@/lib/cron";
import { useToolboxStore } from "@/stores/toolbox";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const expression = toolbox.toolPref("dev.cron.expression", "*/5 * * * *");

const fields = computed(() => {
  const parts = expression.value.trim().split(/\s+/);
  return parts.length === 5 ? parts : ["*", "*", "*", "*", "*"];
});

const fieldDefs = computed(() => [
  { index: 0, label: t("devTools.cron.minute"), hint: "0-59" },
  { index: 1, label: t("devTools.cron.hour"), hint: "0-23" },
  { index: 2, label: t("devTools.cron.day"), hint: "1-31" },
  { index: 3, label: t("devTools.cron.month"), hint: "1-12" },
  { index: 4, label: t("devTools.cron.weekday"), hint: "0-6" },
]);

const presets = computed(() => [
  { label: t("devTools.cron.presets.everyMinute"), value: "* * * * *" },
  { label: t("devTools.cron.presets.every5"), value: "*/5 * * * *" },
  { label: t("devTools.cron.presets.hourly"), value: "0 * * * *" },
  { label: t("devTools.cron.presets.daily"), value: "0 2 * * *" },
  { label: t("devTools.cron.presets.weekly"), value: "0 3 * * 1" },
  { label: t("devTools.cron.presets.monthly"), value: "0 4 1 * *" },
]);

const error = computed(() => validateCron(expression.value));
const runs = computed<Date[]>(() => {
  if (error.value) return [];
  try {
    return nextRuns(expression.value, 6);
  } catch {
    return [];
  }
});

function updateField(index: number, value: string): void {
  const next = [...fields.value];
  next[index] = value.trim() || "*";
  expression.value = next.join(" ");
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.cron.title')"
    :description="t('devTools.tools.cron.description')"
    :icon="CalendarClock"
  >
    <div class="mx-auto max-w-[900px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="flex flex-wrap items-end gap-2">
          <div class="min-w-[220px] flex-1 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.cron.expression")
            }}</Label>
            <Input
              v-model="expression"
              class="numeric h-10 rounded-lg border-hairline-strong bg-well text-[15px] tracking-wide"
            />
          </div>
          <Button
            variant="outline"
            size="sm"
            class="h-10 gap-1.5 border-hairline-strong bg-transparent text-[11px]"
            :disabled="!!error"
            @click="copy('expression', expression)"
          >
            <Check v-if="isCopied('expression')" class="size-3.5 text-signal" />
            <Copy v-else class="size-3.5" />
            {{ isCopied("expression") ? t("common.copiedShort") : t("tools.copy") }}
          </Button>
        </div>

        <div class="grid grid-cols-5 gap-2">
          <div v-for="field in fieldDefs" :key="field.index" class="space-y-1">
            <Label class="text-[10px] text-muted-foreground">{{ field.label }}</Label>
            <Input
              :model-value="fields[field.index]"
              class="numeric h-8 rounded-md border-hairline-strong bg-well px-2 text-center text-[11px]"
              :placeholder="field.hint"
              @update:model-value="(value) => updateField(field.index, String(value))"
            />
          </div>
        </div>

        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="preset in presets"
            :key="preset.value"
            type="button"
            class="rounded-full border border-hairline bg-well px-2 py-0.5 text-[10px] text-muted-foreground transition-colors hover:border-signal/30 hover:text-signal"
            @click="expression = preset.value"
          >
            {{ preset.label }}
          </button>
        </div>

        <p
          v-if="error"
          class="rounded-lg border border-error/25 bg-error/10 px-2.5 py-1.5 text-[11px] text-error"
        >
          {{ t(`devTools.cron.errors.${error}`) }}
        </p>
      </div>

      <div v-if="!error" class="rounded-xl border border-hairline">
        <div class="border-b border-hairline px-3 py-2 text-[11px] font-medium">
          {{ t("devTools.cron.nextRuns") }}
        </div>
        <div class="divide-y divide-hairline">
          <div
            v-for="(run, index) in runs"
            :key="run.toISOString()"
            class="flex items-center gap-3 px-3 py-2"
          >
            <span class="numeric w-6 text-[10px] text-muted-foreground">{{ index + 1 }}</span>
            <span class="numeric text-[12px] text-foreground">{{ run.toLocaleString() }}</span>
            <span class="numeric ml-auto text-[10px] text-muted-foreground">
              {{ run.toISOString() }}
            </span>
          </div>
          <p v-if="!runs.length" class="px-3 py-6 text-center text-[11px] text-muted-foreground">
            {{ t("devTools.cron.noMatches") }}
          </p>
        </div>
      </div>

      <p class="text-[10.5px] leading-relaxed text-muted-foreground">
        {{ t("devTools.cron.hint") }}
      </p>
    </div>
  </ToolPanelFrame>
</template>
