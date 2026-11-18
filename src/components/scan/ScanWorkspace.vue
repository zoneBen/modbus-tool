<script setup lang="ts">
import { Download, Eraser, Plus, Radar, Square, Trash2 } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import EmptyState from "@/components/common/EmptyState.vue";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import NewScanPanel from "@/components/scan/NewScanPanel.vue";
import { formatLatency, formatTime } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import type { ScanTask } from "@/stores/scans";
import { useScansStore } from "@/stores/scans";

const scans = useScansStore();
const { t } = useI18n();

const creating = ref(false);
const removeOpen = ref(false);
const removing = ref(false);
const removeTarget = ref<ScanTask | null>(null);
const cancelling = ref(false);

const statusTone: Record<string, string> = {
  ok: "border-signal/25 bg-signal/10 text-signal",
  exception: "border-warn/25 bg-warn/10 text-warn",
  timeout: "border-hairline-strong bg-overlay-1 text-muted-foreground",
  error: "border-error/25 bg-error/10 text-error",
  open: "border-rx/25 bg-rx/10 text-rx",
};

function kindLabel(task: ScanTask): string {
  return t(`scan.kindLabels.${task.kind}`);
}

function statusLabel(task: ScanTask): string {
  return t(`scan.status.${task.status}`);
}

function progressText(task: ScanTask): string {
  if (!task.progress) return "0/0";
  return `${task.progress.done}/${task.progress.total}`;
}

function foundCount(task: ScanTask): number {
  return task.progress?.found ?? scans.foundResults(task).length;
}

function startNew(): void {
  creating.value = true;
}

function onStarted(taskId: string): void {
  scans.selectTask(taskId);
  creating.value = false;
}

function requestRemove(task: ScanTask): void {
  removeTarget.value = task;
  removeOpen.value = true;
}

function closeRemove(): void {
  if (removing.value) return;
  removeOpen.value = false;
  removeTarget.value = null;
}

