<script setup lang="ts">
import { Check, FileEdit, FolderOpen, Play, RefreshCw } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { FsEntry, RenameItem } from "@/types/tools";

type RenameMode = "regex" | "prefix" | "suffix" | "sequence";

const { t } = useI18n();
const toolbox = useToolboxStore();
const directory = toolbox.toolPref("security.rename.directory", "");
const recursive = toolbox.toolPref("security.rename.recursive", false);
const mode = toolbox.toolPref<RenameMode>("security.rename.mode", "regex");
const pattern = toolbox.toolPref("security.rename.pattern", "\\.log$");
const replacement = toolbox.toolPref("security.rename.replacement", ".txt");
const affix = toolbox.toolPref("security.rename.affix", "backup_");
const sequenceBase = toolbox.toolPref("security.rename.sequenceBase", "file_");
const sequenceStart = toolbox.toolPref("security.rename.sequenceStart", "1");
const sequencePad = toolbox.toolPref("security.rename.sequencePad", "3");
const entries = ref<FsEntry[]>([]);
const selected = ref<Set<string>>(new Set());
const busy = ref(false);
const error = ref<string | null>(null);
const items = ref<RenameItem[]>([]);
const applied = ref(false);

const modeOptions = computed(() => [
  { value: "regex" as const, label: t("securityTools.rename.modes.regex") },
  { value: "prefix" as const, label: t("securityTools.rename.modes.prefix") },
  { value: "suffix" as const, label: t("securityTools.rename.modes.suffix") },
  { value: "sequence" as const, label: t("securityTools.rename.modes.sequence") },
]);

const files = computed(() => entries.value.filter((entry) => !entry.isDir));

function splitName(name: string): { base: string; extension: string } {
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return { base: name, extension: "" };
  return { base: name.slice(0, dot), extension: name.slice(dot) };
}

const preview = computed(() => {
  let regex: RegExp | null = null;
  if (mode.value === "regex") {
    try {
      regex = new RegExp(pattern.value);
    } catch {
      regex = null;
    }
  }
  const start = Math.max(0, Number(sequenceStart.value) || 1);
  const pad = Math.max(1, Math.min(8, Number(sequencePad.value) || 3));
  const targets = new Set<string>();
  return files.value.map((entry, index) => {
    const { base, extension } = splitName(entry.name);
    let next = entry.name;
    if (mode.value === "regex" && regex) next = entry.name.replace(regex, replacement.value);
    if (mode.value === "prefix") next = `${affix.value}${entry.name}`;
    if (mode.value === "suffix") next = `${base}${affix.value}${extension}`;
    if (mode.value === "sequence") {
      next = `${sequenceBase.value}${String(start + index).padStart(pad, "0")}${extension}`;
    }
    let status: RenameItem["status"] = "ok";
    if (next === entry.name) status = "skipped";
    else if (next.includes("/") || next.includes("\\")) status = "invalid";
    else if (targets.has(next)) status = "conflict";
    else if (files.value.some((other) => other.name === next)) status = "conflict";
    targets.add(next);
    return {
      from: entry.path,
      to: next === entry.name ? entry.path : entry.path.replace(/[^/\\]+$/, next),
      status,
      detail: status === "ok" ? undefined : status,
      name: entry.name,
      nextName: next,
      selected: selected.value.has(entry.path),
    };
  });
});

const actionable = computed(() =>
  preview.value.filter((item) => item.selected && item.status === "ok"),
);

async function chooseDirectory(): Promise<void> {
  const path = await ipc.pickDirectory();
  if (path) {
    directory.value = path;
    items.value = [];
    applied.value = false;
    await load();
  }
}

async function load(): Promise<void> {
  if (!directory.value.trim()) return;
  busy.value = true;
  error.value = null;
  try {
    const result = await ipc.toolFsList({
      path: directory.value.trim(),
      recursive: recursive.value,
      limit: 2000,
    });
    entries.value = result.entries;
    selected.value = new Set(
      result.entries.filter((entry) => !entry.isDir).map((entry) => entry.path),
    );
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    entries.value = [];
  } finally {
    busy.value = false;
  }
}

function toggle(path: string): void {
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  selected.value = next;
}

function toggleAll(): void {
  selected.value =
    selected.value.size === files.value.length
      ? new Set()
      : new Set(files.value.map((entry) => entry.path));
}

