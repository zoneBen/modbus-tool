<script setup lang="ts">
import {
  Copy,
  Eraser,
  FolderOpen,
  Plug,
  Plus,
  Power,
  RefreshCw,
  Server,
  Settings2,
  Terminal,
  Trash2,
} from "@lucide/vue";
import { computed, defineAsyncComponent, nextTick, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import SshFilePanel from "@/components/tools/network/SshFilePanel.vue";
import SshSessionDialog from "@/components/tools/network/SshSessionDialog.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { isDemoMode } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useSshStore } from "@/stores/ssh";
import { useToolboxStore } from "@/stores/toolbox";
import type { SshSessionStatus } from "@/types/tools";

const SshTerminal = defineAsyncComponent(
  () => import("@/components/tools/network/SshTerminal.vue"),
);

interface TerminalApi {
  clear(): void;
  focus(): void;
  copySelection(): Promise<void>;
  fit(): void;
}

const { t } = useI18n();
const ssh = useSshStore();
const toolbox = useToolboxStore();
const dialogOpen = ref(false);
const editProfileId = ref<string | null>(null);
const busyId = ref<string | null>(null);
const showFiles = toolbox.toolPref("network.ssh.showFiles", true);
const terminalRefs = new Map<string, TerminalApi>();

const active = computed(() => ssh.getProfile(ssh.activeId) ?? null);
const activeSessionId = computed(() =>
  active.value ? (ssh.sessionIds[active.value.id] ?? null) : null,
);
const status = computed<SshSessionStatus>(() =>
  active.value ? (ssh.statuses[active.value.id] ?? "disconnected") : "disconnected",
);
const detail = computed(() => (active.value ? (ssh.details[active.value.id] ?? null) : null));
const connected = computed(() => status.value === "connected" && !!activeSessionId.value);
const running = computed(() => (active.value ? ssh.isRunning(active.value.id) : false));
const follow = computed(() => active.value?.followCwd ?? false);
const activeTransfer = computed(() =>
  active.value ? (ssh.transfers[active.value.id] ?? null) : null,
);

const terminals = computed(() =>
  ssh.profiles
    .map((profile) => ({
      profileId: profile.id,
      sessionId: ssh.sessionIds[profile.id],
      name: profile.name,
      connected: ssh.statuses[profile.id] === "connected",
    }))
    .filter(
      (item): item is { profileId: string; sessionId: string; name: string; connected: boolean } =>
        Boolean(item.sessionId),
    ),
);

const statusTone: Record<SshSessionStatus, string> = {
  connected: "text-signal",
  connecting: "text-warn",
  disconnected: "text-muted-foreground",
  error: "text-error",
};

function setTerminalRef(sessionId: string, element: unknown): void {
  if (element) terminalRefs.set(sessionId, element as TerminalApi);
  else terminalRefs.delete(sessionId);
}

function currentTerminal(): TerminalApi | undefined {
  return activeSessionId.value ? terminalRefs.get(activeSessionId.value) : undefined;
}

function openDialog(profileId: string): void {
  editProfileId.value = profileId;
  dialogOpen.value = true;
}

function addProfile(): void {
  const profile = ssh.addProfile();
  openDialog(profile.id);
}

function editProfile(): void {
  if (active.value) openDialog(active.value.id);
}

async function connectProfile(profileId: string): Promise<void> {
  const profile = ssh.getProfile(profileId);
  if (!profile || busyId.value) return;
  if (!profile.host.trim()) {
    openDialog(profileId);
    return;
  }
  if (profile.auth === "password" && !ssh.passwords[profileId]) {
    openDialog(profileId);
    return;
  }
  if (profile.auth === "key" && !profile.keyPath.trim()) {
    openDialog(profileId);
    return;
  }
  busyId.value = profileId;
  try {
    await ssh.connect(profileId);
    toast.success(t("networkTools.ssh.connectedToast", { name: profile.name }));
    await nextTick();
    if (ssh.activeId === profileId) {
      const sessionId = ssh.sessionIds[profileId];
      if (sessionId) {
        window.setTimeout(() => terminalRefs.get(sessionId)?.focus(), 80);
      }
    }
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  } finally {
    busyId.value = null;
  }
}

async function disconnectProfile(): Promise<void> {
  if (!active.value || busyId.value) return;
  busyId.value = active.value.id;
  try {
    await ssh.disconnect(active.value.id);
    toast.info(t("networkTools.ssh.disconnectedToast"));
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  } finally {
    busyId.value = null;
  }
}

async function removeProfile(): Promise<void> {
  const profile = active.value;
  if (!profile) return;
  if (!window.confirm(t("networkTools.ssh.removeProfileConfirm", { name: profile.name }))) {
    return;
  }
  await ssh.removeProfile(profile.id);
}

async function setFollow(value: boolean): Promise<void> {
  const profile = active.value;
  if (!profile) return;
  ssh.updateProfile(profile.id, { followCwd: value });
  if (!connected.value) return;
  try {
    await ssh.setCwdTracking(profile.id, value);
  } catch (cause: unknown) {
    toast.error(toolErrorText(t, cause));
  }
}

function clearTerminal(): void {
  currentTerminal()?.clear();
}

function copyTerminal(): void {
  void currentTerminal()?.copySelection();
}

function focusTerminal(): void {
  currentTerminal()?.focus();
}

function formatConnectedAt(value: number): string {
  if (!value) return "—";
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(value));
}

