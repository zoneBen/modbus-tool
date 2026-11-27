<script setup lang="ts">
import {
  ArrowUp,
  Download,
  File,
  Folder,
  FolderPlus,
  Link2,
  Pencil,
  RefreshCw,
  Trash2,
  Upload,
} from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import type { SshFileEntry, SshTransferEvent } from "@/types/tools";

const props = defineProps<{
  sessionId: string;
  follow: boolean;
  cwd?: string;
}>();

const emit = defineEmits<{
  (event: "update:follow", value: boolean): void;
}>();

const { t } = useI18n();
const path = ref("");
const entries = ref<SshFileEntry[]>([]);
const busy = ref(false);
const error = ref<string | null>(null);
const transfer = ref<SshTransferEvent | null>(null);
let unlisten: (() => void) | null = null;

const crumbs = computed(() => {
  if (!path.value) return [];
  const parts = path.value.split("/").filter(Boolean);
  const result: Array<{ label: string; path: string }> = [{ label: "/", path: "/" }];
  let current = "";
  for (const part of parts) {
    current += `/${part}`;
    result.push({ label: part, path: current });
  }
  return result;
});

const parentPath = computed(() => {
  if (!path.value || path.value === "/") return "/";
  const trimmed = path.value.replace(/\/+$/, "");
  const index = trimmed.lastIndexOf("/");
  return index <= 0 ? "/" : trimmed.slice(0, index);
});

const transferPercent = computed(() => {
  if (!transfer.value || !transfer.value.total) return 0;
  return Math.min(100, Math.round((transfer.value.transferred / transfer.value.total) * 100));
});

const transferLabel = computed(() =>
  transfer.value ? t(`networkTools.ssh.transferStatuses.${transfer.value.status}`) : "",
);

function joinRemote(dir: string, name: string): string {
  if (!dir || dir === ".") return name;
  if (dir === "/") return `/${name}`;
  return `${dir.replace(/\/+$/, "")}/${name}`;
}

async function load(target?: string): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    const result = await ipc.toolSshSftpList(props.sessionId, target ?? path.value);
    path.value = result.path;
    entries.value = result.entries;
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    entries.value = [];
  } finally {
    busy.value = false;
  }
}

function enter(entry: SshFileEntry): void {
  if (entry.isDir) {
    void load(entry.path);
  } else {
    void download(entry);
  }
}