async function apply(): Promise<void> {
  if (!actionable.value.length) return;
  busy.value = true;
  error.value = null;
  try {
    const result = await ipc.toolFsRename({
      pairs: actionable.value.map((item) => ({ from: item.from, to: item.to })),
      dryRun: false,
    });
    await load();
    items.value = result.items;
    applied.value = true;
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('securityTools.tools.rename.title')"
    :description="t('securityTools.tools.rename.description')"
    :icon="FileEdit"
  >
    <div class="mx-auto max-w-[1120px] space-y-4">
      <div class="space-y-2 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="flex flex-wrap items-end gap-2">
          <div class="min-w-[280px] flex-1 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("securityTools.rename.directory")
            }}</Label>
            <div class="flex gap-1.5">
              <Input
                v-model="directory"
                class="numeric h-9 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
              <Button
                variant="outline"
                size="sm"
                class="h-9 shrink-0 border-hairline-strong bg-transparent"
                @click="chooseDirectory"
              >
                <FolderOpen class="size-3.5" />
              </Button>
              <Button size="sm" class="h-9 gap-1 text-[11px]" :disabled="busy" @click="load">
                <RefreshCw class="size-3.5" />
                {{ t("securityTools.rename.load") }}
              </Button>
            </div>
          </div>
          <label class="flex items-center gap-2 pb-2 text-[11px]">
            <Switch v-model="recursive" />
            {{ t("securityTools.rename.recursive") }}
          </label>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("securityTools.rename.mode")
            }}</Label>
            <div class="w-40">
              <FormSelect v-model="mode" :options="modeOptions" />
            </div>
          </div>
        </div>

        <div class="grid gap-2 sm:grid-cols-3">
          <template v-if="mode === 'regex'">
            <div class="space-y-1 sm:col-span-2">
              <Label class="text-[10px] text-muted-foreground">{{
                t("securityTools.rename.pattern")
              }}</Label>
              <Input
                v-model="pattern"
                class="numeric h-8 rounded-md border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10px] text-muted-foreground">{{
                t("securityTools.rename.replacement")
              }}</Label>
              <Input
                v-model="replacement"
                class="numeric h-8 rounded-md border-hairline-strong bg-well text-[11px]"
              />
            </div>
          </template>
          <template v-else-if="mode === 'prefix' || mode === 'suffix'">
            <div class="space-y-1 sm:col-span-3">
              <Label class="text-[10px] text-muted-foreground">{{
                t("securityTools.rename.affix")
              }}</Label>
              <Input
                v-model="affix"
                class="numeric h-8 rounded-md border-hairline-strong bg-well text-[11px]"
              />
            </div>
          </template>
          <template v-else>
            <div class="space-y-1">
              <Label class="text-[10px] text-muted-foreground">{{
                t("securityTools.rename.base")
              }}</Label>
              <Input
                v-model="sequenceBase"
                class="numeric h-8 rounded-md border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10px] text-muted-foreground">{{
                t("securityTools.rename.start")
              }}</Label>
              <Input
                v-model="sequenceStart"
                class="numeric h-8 rounded-md border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1">
              <Label class="text-[10px] text-muted-foreground">{{
                t("securityTools.rename.pad")
              }}</Label>
              <Input
                v-model="sequencePad"
                class="numeric h-8 rounded-md border-hairline-strong bg-well text-[11px]"
              />
            </div>
          </template>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ error }}
      </p>

      <div v-if="files.length" class="rounded-xl border border-hairline">
        <div class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2">
          <button
            type="button"
            class="text-[10.5px] text-muted-foreground hover:text-foreground"
            @click="toggleAll"
          >
            {{
              selected.size === files.length
                ? t("securityTools.rename.none")
                : t("securityTools.rename.all")
            }}
          </button>
          <span class="text-[10.5px] text-muted-foreground">
            {{ t("securityTools.rename.selected", { count: selected.size }) }}
          </span>
          <span
            v-if="preview.some((item) => item.status === 'conflict')"
            class="rounded border border-warn/25 bg-warn/10 px-1.5 py-0.5 text-[9.5px] text-warn"
          >
            {{ t("securityTools.rename.conflicts") }}
          </span>
          <Button
            size="sm"
            class="ml-auto h-7 gap-1 text-[10.5px]"
            :disabled="busy || !actionable.length"
            @click="apply"
          >
            <Play class="size-3" />
            {{ t("securityTools.rename.apply", { count: actionable.length }) }}
          </Button>
        </div>

        <div class="scrollbar-thin max-h-[440px] overflow-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="sticky top-0 bg-surface">
              <tr class="border-b border-hairline text-left text-muted-foreground">
                <th class="w-8 px-2 py-2"></th>
                <th class="px-2 py-2 font-medium">{{ t("securityTools.rename.current") }}</th>
                <th class="px-2 py-2 font-medium">{{ t("securityTools.rename.next") }}</th>
                <th class="w-24 px-3 py-2 font-medium"></th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in preview"
                :key="item.from"
                class="border-b border-hairline/60 last:border-0"
                :class="item.selected ? '' : 'opacity-55'"
              >
                <td class="px-2 py-1.5">
                  <input
                    type="checkbox"
                    :checked="item.selected"
                    class="size-3.5 accent-signal"
                    @change="toggle(item.from)"
                  />
                </td>
                <td class="numeric px-2 py-1.5 text-muted-foreground">{{ item.name }}</td>
                <td class="numeric px-2 py-1.5">
                  <span :class="item.status === 'ok' ? 'text-foreground' : 'text-warn'">{{
                    item.nextName
                  }}</span>
                </td>
                <td class="px-3 py-1.5 text-right">
                  <span
                    v-if="item.status !== 'ok'"
                    class="rounded border px-1 py-0.5 text-[9px]"
                    :class="
                      item.status === 'conflict'
                        ? 'border-warn/25 bg-warn/10 text-warn'
                        : 'border-hairline text-muted-foreground'
                    "
                  >
                    {{ t(`securityTools.rename.status.${item.status}`) }}
                  </span>
                  <Check v-else-if="item.selected" class="ml-auto size-3.5 text-signal" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div
        v-else-if="!busy"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("securityTools.rename.hint") }}
      </div>

      <div v-if="items.length" class="rounded-xl border border-hairline bg-well-soft p-3">
        <p class="text-[11px] font-medium">
          {{ applied ? t("securityTools.rename.applied") : t("securityTools.rename.preview") }}
        </p>
        <div class="mt-1 space-y-0.5">
          <p
            v-for="(item, index) in items"
            :key="index"
            class="numeric text-[10px] text-muted-foreground"
          >
            {{ item.from }} → {{ item.to }}
            <span :class="item.status === 'ok' ? 'text-signal' : 'text-warn'"
              >[{{ item.status }}]</span
            >
          </p>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
