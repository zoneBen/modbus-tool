<script setup lang="ts">
import { Check, Copy, Regex } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { useToolboxStore } from "@/stores/toolbox";

interface MatchRow {
  index: number;
  text: string;
  groups: Array<{ index: number; value: string | undefined }>;
}

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const pattern = toolbox.toolPref("dev.regex.pattern", "\\b\\w+@\\w+\\.\\w+\\b");
const flags = toolbox.toolPref("dev.regex.flags", "gi");
const text = toolbox.toolPref(
  "dev.regex.text",
  "Contact: alice@example.com\nSupport: support@demo.org\nNo match here",
);

const templates = computed(() => [
  { label: t("devTools.regex.templates.email"), pattern: "\\b[\\w.+-]+@[\\w-]+\\.[\\w.]+\\b" },
  {
    label: t("devTools.regex.templates.ip"),
    pattern: "\\b(?:(?:25[0-5]|2[0-4]\\d|1?\\d?\\d)\\.){3}(?:25[0-5]|2[0-4]\\d|1?\\d?\\d)\\b",
  },
  { label: t("devTools.regex.templates.date"), pattern: "\\b\\d{4}-\\d{2}-\\d{2}\\b" },
  {
    label: t("devTools.regex.templates.log"),
    pattern: "\\b(?:TRACE|DEBUG|INFO|WARN|ERROR|FATAL)\\b",
  },
]);

const compiled = computed<{ regex?: RegExp; error?: string }>(() => {
  try {
    return { regex: new RegExp(pattern.value, flags.value) };
  } catch (error: unknown) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
});

const matches = computed<MatchRow[]>(() => {
  const regex = compiled.value.regex;
  if (!regex || !pattern.value) return [];
  const rows: MatchRow[] = [];
  const globalRegex = new RegExp(
    regex.source,
    regex.flags.includes("g") ? regex.flags : `${regex.flags}g`,
  );
  let match: RegExpExecArray | null;
  let guard = 0;
  while ((match = globalRegex.exec(text.value)) && guard < 1000) {
    rows.push({
      index: match.index,
      text: match[0],
      groups: match.slice(1).map((value, index) => ({ index: index + 1, value })),
    });
    if (match[0] === "") globalRegex.lastIndex += 1;
    guard += 1;
  }
  return rows;
});

const highlightedSegments = computed(() => {
  const segments: Array<{ text: string; match: boolean }> = [];
  let cursor = 0;
  for (const match of matches.value) {
    if (match.index < cursor) continue;
    if (match.index > cursor) {
      segments.push({ text: text.value.slice(cursor, match.index), match: false });
    }
    segments.push({ text: match.text, match: true });
    cursor = match.index + match.text.length;
  }
  segments.push({ text: text.value.slice(cursor), match: false });
  return segments;
});

function applyTemplate(value: string): void {
  pattern.value = value;
  flags.value = "gi";
}

function toggleFlag(flag: string): void {
  flags.value = flags.value.includes(flag)
    ? flags.value.replace(flag, "")
    : `${flags.value}${flag}`;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.regex.title')"
    :description="t('devTools.tools.regex.description')"
    :icon="Regex"
  >
    <div class="mx-auto max-w-[1080px] space-y-4">
      <div class="space-y-2 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="flex flex-wrap items-center gap-2">
          <span class="numeric text-[13px] text-muted-foreground">/</span>
          <Input
            v-model="pattern"
            class="numeric h-9 min-w-[240px] flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
            placeholder="\\b\\w+\\b"
          />
          <span class="numeric text-[13px] text-muted-foreground">/{{ flags }}</span>
          <div class="flex gap-1">
            <button
              v-for="flag in ['g', 'i', 'm', 's', 'u']"
              :key="flag"
              type="button"
              class="numeric size-6 rounded border text-[10px]"
              :class="
                flags.includes(flag)
                  ? 'border-signal/40 bg-signal/10 text-signal'
                  : 'border-hairline bg-well text-muted-foreground'
              "
              @click="toggleFlag(flag)"
            >
              {{ flag }}
            </button>
          </div>
        </div>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="template in templates"
            :key="template.label"
            type="button"
            class="rounded-full border border-hairline bg-well px-2 py-0.5 text-[10px] text-muted-foreground transition-colors hover:border-signal/30 hover:text-signal"
            @click="applyTemplate(template.pattern)"
          >
            {{ template.label }}
          </button>
        </div>
        <p v-if="compiled.error" class="text-[11px] text-error">{{ compiled.error }}</p>
      </div>

      <div class="grid gap-3 lg:grid-cols-2">
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.regex.text") }}</Label>
          <textarea
            v-model="text"
            class="scrollbar-thin numeric h-[280px] w-full resize-none rounded-xl border border-hairline bg-well px-3 py-2.5 text-[12px] leading-relaxed outline-none focus:border-signal/50"
          />
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="mb-1.5 text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("devTools.regex.highlight") }}
            </p>
            <p class="numeric whitespace-pre-wrap break-all text-[11.5px] leading-relaxed">
              <span
                v-for="(segment, index) in highlightedSegments"
                :key="index"
                :class="segment.match ? 'rounded bg-signal/20 px-0.5 text-signal' : ''"
                >{{ segment.text }}</span
              >
            </p>
          </div>
        </div>

        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("devTools.regex.matches", { count: matches.length }) }}
            </Label>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 gap-1 px-1.5 text-[10.5px]"
              :disabled="!matches.length"
              @click="copy('matches', matches.map((match) => match.text).join('\n'))"
            >
              <Check v-if="isCopied('matches')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("matches") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
          <div class="scrollbar-thin max-h-[360px] overflow-auto rounded-xl border border-hairline">
            <div
              v-for="(match, index) in matches"
              :key="`${match.index}-${index}`"
              class="border-b border-hairline px-3 py-2 last:border-0"
            >
              <div class="flex items-center gap-2">
                <span
                  class="numeric rounded bg-overlay-1 px-1.5 py-0.5 text-[9px] text-muted-foreground"
                >
                  #{{ index + 1 }} @ {{ match.index }}
                </span>
                <span class="numeric min-w-0 flex-1 break-all text-[11.5px] text-signal">
                  {{ match.text }}
                </span>
              </div>
              <div v-if="match.groups.length" class="mt-1 flex flex-wrap gap-1">
                <span
                  v-for="group in match.groups"
                  :key="group.index"
                  class="numeric rounded border border-hairline bg-well px-1.5 py-0.5 text-[9.5px] text-muted-foreground"
                >
                  ${{ group.index }} = {{ group.value ?? "—" }}
                </span>
              </div>
            </div>
            <p
              v-if="!matches.length"
              class="px-3 py-8 text-center text-[11px] text-muted-foreground"
            >
              {{ t("devTools.regex.noMatches") }}
            </p>
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