async function confirmRemove(): Promise<void> {
  const target = removeTarget.value;
  if (!target || removing.value) return;
  removing.value = true;
  try {
    await scans.removeTask(target.id);
    toast.success(t("scan.removed"));
    removeOpen.value = false;
    removeTarget.value = null;
  } catch (error: unknown) {
    toast.error(t("scan.removeFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    removing.value = false;
  }
}

async function cancelActive(): Promise<void> {
  const task = scans.activeTask;
  if (!task || cancelling.value) return;
  cancelling.value = true;
  try {
    await scans.cancel(task.id);
  } finally {
    cancelling.value = false;
  }
}

function csvCell(value: unknown): string {
  return `"${String(value ?? "").replace(/"/g, '""')}"`;
}

async function exportResults(): Promise<void> {
  const task = scans.activeTask;
  if (!task || !task.results.length) {
    toast.info(t("scan.exportEmpty"));
    return;
  }
  const header = ["target", "status", "detail", "latencyMs"];
  const lines = [header.map(csvCell).join(",")];
  for (const result of task.results) {
    lines.push(
      [result.label, result.status, result.detail, result.latencyMs ?? ""].map(csvCell).join(","),
    );
  }
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const scope = task.name.replace(/[\\/:*?"<>|\s]+/g, "_");
  try {
    const saved = await ipc.saveTextFile(
      `modbus-scan-${scope}-${stamp}.csv`,
      `${String.fromCharCode(0xfeff)}${lines.join("\r\n")}`,
    );
    if (saved) toast.success(t("scan.exported", { count: task.results.length }));
  } catch (error: unknown) {
    toast.error(t("scan.exportFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}

const finishedCount = computed(
  () => scans.tasks.filter((task) => task.status !== "running").length,
);
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1">
    <aside class="flex h-full w-[268px] shrink-0 flex-col border-r border-hairline bg-overlay-1">
      <div class="flex h-[52px] shrink-0 items-center justify-between px-4">
        <div class="min-w-0">
          <p class="text-[13px] font-semibold tracking-tight">{{ t("scan.title") }}</p>
          <p class="text-[11px] text-muted-foreground">
            {{ t("scan.runningCount", { count: scans.runningCount }) }}
          </p>
        </div>
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground disabled:opacity-40"
            :title="t('scan.clearFinished')"
            :aria-label="t('scan.clearFinished')"
            :disabled="!finishedCount"
            @click="scans.clearFinished()"
          >
            <Eraser class="size-3" />
          </button>
          <Button
            variant="ghost"
            size="icon-sm"
            class="rounded-lg border border-hairline bg-overlay-1 text-muted-foreground hover:text-foreground"
            :aria-label="t('scan.newScan')"
            :title="t('scan.newScan')"
            @click="startNew"
          >
            <Plus class="size-4" />
          </Button>
        </div>
      </div>

      <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-2.5 pb-3">
        <div v-if="scans.tasks.length" class="space-y-1.5">
          <div
            v-for="task in scans.tasks"
            :key="task.id"
            role="button"
            tabindex="0"
            class="group relative w-full cursor-pointer rounded-xl border px-3 py-2.5 text-left transition-colors"
            :class="
              task.id === scans.activeTaskId && !creating
                ? 'border-hairline-strong bg-overlay-2 shadow-lg shadow-black/20'
                : 'border-transparent hover:border-hairline hover:bg-overlay-1'
            "
            @click="
              () => {
                scans.selectTask(task.id);
                creating = false;
              }
            "
            @keydown.enter="
              () => {
                scans.selectTask(task.id);
                creating = false;
              }
            "
          >
            <span
              v-if="task.id === scans.activeTaskId && !creating"
              class="absolute inset-y-3 left-0 w-[2px] rounded-full bg-signal"
            />
            <div class="flex items-start gap-2">
              <span
                class="mt-1 size-1.5 shrink-0 rounded-full"
                :class="
                  task.status === 'running'
                    ? 'status-dot status-dot-live text-signal'
                    : task.status === 'error'
                      ? 'bg-error'
                      : 'bg-muted-foreground/40'
                "
              />
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-1.5">
                  <span class="truncate text-[12px] font-medium">{{ task.name }}</span>
                  <span
                    class="shrink-0 rounded border px-1 py-0.5 text-[9.5px]"
                    :class="
                      task.status === 'running'
                        ? 'border-signal/20 bg-signal/10 text-signal'
                        : task.status === 'error'
                          ? 'border-error/25 bg-error/10 text-error'
                          : 'border-hairline bg-overlay-1 text-muted-foreground'
                    "
                  >
                    {{ statusLabel(task) }}
                  </span>
                </div>
                <p class="mt-0.5 truncate text-[10px] text-muted-foreground/80">
                  {{ kindLabel(task) }} · {{ task.description }}
                </p>
                <div class="mt-1 flex items-center gap-2 text-[10px] text-muted-foreground">
                  <span class="numeric">{{ progressText(task) }}</span>
                  <span>· {{ t("scan.found", { count: foundCount(task) }) }}</span>
                  <span class="ml-auto numeric">{{ formatTime(task.createdAt) }}</span>
                </div>
                <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-overlay-2">
                  <div
                    class="h-full rounded-full transition-[width] duration-200"
                    :class="
                      task.status === 'error' ? 'bg-error/70' : 'bg-gradient-to-r from-signal to-rx'
                    "
                    :style="{ width: `${scans.percent(task)}%` }"
                  />
                </div>
              </div>
              <span
                role="button"
                tabindex="0"
                class="flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-all hover:bg-error/10 hover:text-error group-hover:opacity-100"
                :title="t('scan.remove')"
                :aria-label="t('scan.remove')"
                @click.stop="requestRemove(task)"
                @keydown.enter.stop="requestRemove(task)"
              >
                <Trash2 class="size-3" />
              </span>
            </div>
          </div>
        </div>

        <div v-else class="mt-8 flex flex-col items-center gap-2 px-3 text-center">
          <span
            class="flex size-10 items-center justify-center rounded-xl border border-signal/20 bg-signal/10 text-signal"
          >
            <Radar class="size-4" />
          </span>
          <p class="text-[12px] text-muted-foreground">{{ t("scan.noTasks") }}</p>
          <p class="text-[10.5px] leading-relaxed text-muted-foreground/75">
            {{ t("scan.noTasksHint") }}
          </p>
          <Button size="sm" class="mt-1 gap-1 text-[11.5px]" @click="startNew">
            <Plus class="size-3" />
            {{ t("scan.newScan") }}
          </Button>
        </div>
      </div>
    </aside>

    <NewScanPanel v-if="creating" @close="creating = false" @started="onStarted" />

    <div v-else-if="scans.activeTask" class="flex min-h-0 min-w-0 flex-1 flex-col">
      <header class="flex h-[54px] shrink-0 items-center gap-3 border-b border-hairline px-4">
        <span
          class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
        >
          <Radar class="size-4" />
        </span>
        <div class="min-w-0 flex-1">
          <div class="flex min-w-0 items-center gap-2">
            <p class="truncate text-[13px] font-semibold">{{ scans.activeTask.name }}</p>
            <span
              class="rounded border border-hairline bg-overlay-1 px-1.5 py-0.5 text-[10px] text-muted-foreground"
            >
              {{ kindLabel(scans.activeTask) }}
            </span>
            <span
              class="rounded border px-1.5 py-0.5 text-[10px]"
              :class="
                scans.activeTask.status === 'running'
                  ? 'border-signal/20 bg-signal/10 text-signal'
                  : scans.activeTask.status === 'error'
                    ? 'border-error/25 bg-error/10 text-error'
                    : 'border-hairline bg-overlay-1 text-muted-foreground'
              "
            >
              {{ statusLabel(scans.activeTask) }}
            </span>
          </div>
          <p class="mt-0.5 truncate text-[10.5px] text-muted-foreground">
            {{ scans.activeTask.description }} · {{ progressText(scans.activeTask) }} ·
            {{ t("scan.found", { count: foundCount(scans.activeTask) }) }}
          </p>
        </div>
        <Button
          v-if="scans.activeTask.status === 'running'"
          variant="outline"
          size="sm"
          class="h-8 shrink-0 gap-1.5 border-warn/25 bg-warn/10 text-[11px] text-warn"
          :disabled="cancelling"
          @click="cancelActive"
        >
          <Square class="size-3" />
          {{ t("scan.cancel") }}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-8 shrink-0 gap-1 text-[11px] text-muted-foreground"
          :disabled="!scans.activeTask.results.length"
          @click="exportResults"
        >
          <Download class="size-3" />
          {{ t("scan.export") }}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-8 shrink-0 gap-1 text-[11px] text-muted-foreground"
          :disabled="!scans.activeTask.results.length"
          @click="scans.clearResults(scans.activeTask.id)"
        >
          <Eraser class="size-3" />
          {{ t("scan.clearResults") }}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-8 shrink-0 gap-1 text-[11px] text-muted-foreground hover:text-error"
          @click="requestRemove(scans.activeTask)"
        >
          <Trash2 class="size-3" />
          {{ t("scan.remove") }}
        </Button>
      </header>

      <div
        v-if="scans.activeTask.error"
        class="shrink-0 border-b border-error/25 bg-error/10 px-4 py-2 text-[11px] text-error"
      >
        {{ scans.activeTask.error }}
      </div>

      <div class="shrink-0 border-b border-hairline bg-overlay-1/35 px-4 py-2">
        <div class="mb-1 flex items-center gap-2 text-[10.5px] text-muted-foreground">
          <span>{{ t("scan.progress") }}</span>
          <span class="numeric">{{ progressText(scans.activeTask) }}</span>
          <span class="ml-auto numeric">{{ scans.percent(scans.activeTask) }}%</span>
        </div>
        <div class="h-1 overflow-hidden rounded-full bg-overlay-2">
          <div
            class="h-full rounded-full bg-gradient-to-r from-signal to-rx transition-[width] duration-200"
            :style="{ width: `${scans.percent(scans.activeTask)}%` }"
          />
        </div>
      </div>

      <div class="scrollbar-thin min-h-0 flex-1 overflow-auto px-4 py-3">
        <div v-if="scans.activeTask.results.length" class="rounded-xl border border-hairline">
          <table class="w-full border-collapse text-[11px]">
            <thead class="sticky top-0 bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("scan.target") }}
                </th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("scan.resultStatus") }}
                </th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("scan.detail") }}
                </th>
                <th class="px-3 py-2 text-right font-medium text-muted-foreground">
                  {{ t("scan.latency") }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(result, index) in scans.activeTask.results"
                :key="`${result.label}-${index}`"
                class="border-b border-hairline last:border-0"
              >
                <td class="numeric px-3 py-1.5 text-foreground">{{ result.label }}</td>
                <td class="px-2 py-1.5">
                  <span
                    class="rounded border px-1 py-0.5 text-[10px] uppercase"
                    :class="statusTone[result.status]"
                  >
                    {{ result.status }}
                  </span>
                </td>
                <td class="max-w-[420px] truncate px-2 py-1.5 text-muted-foreground">
                  {{ result.detail }}
                </td>
                <td class="numeric px-3 py-1.5 text-right text-muted-foreground">
                  {{ result.latencyMs !== undefined ? formatLatency(result.latencyMs) : "—" }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div
          v-else
          class="flex h-full min-h-[240px] flex-col items-center justify-center gap-2 text-center"
        >
          <span
            class="status-dot"
            :class="
              scans.activeTask.status === 'running' ? 'status-dot-live text-signal' : 'opacity-40'
            "
          />
          <p class="text-[12px] text-muted-foreground">
            {{
              scans.activeTask.status === "running"
                ? t("scan.waiting")
                : t("scan.completedNoResults")
            }}
          </p>
        </div>
      </div>
    </div>

    <EmptyState
      v-else
      :title="t('scan.noSelection')"
      :description="t('scan.noSelectionHint')"
      class="m-auto flex-1"
    >
      <template #icon>
        <Radar class="size-5" />
      </template>
      <Button size="sm" class="mt-1 gap-1" @click="startNew">
        <Plus class="size-3" />
        {{ t("scan.newScan") }}
      </Button>
    </EmptyState>

    <AlertDialog
      :open="removeOpen"
      @update:open="
        (open) => {
          if (!open) closeRemove();
        }
      "
    >
      <AlertDialogContent
        class="border-glass-border bg-[color:var(--glass-strong)] backdrop-blur-2xl"
      >
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t("scan.removeTitle") }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{ t("scan.removeDescription", { name: removeTarget?.name ?? "" }) }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel :disabled="removing">{{ t("common.cancel") }}</AlertDialogCancel>
          <Button
            variant="destructive"
            class="text-white"
            :disabled="removing"
            @click="confirmRemove"
          >
            {{ removing ? t("common.loading") : t("scan.remove") }}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
