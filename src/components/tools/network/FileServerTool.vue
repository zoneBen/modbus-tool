<script setup lang="ts">
import {
  ArrowUp,
  Copy,
  File,
  Folder,
  FolderOpen,
  Play,
  RefreshCw,
  Server,
  Square,
} from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { copyText } from "@/lib/clipboard";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { FileServerEntry, FileServerStatus } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const root = toolbox.toolPref("network.fileserver.root", "");
const host = toolbox.toolPref("network.fileserver.host", "127.0.0.1");
const port = toolbox.toolPref("network.fileserver.port", "80");
const status = ref<FileServerStatus | null>(null);
const currentPath = ref("");
const copiedPath = ref<string | null>(null);
const busy = ref(false);
const error = ref<string | null>(null);
let poller: ReturnType<typeof setInterval> | null = null;
let copiedTimer: ReturnType<typeof setTimeout> | null = null;

const hostOptions = computed(() => [
  { label: t("networkTools.fileserver.hosts.loopback"), value: "127.0.0.1" },
  { label: t("networkTools.fileserver.hosts.lan"), value: "0.0.0.0" },
]);

const running = computed(() => Boolean(status.value?.running));
const lanExposed = computed(() => host.value === "0.0.0.0" || status.value?.host === "0.0.0.0");
const entries = computed(() => status.value?.entries ?? []);
const directoryCount = computed(() => entries.value.filter((entry) => entry.isDir).length);
const fileCount = computed(() => entries.value.length - directoryCount.value);
const shareBase = computed(() => {
  const urls = status.value?.urls ?? [];
  return (
    urls.find((url) => !url.includes("127.0.0.1") && !url.includes("localhost")) ?? urls[0] ?? ""
  );
});
const breadcrumbs = computed(() => {
  const path = status.value?.path ?? "";
  const items: Array<{ label: string; path: string }> = [
    { label: t("networkTools.fileserver.rootLabel"), path: "" },
  ];
  const segments = path.split("/").filter(Boolean);
  let cumulative = "";
  for (const segment of segments) {
    cumulative = cumulative ? `${cumulative}/${segment}` : segment;
    items.push({ label: segment, path: cumulative });
  }
  return items;
});

onMounted(async () => {
  try {
    status.value = await ipc.toolFileserverStatus(currentPath.value);
    if (status.value.running) currentPath.value = status.value.path;
    if (status.value.root) root.value = status.value.root;
    if (status.value.host) host.value = status.value.host;
    if (status.value.port) port.value = String(status.value.port);
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  }
  poller = setInterval(() => {
    if (status.value?.running) void refresh();
  }, 1500);
});

onBeforeUnmount(() => {
  if (poller) clearInterval(poller);
  poller = null;
  if (copiedTimer) clearTimeout(copiedTimer);
  copiedTimer = null;
});

async function refresh(): Promise<void> {
  try {
    status.value = await ipc.toolFileserverStatus(currentPath.value);
  } catch (cause: unknown) {
    if (currentPath.value) {
      currentPath.value = "";
      try {
        status.value = await ipc.toolFileserverStatus();
      } catch {
        // Keep the last snapshot when the service is stopping.
      }
    } else {
      error.value = toolErrorText(t, cause);
    }
  }
}

async function navigate(path: string): Promise<void> {
  currentPath.value = path;
  error.value = null;
  await refresh();
}

async function browse(): Promise<void> {
  const path = await ipc.pickDirectory();
  if (path) root.value = path;
}

async function start(): Promise<void> {
  if (busy.value) return;
  if (!root.value.trim()) {
    error.value = t("networkTools.fileserver.noRoot");
    return;
  }
  busy.value = true;
  error.value = null;
  currentPath.value = "";
  try {
    status.value = await ipc.toolFileserverStart({
      root: root.value.trim(),
      host: host.value,
      port: Math.min(65_535, Math.max(1, Number(port.value) || 80)),
    });
    currentPath.value = status.value.path;
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    status.value = null;
  } finally {
    busy.value = false;
  }
}

async function stop(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    status.value = await ipc.toolFileserverStop();
    currentPath.value = "";
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  } finally {
    busy.value = false;
  }
}

async function copy(value: string): Promise<void> {
  if (await copyText(value)) {
    toast.success(t("common.copied"));
  } else {
    toast.error(t("tools.copyFailed"));
  }
}