onMounted(async () => {
  await ssh.init();
  if (!ssh.activeId && ssh.profiles.length) ssh.activeId = ssh.profiles[0].id;
  if (isDemoMode && !ssh.profiles.length) {
    const profile = ssh.addProfile();
    ssh.updateProfile(profile.id, {
      name: "Demo SSH",
      host: "demo.local",
      username: "demo",
    });
    ssh.passwords[profile.id] = "demo";
    await connectProfile(profile.id).catch(() => undefined);
  }
});
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.ssh.title')"
    :description="t('networkTools.tools.ssh.description')"
    :icon="Terminal"
  >
    <div class="mx-auto flex h-full min-h-[620px] max-w-[1560px] gap-3">
      <aside
        class="flex w-[220px] shrink-0 flex-col rounded-xl border border-hairline bg-well-soft"
      >
        <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
          <span class="text-[11px] font-medium">{{ t("networkTools.ssh.sessions") }}</span>
          <Button
            variant="ghost"
            size="sm"
            class="h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
            @click="addProfile"
          >
            <Plus class="size-3" />
            {{ t("networkTools.ssh.addSession") }}
          </Button>
        </div>
        <div class="scrollbar-thin min-h-0 flex-1 space-y-1 overflow-y-auto p-2">
          <button
            v-for="profile in ssh.profiles"
            :key="profile.id"
            type="button"
            class="w-full rounded-lg border px-2.5 py-2 text-left transition-colors"
            :class="
              profile.id === ssh.activeId
                ? 'border-rx/30 bg-rx/10'
                : 'border-transparent hover:border-hairline hover:bg-overlay-1'
            "
            @click="ssh.activeId = profile.id"
          >
            <div class="flex items-center gap-1.5">
              <span
                class="size-1.5 shrink-0 rounded-full"
                :class="{
                  'bg-signal': ssh.statuses[profile.id] === 'connected',
                  'bg-warn animate-pulse': ssh.statuses[profile.id] === 'connecting',
                  'bg-error': ssh.statuses[profile.id] === 'error',
                  'bg-muted-foreground/40': !ssh.statuses[profile.id],
                }"
              />
              <span class="min-w-0 flex-1 truncate text-[11.5px] font-medium">{{
                profile.name
              }}</span>
            </div>
            <p class="numeric mt-0.5 truncate text-[10px] text-muted-foreground">
              {{ profile.username }}@{{ profile.host || "—" }}:{{ profile.port }}
            </p>
            <p class="mt-0.5 truncate text-[9.5px] text-muted-foreground/80">
              {{
                profile.followCwd ? t("networkTools.ssh.followOn") : t("networkTools.ssh.followOff")
              }}
            </p>
          </button>
          <p
            v-if="!ssh.profiles.length"
            class="px-2 py-6 text-center text-[10.5px] leading-relaxed text-muted-foreground"
          >
            {{ t("networkTools.ssh.noSessions") }}
          </p>
        </div>
        <div
          class="border-t border-hairline px-2.5 py-2 text-[9.5px] leading-relaxed text-muted-foreground"
        >
          {{ t("networkTools.ssh.mcpHint") }}
        </div>
      </aside>

      <div v-if="active" class="flex min-w-0 flex-1 flex-col gap-2">
        <div class="rounded-xl border border-hairline bg-well-soft px-3 py-2">
          <div class="flex flex-wrap items-center gap-2">
            <span
              class="flex size-7 items-center justify-center rounded-lg border border-rx/25 bg-rx/10 text-rx"
            >
              <Server class="size-3.5" />
            </span>
            <div class="min-w-0">
              <p class="truncate text-[12px] font-medium">
                {{ active.name }}
                <span class="numeric ml-1 text-[10px] text-muted-foreground">
                  {{ active.username }}@{{ active.host || "—" }}:{{ active.port }}
                </span>
              </p>
              <div class="mt-0.5 flex items-center gap-2 text-[10px]">
                <span class="flex items-center gap-1" :class="statusTone[status]">
                  <span
                    class="status-dot scale-75"
                    :class="
                      status === 'connected' || status === 'connecting' ? 'status-dot-live' : ''
                    "
                  />
                  {{ t(`networkTools.ssh.statuses.${status}`) }}
                </span>
                <span v-if="activeSessionId" class="numeric text-muted-foreground">
                  {{ formatConnectedAt(ssh.connectedAt[active.id] ?? 0) }}
                </span>
                <span v-if="detail" class="min-w-0 truncate text-muted-foreground" :title="detail">
                  {{ detail }}
                </span>
              </div>
            </div>
            <div class="ml-auto flex flex-wrap items-center gap-1">
              <Button
                v-if="running"
                variant="outline"
                size="sm"
                class="h-7 gap-1 border-warn/25 bg-warn/10 text-[10.5px] text-warn"
                :disabled="busyId === active.id"
                @click="disconnectProfile"
              >
                <Power class="size-3" />
                {{ t("networkTools.ssh.disconnect") }}
              </Button>
              <Button
                v-else
                size="sm"
                class="h-7 gap-1 text-[10.5px]"
                :disabled="busyId === active.id"
                @click="connectProfile(active.id)"
              >
                <Plug class="size-3" />
                {{
                  busyId === active.id
                    ? t("networkTools.ssh.connecting")
                    : activeSessionId
                      ? t("networkTools.ssh.reconnect")
                      : t("networkTools.ssh.connect")
                }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                @click="editProfile"
              >
                <Settings2 class="size-3" />
                {{ t("networkTools.ssh.edit") }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                @click="showFiles = !showFiles"
              >
                <FolderOpen class="size-3" />
                {{ showFiles ? t("networkTools.ssh.hideFiles") : t("networkTools.ssh.showFiles") }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                :disabled="!activeSessionId"
                @click="copyTerminal"
              >
                <Copy class="size-3" />
                {{ t("networkTools.ssh.copySelection") }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                :disabled="!activeSessionId"
                @click="clearTerminal"
              >
                <Eraser class="size-3" />
                {{ t("networkTools.ssh.clear") }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                @click="removeProfile"
              >
                <Trash2 class="size-3" />
                {{ t("networkTools.ssh.deleteProfile") }}
              </Button>
            </div>
          </div>
        </div>

        <div class="flex min-h-0 flex-1 gap-2">
          <div class="relative min-w-0 flex-1 rounded-xl border border-hairline bg-[#0a111b]">
            <div
              v-for="item in terminals"
              v-show="item.sessionId === activeSessionId"
              :key="item.sessionId"
              class="absolute inset-0"
              @click="focusTerminal"
            >
              <SshTerminal
                :ref="(element) => setTerminalRef(item.sessionId, element)"
                :session-id="item.sessionId"
                :active="item.sessionId === activeSessionId"
                :connected="item.connected"
              />
            </div>
            <div
              v-if="!activeSessionId"
              class="flex h-full min-h-[300px] flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-hairline bg-well-soft p-8 text-center"
            >
              <span
                class="flex size-12 items-center justify-center rounded-2xl border border-rx/25 bg-rx/10 text-rx"
              >
                <Terminal class="size-6" />
              </span>
              <div>
                <p class="text-[13px] font-medium">{{ t("networkTools.ssh.notConnected") }}</p>
                <p class="mt-1 text-[11px] text-muted-foreground">
                  {{ t("networkTools.ssh.connectHint") }}
                </p>
              </div>
              <Button size="sm" class="gap-1.5 text-[11px]" @click="connectProfile(active.id)">
                <Plug class="size-3.5" />
                {{ t("networkTools.ssh.connect") }}
              </Button>
            </div>
            <div
              v-else-if="status !== 'connected'"
              class="pointer-events-none absolute inset-x-3 top-3 z-10 flex items-center gap-2 rounded-lg border border-warn/25 bg-warn/10 px-2.5 py-1.5 text-[10.5px] text-warn backdrop-blur"
            >
              <RefreshCw class="size-3" />
              {{ t("networkTools.ssh.sessionEnded") }}
              <button
                type="button"
                class="pointer-events-auto ml-auto underline underline-offset-2"
                @click="connectProfile(active.id)"
              >
                {{ t("networkTools.ssh.reconnect") }}
              </button>
            </div>
          </div>

          <SshFilePanel
            v-if="showFiles && connected && activeSessionId"
            :session-id="activeSessionId"
            :follow="follow"
            :cwd="ssh.cwds[active.id]"
            @update:follow="setFollow"
          />
        </div>

        <div
          v-if="activeTransfer"
          class="flex items-center gap-2 rounded-lg border border-hairline bg-well-soft px-2.5 py-1.5 text-[10px] text-muted-foreground"
        >
          <span class="numeric">
            {{
              activeTransfer.direction === "upload"
                ? t("networkTools.ssh.upload")
                : t("networkTools.ssh.download")
            }}
          </span>
          <span class="min-w-0 flex-1 truncate">{{ activeTransfer.path }}</span>
          <span class="numeric">
            {{
              Math.round((activeTransfer.transferred / Math.max(1, activeTransfer.total)) * 100)
            }}%
          </span>
        </div>
      </div>

      <div
        v-else
        class="flex flex-1 items-center justify-center rounded-xl border border-dashed border-hairline text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.ssh.selectOrCreate") }}
      </div>
    </div>

    <SshSessionDialog
      v-model:open="dialogOpen"
      :profile-id="editProfileId"
      @connect="connectProfile"
    />
  </ToolPanelFrame>
</template>
