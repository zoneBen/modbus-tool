<script setup lang="ts">
import { Check, Copy, FolderOpen, Search, Square } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import { useToolEventBuffer } from "@/composables/useToolEventBuffer";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { useToolboxStore } from "@/stores/toolbox";
import { toolErrorText } from "@/lib/tool-error";
import type { SearchHit, ToolEvent } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const folder = ref("");
const query = ref("");
const mode = toolbox.toolPref<"literal" | "regex">("dev.search.mode", "literal");
const caseSensitive = toolbox.toolPref("dev.search.caseSensitive", false);
const include = toolbox.toolPref("dev.search.include", "*.ts, *.vue, *.md, *.json");
const exclude = toolbox.toolPref("dev.search.exclude", "");
const encoding = toolbox.toolPref<"auto" | "utf-8" | "utf-16le" | "utf-16be" | "gb18030">(
  "dev.search.encoding",
  "auto",
);
const maxFileSizeMb = toolbox.toolPref("dev.search.maxFileSize", "10");
const maxResults = toolbox.toolPref("dev.search.maxResults", "1000");
const skipHidden = toolbox.toolPref("dev.search.skipHidden", true);
const running = ref(false);
const hits = ref<SearchHit[]>([]);
const progress = ref({ files: 0, matches: 0 });
const done = ref<{ skipped: number; truncated: boolean; files: number; matches: number } | null>(
  null,
);
const error = ref<string | null>(null);

function onEvent(event: ToolEvent): void {
  if (event.event === "searchProgress") {
    progress.value = { files: event.files, matches: event.matches };
  } else if (event.event === "searchHits") {
    hits.value = [...hits.value, ...event.hits];
  } else if (event.event === "searchDone") {
    running.value = false;
    unbind();
    done.value = {
      skipped: event.skipped,
      truncated: event.truncated,
      files: event.files,
      matches: event.matches,
    };
    if (event.error && event.error.code !== "cancelled") {
      error.value = toolErrorText(t, event.error);
    }
  }
}

const { taskId, bind, unbind } = useToolEventBuffer(onEvent);

const modeOptions = computed(() => [
  { label: t("devTools.search.literal"), value: "literal" as const },
  { label: t("devTools.search.regex"), value: "regex" as const },
]);
const encodingOptions = ["auto", "utf-8", "utf-16le", "utf-16be", "gb18030"].map((value) => ({
  label: value.toUpperCase(),
  value,
}));

async function browse(): Promise<void> {
  const path = await ipc.pickDirectory();
  if (path) folder.value = path;
}

async function start(): Promise<void> {
  if (running.value) return;
  if (!folder.value.trim()) {
    error.value = t("devTools.search.noFolder");
    return;
  }
  if (!query.value.trim()) {
    error.value = t("devTools.search.noQuery");
    return;
  }
  error.value = null;
  hits.value = [];
  done.value = null;
  progress.value = { files: 0, matches: 0 };
  running.value = true;
  try {
    bind(
      await ipc.toolSearchStart({
        folder: folder.value.trim(),
        query: query.value,
        mode: mode.value,
        caseSensitive: caseSensitive.value,
        include: include.value,
        exclude: exclude.value,
        encoding: encoding.value,
        maxFileSize: Math.min(1024, Math.max(1, Number(maxFileSizeMb.value) || 10)) * 1024 * 1024,
        maxResults: Math.min(10_000, Math.max(1, Number(maxResults.value) || 1000)),
        skipHidden: skipHidden.value,
      }),
    );
  } catch (cause: unknown) {
    running.value = false;
    unbind();
    error.value = toolErrorText(t, cause);
  }
}

async function cancel(): Promise<void> {
  const current = taskId.value;
  if (current) await ipc.toolTaskCancel(current);
}

const { copy: copyFeedback, isCopied } = useCopyFeedback();

async function copy(value: string): Promise<void> {
  if (!value) return;
  await copyFeedback("hits", value);
}