function encodePath(path: string): string {
  return path
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

function fileLink(entry: FileServerEntry): string {
  return shareBase.value ? `${shareBase.value}/download/${encodePath(entry.path)}` : "";
}

async function copyFileLink(entry: FileServerEntry): Promise<void> {
  const url = fileLink(entry);
  if (!url) {
    toast.error(t("networkTools.fileserver.linkUnavailable"));
    return;
  }
  if (!(await copyText(url))) {
    toast.error(t("networkTools.fileserver.linkCopyFailed"));
    return;
  }
  copiedPath.value = entry.path;
  toast.success(t("networkTools.fileserver.linkCopied"), { description: url });
  if (copiedTimer) clearTimeout(copiedTimer);
  copiedTimer = setTimeout(() => {
    copiedPath.value = null;
    copiedTimer = null;
  }, 1600);
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatTime(timestampMs: number): string {
  if (!timestampMs) return "—";
  return new Intl.DateTimeFormat(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(timestampMs));
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.fileserver.title')"
    :description="t('networkTools.tools.fileserver.description')"
    :icon="Server"
  >
    <div class="mx-auto max-w-[1120px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-[minmax(0,1fr)_190px_110px_120px]">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.fileserver.root")
            }}</Label>
            <div class="flex gap-2">
              <Input
                v-model="root"
                :disabled="running"
                class="numeric h-9 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
                :placeholder="t('networkTools.fileserver.rootPlaceholder')"
              />
              <Button
                variant="outline"
                size="sm"
                class="h-9 shrink-0 gap-1 border-hairline-strong bg-transparent text-[11px]"
                :disabled="running"
                @click="browse"
              >
                <FolderOpen class="size-3.5" />
                {{ t("networkTools.fileserver.browse") }}
              </Button>
            </div>
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.fileserver.host")
            }}</Label>
            <FormSelect v-model="host" :options="hostOptions" :disabled="running" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.fileserver.port")
            }}</Label>
            <Input
              v-model="port"
              :disabled="running"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="flex items-end">
            <Button
              v-if="!running"
              size="sm"
              class="h-9 w-full gap-1.5 text-[12px]"
              :disabled="busy"
              @click="start"
            >
              <Play class="size-3.5" />
              {{ busy ? t("tools.running") : t("networkTools.fileserver.start") }}
            </Button>
            <Button
              v-else
              variant="outline"
              size="sm"
              class="h-9 w-full gap-1.5 border-warn/25 bg-warn/10 text-[12px] text-warn"
              :disabled="busy"
              @click="stop"
            >
              <Square class="size-3" />
              {{ t("networkTools.fileserver.stop") }}
            </Button>
          </div>
        </div>

        <div
          class="rounded-lg border px-3 py-2 text-[10.5px] leading-relaxed"
          :class="
            lanExposed
              ? 'border-warn/25 bg-warn/[0.06] text-warn'
              : 'border-hairline text-muted-foreground'
          "
        >
          {{
            lanExposed
              ? t("networkTools.fileserver.lanWarning")
              : t("networkTools.fileserver.localHint")
          }}
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="status?.running" class="space-y-3">
        <div class="rounded-xl border border-hairline bg-well-soft px-3 py-2.5">
          <div class="flex flex-wrap items-center gap-2 text-[11px]">
            <span class="flex items-center gap-1.5 text-signal">
              <span class="status-dot status-dot-live scale-75" aria-hidden="true" />
              {{ t("networkTools.fileserver.running") }}
            </span>
            <span class="numeric text-muted-foreground">
              {{ status.host }}:{{ status.port }} · {{ directoryCount }}
              {{ t("networkTools.fileserver.folderCount") }} · {{ fileCount }}
              {{ t("networkTools.fileserver.fileCount") }} · ↑{{ status.uploads }} ↓{{
                status.downloads
              }}
            </span>
            <Button
              variant="ghost"
              size="sm"
              class="ml-auto h-7 gap-1 text-[11px] text-muted-foreground"
              @click="refresh"
            >
              <RefreshCw class="size-3" />
              {{ t("networkTools.fileserver.refresh") }}
            </Button>
          </div>
          <div class="mt-2 flex flex-wrap gap-2">
            <button
              v-for="url in status.urls"
              :key="url"
              type="button"
              class="numeric flex items-center gap-1.5 rounded-lg border border-hairline-strong bg-well px-2.5 py-1.5 text-[11px] text-foreground transition-colors hover:border-rx/40 hover:text-rx"
              @click="copy(url)"
            >
              {{ url }}
              <Copy class="size-3 text-muted-foreground" />
            </button>
          </div>
        </div>

        <div
          class="flex flex-wrap items-center gap-1.5 rounded-xl border border-hairline bg-well-soft px-3 py-2 text-[11px]"
        >
          <template v-for="(crumb, index) in breadcrumbs" :key="`${crumb.path}-${index}`">
            <span v-if="index > 0" class="text-muted-foreground/50" aria-hidden="true">/</span>
            <button
              type="button"
              class="rounded px-1.5 py-0.5 transition-colors"
              :class="
                index === breadcrumbs.length - 1
                  ? 'text-foreground'
                  : 'text-muted-foreground hover:bg-overlay-2 hover:text-foreground'
              "
              @click="navigate(crumb.path)"
            >
              {{ crumb.label }}
            </button>
          </template>
          <Button
            v-if="status.parent !== null"
            variant="ghost"
            size="sm"
            class="ml-auto h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
            @click="navigate(status.parent ?? '')"
          >
            <ArrowUp class="size-3" />
            {{ t("networkTools.fileserver.up") }}
          </Button>
        </div>

        <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <div class="rounded-xl border border-hairline">
            <div class="border-b border-hairline px-3 py-2 text-[11px] font-medium">
              {{ t("networkTools.fileserver.files") }}
            </div>
            <div class="scrollbar-thin max-h-[300px] overflow-auto">
              <table class="w-full border-collapse text-[11px]">
                <thead class="sticky top-0 bg-surface">
                  <tr class="border-b border-hairline">
                    <th class="px-3 py-1.5 text-left font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.name") }}
                    </th>
                    <th class="px-2 py-1.5 text-right font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.size") }}
                    </th>
                    <th class="px-3 py-1.5 text-right font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.modified") }}
                    </th>
                    <th class="px-3 py-1.5 text-right font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.actions") }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="entry in entries"
                    :key="entry.path"
                    class="border-b border-hairline last:border-0"
                  >
                    <td class="px-3 py-1.5 text-foreground">
                      <button
                        v-if="entry.isDir"
                        type="button"
                        class="flex min-w-0 max-w-full items-center gap-1.5 text-left transition-colors hover:text-rx"
                        @click="navigate(entry.path)"
                      >
                        <Folder class="size-3.5 shrink-0 text-sim" />
                        <span class="numeric break-all">{{ entry.name }}</span>
                      </button>
                      <span v-else class="flex min-w-0 max-w-full items-center gap-1.5">
                        <File class="size-3.5 shrink-0 text-muted-foreground" />
                        <span class="numeric break-all">{{ entry.name }}</span>
                      </span>
                    </td>
                    <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                      {{ entry.isDir ? "—" : formatSize(entry.size) }}
                    </td>
                    <td class="numeric px-3 py-1.5 text-right text-muted-foreground">
                      {{ formatTime(entry.modifiedMs) }}
                    </td>
                    <td class="px-3 py-1.5 text-right">
                      <Button
                        v-if="!entry.isDir"
                        variant="ghost"
                        size="sm"
                        class="h-6 gap-1 px-1.5 text-[10.5px]"
                        :class="copiedPath === entry.path ? 'text-signal' : 'text-muted-foreground'"
                        :title="fileLink(entry)"
                        @click="copyFileLink(entry)"
                      >
                        <Copy class="size-3" />
                        {{
                          copiedPath === entry.path
                            ? t("networkTools.fileserver.copied")
                            : t("networkTools.fileserver.copyLink")
                        }}
                      </Button>
                    </td>
                  </tr>
                </tbody>
              </table>
              <p
                v-if="!entries.length"
                class="px-3 py-8 text-center text-[11px] text-muted-foreground"
              >
                {{ t("networkTools.fileserver.empty") }}
              </p>
            </div>
          </div>

          <div class="rounded-xl border border-hairline">
            <div class="border-b border-hairline px-3 py-2 text-[11px] font-medium">
              {{ t("networkTools.fileserver.requests") }}
            </div>
            <div class="scrollbar-thin max-h-[300px] overflow-auto">
              <table class="w-full border-collapse text-[11px]">
                <thead class="sticky top-0 bg-surface">
                  <tr class="border-b border-hairline">
                    <th class="px-3 py-1.5 text-left font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.time") }}
                    </th>
                    <th class="px-2 py-1.5 text-left font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.remote") }}
                    </th>
                    <th class="px-2 py-1.5 text-left font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.request") }}
                    </th>
                    <th class="px-3 py-1.5 text-right font-medium text-muted-foreground">
                      {{ t("networkTools.fileserver.status") }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(request, index) in status.requests"
                    :key="`${request.timestampMs}-${index}`"
                    class="border-b border-hairline last:border-0"
                  >
                    <td class="numeric px-3 py-1.5 text-muted-foreground">
                      {{ formatTime(request.timestampMs) }}
                    </td>
                    <td class="numeric px-2 py-1.5 text-muted-foreground">{{ request.remote }}</td>
                    <td class="numeric max-w-[220px] truncate px-2 py-1.5 text-foreground">
                      {{ request.method }} {{ request.path }}
                    </td>
                    <td
                      class="numeric px-3 py-1.5 text-right"
                      :class="request.status < 400 ? 'text-signal' : 'text-error'"
                    >
                      {{ request.status }} · {{ formatSize(request.bytes) }}
                    </td>
                  </tr>
                </tbody>
              </table>
              <p
                v-if="!status.requests.length"
                class="px-3 py-8 text-center text-[11px] text-muted-foreground"
              >
                {{ t("networkTools.fileserver.noRequests") }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else-if="!busy"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.fileserver.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
