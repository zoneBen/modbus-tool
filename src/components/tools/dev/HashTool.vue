<script setup lang="ts">
import { Copy, FolderOpen, Hash, Square } from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ipc } from "@/lib/ipc";
import { useToolboxStore } from "@/stores/toolbox";
import { toolErrorText } from "@/lib/tool-error";
import type { HashResult } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const filePath = ref("");
const algorithms = toolbox.toolPref<string[]>("dev.hash.algorithms", ["sha256"]);
const running = ref(false);
const taskId = ref<string | null>(null);
const progress = ref({ done: 0, total: 0 });
const result = ref<HashResult | null>(null);
const error = ref<string | null>(null);
let unlisten: (() => void) | null = null;

const options = [
  { value: "md5", label: "MD5" },
  { value: "sha1", label: "SHA-1" },
  { value: "sha256", label: "SHA-256" },
  { value: "sha512", label: "SHA-512" },
  { value: "crc32", label: "CRC32" },
];

const percent = computed(() =>
  progress.value.total > 0
    ? Math.min(100, Math.round((progress.value.done / progress.value.total) * 100))
    : running.value
      ? 0
      : 100,
);

onMounted(async () => {
  unlisten = await ipc.on("tool:event", (event) => {
    if (!taskId.value || event.taskId !== taskId.value) return;
    if (event.event === "hashProgress") {
      progress.value = { done: event.done, total: event.total };
    } else if (event.event === "hashDone") {
      running.value = false;
      taskId.value = null;
      if (event.error) {
        error.value = toolErrorText(t, event.error);
      } else if (event.result) {
        result.value = event.result;
      }
    }
  });
});

onBeforeUnmount(() => {
  unlisten?.();
  unlisten = null;
});

function toggleAlgorithm(value: string): void {
  algorithms.value = algorithms.value.includes(value)
    ? algorithms.value.filter((item) => item !== value)
    : [...algorithms.value, value];
}

async function browse(): Promise<void> {
  const path = await ipc.pickFile();
  if (path) filePath.value = path;
}

async function start(): Promise<void> {
  if (running.value) return;
  if (!filePath.value.trim()) {
    error.value = t("devTools.hash.noFile");
    return;
  }
  if (!algorithms.value.length) {
    error.value = t("devTools.hash.noAlgorithm");
    return;
  }
  error.value = null;
  result.value = null;
  progress.value = { done: 0, total: 0 };
  running.value = true;
  try {
    taskId.value = await ipc.toolHashStart({
      path: filePath.value.trim(),
      algorithms: algorithms.value,
    });
  } catch (cause: unknown) {
    running.value = false;
    error.value = toolErrorText(t, cause);
  }
}

async function cancel(): Promise<void> {
  const current = taskId.value;
  if (current) await ipc.toolTaskCancel(current);
}

async function copy(value: string): Promise<void> {
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    toast.success(t("common.copied"));
  } catch {
    toast.error(t("tools.copyFailed"));
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.hash.title')"
    :description="t('devTools.tools.hash.description')"
    :icon="Hash"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.hash.file") }}</Label>
          <div class="flex gap-2">
            <Input
              v-model="filePath"
              class="numeric h-9 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
              :placeholder="t('devTools.hash.filePlaceholder')"
            />
            <Button
              variant="outline"
              size="sm"
              class="h-9 shrink-0 gap-1 border-hairline-strong bg-transparent text-[11px]"
              @click="browse"
            >
              <FolderOpen class="size-3.5" />
              {{ t("devTools.hash.browse") }}
            </Button>
          </div>
        </div>

        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.hash.algorithms")
          }}</Label>
          <div class="flex flex-wrap gap-2">
            <label
              v-for="option in options"
              :key="option.value"
              class="flex cursor-pointer items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[11px] transition-colors"
              :class="
                algorithms.includes(option.value)
                  ? 'border-signal/30 bg-signal/10 text-signal'
                  : 'border-hairline bg-well text-muted-foreground hover:text-foreground'
              "
            >
              <input
                type="checkbox"
                class="accent-[color:var(--signal)]"
                :checked="algorithms.includes(option.value)"
                @change="toggleAlgorithm(option.value)"
              />
              {{ option.label }}
            </label>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <Button v-if="!running" size="sm" class="h-8 gap-1.5 text-[12px]" @click="start">
            <Hash class="size-3.5" />
            {{ t("devTools.hash.compute") }}
          </Button>
          <Button
            v-else
            variant="outline"
            size="sm"
            class="h-8 gap-1.5 border-warn/25 bg-warn/10 text-[12px] text-warn"
            @click="cancel"
          >
            <Square class="size-3" />
            {{ t("devTools.hash.cancel") }}
          </Button>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="running || result" class="rounded-xl border border-hairline bg-well-soft">
        <div class="border-b border-hairline px-3 py-2">
          <div class="flex items-center gap-2 text-[10.5px] text-muted-foreground">
            <span>{{ t("devTools.hash.progress") }}</span>
            <span class="numeric">{{ percent }}%</span>
            <span class="ml-auto numeric">
              {{ formatSize(progress.done) }} / {{ formatSize(progress.total) }}
            </span>
          </div>
          <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-overlay-2">
            <div
              class="h-full rounded-full bg-gradient-to-r from-signal to-rx transition-[width] duration-200"
              :style="{ width: `${percent}%` }"
            />
          </div>
        </div>

        <div v-if="result" class="overflow-x-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("devTools.hash.algorithm") }}
                </th>
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("devTools.hash.digest") }}
                </th>
                <th class="px-3 py-2"></th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="value in result.values"
                :key="value.algorithm"
                class="border-b border-hairline last:border-0"
              >
                <td class="px-3 py-2 text-muted-foreground">{{ value.algorithm }}</td>
                <td class="numeric break-all px-3 py-2 text-foreground">{{ value.digest }}</td>
                <td class="px-3 py-2 text-right">
                  <Button
                    variant="ghost"
                    size="sm"
                    class="h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                    @click="copy(value.digest)"
                  >
                    <Copy class="size-3" />
                    {{ t("tools.copy") }}
                  </Button>
                </td>
              </tr>
            </tbody>
          </table>
          <div
            class="flex flex-wrap items-center gap-2 border-t border-hairline px-3 py-2 text-[10.5px] text-muted-foreground"
          >
            <span class="numeric min-w-0 flex-1 truncate">{{ result.path }}</span>
            <span class="numeric">{{ formatSize(result.size) }} · {{ result.elapsedMs }} ms</span>
          </div>
        </div>
      </div>

      <div
        v-else-if="!running"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("devTools.hash.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