async function download(entry: SshFileEntry): Promise<void> {
  const localPath = await ipc.pickSavePath(entry.name);
  if (!localPath) return;
  try {
    await ipc.toolSshDownload({
      sessionId: props.sessionId,
      remotePath: entry.path,
      localPath,
    });
    toast.success(t("networkTools.ssh.downloadDone"), { description: localPath });
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

async function upload(): Promise<void> {
  const localPath = await ipc.pickFile();
  if (!localPath) return;
  try {
    await ipc.toolSshUpload({
      sessionId: props.sessionId,
      localPath,
      remoteDir: path.value,
    });
    toast.success(t("networkTools.ssh.uploadDone"));
    await load();
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

async function createFolder(): Promise<void> {
  const name = window.prompt(t("networkTools.ssh.folderPrompt"));
  if (!name) return;
  try {
    await ipc.toolSshSftpMkdir(props.sessionId, joinRemote(path.value, name));
    await load();
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

async function renameEntry(entry: SshFileEntry): Promise<void> {
  const name = window.prompt(t("networkTools.ssh.renamePrompt"), entry.name);
  if (!name || name === entry.name) return;
  try {
    await ipc.toolSshSftpRename(props.sessionId, entry.path, joinRemote(path.value, name));
    await load();
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

async function removeEntry(entry: SshFileEntry): Promise<void> {
  if (!window.confirm(t("networkTools.ssh.removeConfirm", { name: entry.name }))) return;
  try {
    await ipc.toolSshSftpRemove(props.sessionId, entry.path, entry.isDir);
    await load();
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

function go(): void {
  if (path.value.trim()) void load(path.value.trim());
}

function toggleFollow(value: boolean): void {
  emit("update:follow", value);
}

function formatBytes(size: number): string {
  if (!size) return "—";
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  if (size < 1024 * 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`;
  return `${(size / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatTime(value: number): string {
  if (!value) return "—";
  return new Intl.DateTimeFormat(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

watch(
  () => props.sessionId,
  () => {
    path.value = props.cwd || "";
    void load();
  },
  { immediate: true },
);

watch(
  () => props.cwd,
  (cwd) => {
    if (!props.follow || !cwd) return;
    if (cwd === path.value) return;
    path.value = cwd;
    void load(cwd);
  },
);

onMounted(async () => {
  unlisten = await ipc.on("ssh:transfer", (event) => {
    if (event.sessionId !== props.sessionId) return;
    transfer.value = event;
    if (event.status === "done" || event.status === "error") {
      window.setTimeout(() => {
        if (transfer.value?.timestampMs === event.timestampMs) transfer.value = null;
      }, 1800);
      if (event.status === "error") toast.error(event.detail || transferLabel.value);
    }
  });
});

onBeforeUnmount(() => {
  unlisten?.();
  unlisten = null;
});
</script>

<template>
  <aside class="flex w-[340px] shrink-0 flex-col rounded-xl border border-hairline bg-well-soft">
    <div class="flex items-center gap-2 border-b border-hairline px-3 py-2">
      <span class="text-[11px] font-medium">{{ t("networkTools.ssh.files") }}</span>
      <span
        class="numeric rounded border border-hairline px-1.5 py-0.5 text-[9px] text-muted-foreground"
      >
        {{ entries.length }}
      </span>
      <label class="ml-auto flex items-center gap-1.5 text-[9.5px] text-muted-foreground">
        <Switch :model-value="follow" @update:model-value="toggleFollow(Boolean($event))" />
        {{ t("networkTools.ssh.followCwd") }}
      </label>
    </div>

    <div class="flex items-center gap-1 border-b border-hairline px-2 py-1.5">
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 text-muted-foreground"
        :title="t('networkTools.ssh.parent')"
        @click="load(parentPath)"
      >
        <ArrowUp class="size-3" />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 text-muted-foreground"
        :title="t('networkTools.ssh.refresh')"
        :disabled="busy"
        @click="load()"
      >
        <RefreshCw class="size-3" :class="{ 'animate-spin': busy }" />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 text-muted-foreground"
        :title="t('networkTools.ssh.upload')"
        @click="upload"
      >
        <Upload class="size-3" />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 text-muted-foreground"
        :title="t('networkTools.ssh.newFolder')"
        @click="createFolder"
      >
        <FolderPlus class="size-3" />
      </Button>
      <Input
        v-model="path"
        class="numeric ml-1 h-6 min-w-0 flex-1 rounded border-hairline-strong bg-well px-1.5 text-[10px]"
        @keydown.enter="go"
      />
      <Button
        variant="outline"
        size="sm"
        class="h-6 shrink-0 border-hairline-strong bg-transparent px-1.5 text-[9.5px]"
        @click="go"
      >
        {{ t("networkTools.ssh.go") }}
      </Button>
    </div>

    <div
      class="scrollbar-thin flex flex-wrap items-center gap-0.5 border-b border-hairline px-2 py-1 text-[9.5px]"
    >
      <button
        v-for="crumb in crumbs"
        :key="crumb.path"
        type="button"
        class="rounded px-1 py-0.5 text-muted-foreground hover:bg-overlay-1 hover:text-foreground"
        @click="load(crumb.path)"
      >
        {{ crumb.label }}
        <span v-if="crumb.path !== path" class="opacity-40">/</span>
      </button>
    </div>

    <div v-if="transfer" class="border-b border-hairline px-2 py-1.5">
      <div class="flex items-center gap-2 text-[9.5px] text-muted-foreground">
        <span class="numeric">
          {{
            transfer.direction === "upload"
              ? t("networkTools.ssh.upload")
              : t("networkTools.ssh.download")
          }}
        </span>
        <span class="min-w-0 flex-1 truncate">{{ transfer.path }}</span>
        <span class="numeric">{{ transferPercent }}%</span>
      </div>
      <div class="mt-1 h-1 overflow-hidden rounded-full bg-overlay-2">
        <div
          class="h-full rounded-full transition-all"
          :class="transfer.status === 'error' ? 'bg-error' : 'bg-rx'"
          :style="{ width: `${transferPercent}%` }"
        />
      </div>
    </div>

    <p v-if="error" class="border-b border-error/20 bg-error/10 px-2 py-1.5 text-[10px] text-error">
      {{ error }}
    </p>

    <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto">
      <div
        v-for="entry in entries"
        :key="entry.path"
        class="group flex items-center gap-2 border-b border-hairline/60 px-2 py-1.5 hover:bg-overlay-1"
      >
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-1.5 text-left"
          :title="entry.path"
          @dblclick="enter(entry)"
          @click="entry.isDir ? enter(entry) : undefined"
        >
          <Folder v-if="entry.isDir" class="size-3.5 shrink-0 text-rx" />
          <File v-else class="size-3.5 shrink-0 text-muted-foreground" />
          <Link2 v-if="entry.isSymlink" class="size-3 shrink-0 text-sim" />
          <span class="min-w-0 flex-1 truncate text-[11px]">{{ entry.name }}</span>
        </button>
        <span class="numeric w-14 shrink-0 text-right text-[9px] text-muted-foreground">
          {{ entry.isDir ? "—" : formatBytes(entry.size) }}
        </span>
        <span
          class="numeric hidden w-[74px] shrink-0 text-right text-[9px] text-muted-foreground sm:block"
        >
          {{ formatTime(entry.modifiedMs) }}
        </span>
        <div
          class="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100"
        >
          <button
            v-if="!entry.isDir"
            type="button"
            class="rounded p-1 text-muted-foreground hover:bg-overlay-2 hover:text-signal"
            :title="t('networkTools.ssh.download')"
            @click="download(entry)"
          >
            <Download class="size-3" />
          </button>
          <button
            type="button"
            class="rounded p-1 text-muted-foreground hover:bg-overlay-2 hover:text-foreground"
            :title="t('networkTools.ssh.rename')"
            @click="renameEntry(entry)"
          >
            <Pencil class="size-3" />
          </button>
          <button
            type="button"
            class="rounded p-1 text-muted-foreground hover:bg-overlay-2 hover:text-error"
            :title="t('networkTools.ssh.remove')"
            @click="removeEntry(entry)"
          >
            <Trash2 class="size-3" />
          </button>
        </div>
      </div>
      <p
        v-if="!entries.length && !busy"
        class="px-3 py-8 text-center text-[10.5px] text-muted-foreground"
      >
        {{ t("networkTools.ssh.emptyDir") }}
      </p>
    </div>

    <div class="border-t border-hairline px-2 py-1.5 text-[9px] text-muted-foreground">
      {{ t("networkTools.ssh.permissionsHint") }}
    </div>
  </aside>
</template>
