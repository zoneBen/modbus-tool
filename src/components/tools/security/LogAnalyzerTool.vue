<script setup lang="ts">
import { Check, Copy, FolderOpen, ScrollText, Trash2 } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { analyzeLogs } from "@/lib/log-analyze";
import type { LogLevel } from "@/lib/log-analyze";
import { useToolboxStore } from "@/stores/toolbox";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const text = ref("");
const query = toolbox.toolPref("security.log.query", "");
const level = toolbox.toolPref<LogLevel | "ALL">("security.log.level", "ALL");
const caseSensitive = toolbox.toolPref("security.log.caseSensitive", false);
const busy = ref(false);

const levelOptions = computed(() => [
  { value: "ALL" as const, label: t("securityTools.log.levels.ALL") },
  ...(["ERROR", "FATAL", "WARN", "INFO", "DEBUG", "TRACE", "UNKNOWN"] as LogLevel[]).map(
    (value) => ({ value, label: value }),
  ),
]);

const analysis = computed(() =>
  analyzeLogs(text.value, {
    query: query.value,
    level: level.value,
    caseSensitive: caseSensitive.value,
    limit: 3000,
  }),
);

const levelTone: Record<LogLevel, string> = {
  FATAL: "border-error/30 bg-error/10 text-error",
  ERROR: "border-error/25 bg-error/[0.07] text-error",
  WARN: "border-warn/25 bg-warn/10 text-warn",
  INFO: "border-rx/25 bg-rx/10 text-rx",
  DEBUG: "border-sim/25 bg-sim/10 text-sim",
  TRACE: "border-hairline bg-well text-muted-foreground",
  UNKNOWN: "border-hairline bg-well text-muted-foreground",
};

const maxBucket = computed(() =>
  Math.max(1, ...analysis.value.timeline.map((bucket) => bucket.count)),
);

async function loadFile(): Promise<void> {
  const path = await ipc.pickFile([
    { name: "Log files", extensions: ["log", "txt", "out", "err"] },
    { name: "All files", extensions: ["*"] },
  ]);
  if (!path) return;
  busy.value = true;
  try {
    text.value = await ipc.readTextFile(path);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('securityTools.tools.log.title')"
    :description="t('securityTools.tools.log.description')"
    :icon="ScrollText"
  >
    <div class="mx-auto max-w-[1180px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="min-w-[220px] flex-1 space-y-1.5">
          <p class="text-[11px] text-muted-foreground">{{ t("securityTools.log.query") }}</p>
          <Input
            v-model="query"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            :placeholder="t('securityTools.log.queryPlaceholder')"
          />
        </div>
        <div class="space-y-1.5">
          <p class="text-[11px] text-muted-foreground">{{ t("securityTools.log.level") }}</p>
          <div class="w-32">
            <FormSelect v-model="level" :options="levelOptions" />
          </div>
        </div>
        <label class="flex items-center gap-2 pb-2 text-[11px]">
          <input v-model="caseSensitive" type="checkbox" class="size-3.5 accent-signal" />
          Aa
        </label>
        <Button
          variant="outline"
          size="sm"
          class="h-9 gap-1.5 border-hairline-strong bg-transparent text-[11px]"
          :disabled="busy"
          @click="loadFile"
        >
          <FolderOpen class="size-3.5" />
          {{ t("securityTools.log.loadFile") }}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-9 gap-1 text-[11px] text-muted-foreground"
          @click="text = ''"
        >
          <Trash2 class="size-3.5" />
          {{ t("securityTools.log.clear") }}
        </Button>
      </div>

      <textarea
        v-model="text"
        class="scrollbar-thin numeric h-[160px] w-full resize-y rounded-xl border border-hairline bg-well px-3 py-2.5 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
        :placeholder="t('securityTools.log.pasteHint')"
      />

      <div class="grid gap-2 sm:grid-cols-4 lg:grid-cols-7">
        <div
          v-for="(count, key) in analysis.counts"
          :key="key"
          class="rounded-lg border px-2 py-1.5"
          :class="levelTone[key]"
        >
          <p class="text-[9px] font-medium uppercase">{{ key }}</p>
          <p class="numeric text-[16px] font-semibold leading-tight">{{ count }}</p>
        </div>
      </div>

      <div
        v-if="analysis.timeline.length"
        class="rounded-xl border border-hairline bg-well-soft p-3"
      >
        <p class="mb-2 text-[10px] uppercase tracking-wider text-muted-foreground">
          {{ t("securityTools.log.timeline") }}
        </p>
        <div class="flex h-16 items-end gap-0.5">
          <div
            v-for="bucket in analysis.timeline"
            :key="bucket.minute"
            class="group relative flex-1 rounded-t bg-rx/40 transition-colors hover:bg-rx/70"
            :style="{ height: `${Math.max(4, (bucket.count / maxBucket) * 100)}%` }"
            :title="`${bucket.minute} · ${bucket.count}`"
          />
        </div>
        <div class="mt-1 flex justify-between text-[9px] text-muted-foreground">
          <span class="numeric">{{ analysis.timeline[0]?.minute }}</span>
          <span class="numeric">{{ analysis.timeline[analysis.timeline.length - 1]?.minute }}</span>
        </div>
      </div>

      <div class="rounded-xl border border-hairline">
        <div class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2">
          <span class="text-[11px] text-muted-foreground">
            {{ t("securityTools.log.filtered", { count: analysis.filtered.length }) }}
          </span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
            :disabled="!analysis.filtered.length"
            @click="copy('logs', analysis.filtered.map((line) => line.text).join('\n'))"
          >
            <Check v-if="isCopied('logs')" class="size-3 text-signal" />
            <Copy v-else class="size-3" />
            {{ isCopied("logs") ? t("common.copiedShort") : t("tools.copy") }}
          </Button>
        </div>
        <div class="scrollbar-thin max-h-[420px] overflow-auto">
          <div
            v-for="line in analysis.filtered"
            :key="line.line"
            class="flex items-start gap-2 border-b border-hairline/60 px-3 py-1 last:border-0 hover:bg-overlay-1"
          >
            <span class="numeric w-12 shrink-0 text-right text-[9px] text-muted-foreground">{{
              line.line
            }}</span>
            <span
              class="shrink-0 rounded border px-1 text-[8.5px] leading-4"
              :class="levelTone[line.level]"
              >{{ line.level }}</span
            >
            <span class="numeric min-w-0 flex-1 whitespace-pre-wrap break-all text-[10.5px]">{{
              line.text
            }}</span>
          </div>
          <p
            v-if="!analysis.filtered.length"
            class="px-3 py-8 text-center text-[11px] text-muted-foreground"
          >
            {{ t("securityTools.log.noResult") }}
          </p>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