function hitLabel(hit: SearchHit): string {
  return `${hit.path}:${hit.line}:${hit.column}`;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.search.title')"
    :description="t('devTools.tools.search.description')"
    :icon="Search"
  >
    <div class="mx-auto max-w-[1120px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.search.folder")
            }}</Label>
            <div class="flex gap-2">
              <Input
                v-model="folder"
                class="numeric h-9 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
              <Button
                variant="outline"
                size="sm"
                class="h-9 shrink-0 gap-1 border-hairline-strong bg-transparent text-[11px]"
                @click="browse"
              >
                <FolderOpen class="size-3.5" />
                {{ t("devTools.search.browse") }}
              </Button>
            </div>
          </div>
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.search.query")
            }}</Label>
            <Input
              v-model="query"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              @keydown.enter="start"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("devTools.search.mode") }}</Label>
            <FormSelect v-model="mode" :options="modeOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.search.encoding")
            }}</Label>
            <FormSelect v-model="encoding" :options="encodingOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.search.maxSize")
            }}</Label>
            <Input
              v-model="maxFileSizeMb"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.search.maxResults")
            }}</Label>
            <Input
              v-model="maxResults"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.search.include")
            }}</Label>
            <Input
              v-model="include"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("devTools.search.exclude")
            }}</Label>
            <Input
              v-model="exclude"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-4">
          <label class="flex items-center gap-2 text-[11px] text-muted-foreground">
            <Switch v-model="caseSensitive" />
            {{ t("devTools.search.caseSensitive") }}
          </label>
          <label class="flex items-center gap-2 text-[11px] text-muted-foreground">
            <Switch v-model="skipHidden" />
            {{ t("devTools.search.skipHidden") }}
          </label>
          <div class="ml-auto flex items-center gap-2">
            <Button v-if="!running" size="sm" class="h-8 gap-1.5 text-[12px]" @click="start">
              <Search class="size-3.5" />
              {{ t("devTools.search.start") }}
            </Button>
            <Button
              v-else
              variant="outline"
              size="sm"
              class="h-8 gap-1.5 border-warn/25 bg-warn/10 text-[12px] text-warn"
              @click="cancel"
            >
              <Square class="size-3" />
              {{ t("devTools.search.cancel") }}
            </Button>
          </div>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="running || done || hits.length" class="rounded-xl border border-hairline">
        <div
          class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2 text-[10.5px] text-muted-foreground"
        >
          <span>{{ t("devTools.search.progress") }}</span>
          <span class="numeric">{{ progress.files }} {{ t("devTools.search.files") }}</span>
          <span class="numeric">{{ progress.matches }} {{ t("devTools.search.matches") }}</span>
          <span v-if="done" class="numeric">
            · {{ t("devTools.search.skipped", { count: done.skipped }) }}
            <template v-if="done.truncated"> · {{ t("devTools.search.truncated") }}</template>
          </span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-6 gap-1 px-1.5 text-[10.5px]"
            :disabled="!hits.length"
            @click="copy(hits.map(hitLabel).join('\n'))"
          >
            <Check v-if="isCopied('hits')" class="size-3 text-signal" />
            <Copy v-else class="size-3" />
            {{ isCopied("hits") ? t("common.copiedShort") : t("tools.copy") }}
          </Button>
        </div>
        <div v-if="hits.length" class="overflow-x-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("devTools.search.file") }}
                </th>
                <th class="px-2 py-2 text-right font-medium text-muted-foreground">
                  {{ t("devTools.search.location") }}
                </th>
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("devTools.search.preview") }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(hit, index) in hits"
                :key="`${hit.path}-${hit.line}-${hit.column}-${index}`"
                class="border-b border-hairline last:border-0"
              >
                <td
                  class="numeric max-w-[420px] truncate px-3 py-1.5 text-muted-foreground"
                  :title="hit.path"
                >
                  {{ hit.path }}
                </td>
                <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                  {{ hit.line }}:{{ hit.column }}
                </td>
                <td class="px-3 py-1.5 text-foreground">
                  <span class="block max-w-[620px] truncate" :title="hit.preview">{{
                    hit.preview
                  }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p v-else-if="done" class="px-3 py-6 text-center text-[11px] text-muted-foreground">
          {{ t("devTools.search.noHits") }}
        </p>
      </div>

      <div
        v-else-if="!running"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("devTools.search.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
