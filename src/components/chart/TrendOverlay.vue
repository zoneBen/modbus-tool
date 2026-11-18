<script setup lang="ts">
import { ArrowLeft, LineChart } from "@lucide/vue";
import { computed, onBeforeUnmount, watch } from "vue";
import { useI18n } from "vue-i18n";
import ChartPanel from "@/components/chart/ChartPanel.vue";
import { Button } from "@/components/ui/button";
import { formatRange } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";

const open = defineModel<boolean>("open", { default: false });
const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const definition = computed(() => sessions.activeDefinition);
const range = computed(() =>
  definition.value
    ? formatRange(definition.value.startAddress, definition.value.quantity, settings.addressBase)
    : "—",
);

function close(): void {
  open.value = false;
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") close();
}

watch(
  open,
  (isOpen) => {
    if (isOpen) window.addEventListener("keydown", onKeydown);
    else window.removeEventListener("keydown", onKeydown);
  },
  { immediate: true },
);

onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="animate-enter fixed inset-0 z-40 flex flex-col bg-background"
      role="dialog"
      aria-modal="true"
      :aria-label="t('chart.trendTitle')"
    >
      <header class="flex h-14 shrink-0 items-center gap-3 border-b border-hairline px-4">
        <Button variant="outline" size="sm" class="h-8 gap-1.5 text-[12px]" @click="close">
          <ArrowLeft class="size-3.5" />
          {{ t("chart.back") }}
        </Button>
        <div class="min-w-0">
          <p class="truncate text-[13px] font-semibold">
            {{ definition?.name ?? t("chart.trendTitle") }}
          </p>
          <p class="numeric truncate text-[11px] text-muted-foreground">
            {{ sessions.activeSession?.name ?? "—" }} · {{ range }}
          </p>
        </div>
        <div class="ml-auto flex items-center gap-2 text-[11px] text-muted-foreground">
          <span class="numeric rounded border border-hairline bg-well px-1.5 py-0.5 uppercase">
            {{ definition?.dataType.toUpperCase() ?? "—" }}
          </span>
          <span
            class="status-dot"
            :class="sessions.activeSnapshot?.error ? 'text-error' : 'text-signal'"
          />
          <span>{{
            sessions.activeSnapshot?.error ? t("chart.readError") : t("chart.liveTrend")
          }}</span>
        </div>
      </header>

      <div class="flex min-h-0 flex-1 items-center justify-center p-3">
        <div v-if="definition" class="glass h-full w-full overflow-hidden rounded-xl">
          <ChartPanel class="h-full" />
        </div>
        <div v-else class="flex flex-col items-center gap-2 text-center">
          <LineChart class="size-6 text-muted-foreground" />
          <p class="text-[13px] text-foreground">{{ t("chart.noDefinitionTitle") }}</p>
          <p class="text-[11px] text-muted-foreground">
            {{ t("chart.noDefinitionHint") }}
          </p>
        </div>
      </div>
    </div>
  </Teleport>
</template>
