<script setup lang="ts">
import { Check, Copy, FileDiff } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { diffLines, diffStats } from "@/lib/diff";
import { useToolboxStore } from "@/stores/toolbox";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const left = toolbox.toolPref("dev.diff.left", "alpha\nbravo\ncharlie\ndelta");
const right = toolbox.toolPref("dev.diff.right", "alpha\nbravo\nCHARLIE\ndelta\necho");

const result = computed(() => {
  try {
    const lines = diffLines(left.value, right.value);
    return { lines, stats: diffStats(lines), error: null };
  } catch (error: unknown) {
    return {
      lines: [],
      stats: { added: 0, removed: 0, same: 0 },
      error: error instanceof Error ? error.message : String(error),
    };
  }
});

const diffText = computed(() =>
  result.value.lines
    .filter((line) => line.kind !== "same")
    .map((line) => `${line.kind === "add" ? "+" : "-"} ${line.text}`)
    .join("\n"),
);

const tone: Record<string, string> = {
  add: "bg-signal/10 text-signal",
  remove: "bg-error/10 text-error",
  same: "text-muted-foreground",
};
const marker: Record<string, string> = { add: "+", remove: "-", same: " " };
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.diff.title')"
    :description="t('devTools.tools.diff.description')"
    :icon="FileDiff"
  >
    <div class="mx-auto max-w-[1180px] space-y-3">
      <div class="grid gap-3 lg:grid-cols-2">
        <div class="space-y-1">
          <p class="text-[11px] font-medium">{{ t("devTools.diff.left") }}</p>
          <textarea
            v-model="left"
            class="scrollbar-thin numeric h-[200px] w-full resize-y rounded-xl border border-hairline bg-well px-3 py-2 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
          />
        </div>
        <div class="space-y-1">
          <p class="text-[11px] font-medium">{{ t("devTools.diff.right") }}</p>
          <textarea
            v-model="right"
            class="scrollbar-thin numeric h-[200px] w-full resize-y rounded-xl border border-hairline bg-well px-3 py-2 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
          />
        </div>
      </div>

      <div
        class="flex flex-wrap items-center gap-2 rounded-xl border border-hairline bg-well-soft px-3 py-2"
      >
        <FileDiff class="size-3.5 text-muted-foreground" />
        <span class="text-[11px] text-muted-foreground">
          {{
            t("devTools.diff.stats", { added: result.stats.added, removed: result.stats.removed })
          }}
        </span>
        <Button
          variant="ghost"
          size="sm"
          class="ml-auto h-7 gap-1 text-[10.5px] text-muted-foreground"
          :disabled="!diffText"
          @click="copy('diff', diffText)"
        >
          <Check v-if="isCopied('diff')" class="size-3 text-signal" />
          <Copy v-else class="size-3" />
          {{ isCopied("diff") ? t("common.copiedShort") : t("tools.copy") }}
        </Button>
      </div>

      <p
        v-if="result.error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ t("devTools.diff.tooLarge") }}
      </p>

      <div
        v-else
        class="scrollbar-thin max-h-[420px] overflow-auto rounded-xl border border-hairline"
      >
        <div
          v-for="(line, index) in result.lines"
          :key="index"
          class="flex items-start gap-2 border-b border-hairline/60 px-2 py-0.5 last:border-0"
          :class="tone[line.kind]"
        >
          <span class="numeric w-8 shrink-0 text-right text-[9.5px] opacity-60">
            {{ line.leftNo ?? "" }}
          </span>
          <span class="numeric w-8 shrink-0 text-right text-[9.5px] opacity-60">
            {{ line.rightNo ?? "" }}
          </span>
          <span class="numeric w-3 shrink-0 text-center text-[10px] font-semibold">
            {{ marker[line.kind] }}
          </span>
          <span class="numeric min-w-0 flex-1 whitespace-pre-wrap break-all text-[11px]">
            {{ line.text || " " }}
          </span>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
