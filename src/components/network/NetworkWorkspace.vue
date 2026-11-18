<script setup lang="ts">
import { Network, Plus, Trash2 } from "@lucide/vue";
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
import NetworkConsole from "@/components/network/NetworkConsole.vue";
import NewNetworkSessionDialog from "@/components/network/NewNetworkSessionDialog.vue";
import type { NetworkSession } from "@/stores/network";
import { useNetworkStore } from "@/stores/network";
import type { NetworkMode } from "@/types/models";

const network = useNetworkStore();
const { t } = useI18n();

const createOpen = ref(false);
const deleteOpen = ref(false);
const deleting = ref(false);
const deleteTarget = ref<NetworkSession | null>(null);

const modeKeys: Record<NetworkMode, string> = {
  "tcp-server": "tcpServer",
  "tcp-client": "tcpClient",
  "udp-server": "udpServer",
  "udp-client": "udpClient",
  "tcp-relay": "tcpRelay",
};

function modeLabel(mode: NetworkMode): string {
  return t(`network.modes.${modeKeys[mode]}`);
}

function endpointLabel(session: NetworkSession): string {
  if (session.endpoint) return session.endpoint;
  if (session.mode === "tcp-server" || session.mode === "udp-server") {
    return `${session.localHost || "0.0.0.0"}:${session.localPort}`;
  }
  if (session.mode === "tcp-relay") {
    return `${session.localHost || "0.0.0.0"}:${session.localPort} → ${session.remoteHost}:${session.remotePort}`;
  }
  return `${session.remoteHost}:${session.remotePort}`;
}

function requestDelete(session: NetworkSession): void {
  deleteTarget.value = session;
  deleteOpen.value = true;
}

function closeDelete(): void {
  if (deleting.value) return;
  deleteOpen.value = false;
  deleteTarget.value = null;
}

async function confirmDelete(): Promise<void> {
  const target = deleteTarget.value;
  if (!target || deleting.value) return;
  deleting.value = true;
  try {
    await network.deleteSession(target.id);
    toast.success(t("network.deleted"));
    deleteOpen.value = false;
    deleteTarget.value = null;
  } catch (error: unknown) {
    toast.error(t("network.deleteFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    deleting.value = false;
  }
}

const activeCount = computed(() => network.activeCount);
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1">
    <aside class="flex h-full w-[268px] shrink-0 flex-col border-r border-hairline bg-overlay-1">
      <div class="flex h-[52px] shrink-0 items-center justify-between px-4">
        <div class="min-w-0">
          <p class="text-[13px] font-semibold tracking-tight">{{ t("network.title") }}</p>
          <p class="text-[11px] text-muted-foreground">
            {{ t("network.activeSessions", { count: activeCount }) }}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          class="rounded-lg border border-hairline bg-overlay-1 text-muted-foreground hover:text-foreground"
          :aria-label="t('network.newSession')"
          :title="t('network.newSession')"
          @click="createOpen = true"
        >
          <Plus class="size-4" />
        </Button>
      </div>

      <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-2.5 pb-3">
        <div v-if="network.sessions.length" class="space-y-1.5">
          <div
            v-for="session in network.sessions"
            :key="session.id"
            role="button"
            tabindex="0"
            class="group relative w-full cursor-pointer rounded-xl border px-3 py-2.5 text-left transition-colors"
            :class="
              session.id === network.activeSessionId
                ? 'border-hairline-strong bg-overlay-2 shadow-lg shadow-black/20'
                : 'border-transparent hover:border-hairline hover:bg-overlay-1'
            "
            @click="network.selectSession(session.id)"
            @keydown.enter="network.selectSession(session.id)"
            @keydown.space.prevent="network.selectSession(session.id)"
          >
            <span
              v-if="session.id === network.activeSessionId"
              class="absolute inset-y-3 left-0 w-[2px] rounded-full bg-rx"
            />
            <div class="flex items-start gap-2">
              <span
                class="mt-1 size-1.5 shrink-0 rounded-full"
                :class="
                  session.active
                    ? 'status-dot status-dot-live text-signal'
                    : 'bg-muted-foreground/40'
                "
              />
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-1.5">
                  <span class="truncate text-[12px] font-medium">{{ session.name }}</span>
                  <span
                    class="shrink-0 rounded border border-rx/20 bg-rx/10 px-1 py-0.5 text-[9.5px] text-rx"
                  >
                    {{ modeLabel(session.mode) }}
                  </span>
                </div>
                <p class="numeric mt-1 truncate text-[10.5px] text-muted-foreground">
                  {{ endpointLabel(session) }}
                </p>
                <p class="mt-0.5 text-[10px] text-muted-foreground/70">
                  {{ t("network.entries", { count: session.entries.length }) }}
                  <span v-if="session.active" class="text-signal">
                    · {{ t("network.running") }}
                  </span>
                </p>
              </div>
              <button
                type="button"
                class="flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-all hover:bg-error/10 hover:text-error group-hover:opacity-100"
                :title="t('network.deleteSession')"
                :aria-label="t('network.deleteSession')"
                @click.stop="requestDelete(session)"
              >
                <Trash2 class="size-3" />
              </button>
            </div>
          </div>
        </div>

        <div v-else class="mt-8 flex flex-col items-center gap-2 px-3 text-center">
          <span
            class="flex size-10 items-center justify-center rounded-xl border border-rx/20 bg-rx/10 text-rx"
          >
            <Network class="size-4" />
          </span>
          <p class="text-[12px] text-muted-foreground">{{ t("network.noSessions") }}</p>
          <p class="text-[10.5px] leading-relaxed text-muted-foreground/75">
            {{ t("network.noSessionsHint") }}
          </p>
          <Button size="sm" class="mt-1 gap-1 text-[11.5px]" @click="createOpen = true">
            <Plus class="size-3" />
            {{ t("network.newSession") }}
          </Button>
        </div>
      </div>
    </aside>

    <NetworkConsole
      v-if="network.activeSession"
      :key="network.activeSession.id"
      :session="network.activeSession"
    />

    <EmptyState
      v-else
      :title="t('network.noSelection')"
      :description="t('network.noSelectionHint')"
      class="m-auto flex-1"
    >
      <template #icon>
        <Network class="size-5" />
      </template>
      <Button size="sm" class="mt-1 gap-1" @click="createOpen = true">
        <Plus class="size-3" />
        {{ t("network.newSession") }}
      </Button>
    </EmptyState>

    <NewNetworkSessionDialog v-model:open="createOpen" />

    <AlertDialog
      :open="deleteOpen"
      @update:open="
        (open) => {
          if (!open) closeDelete();
        }
      "
    >
      <AlertDialogContent
        class="border-glass-border bg-[color:var(--glass-strong)] backdrop-blur-2xl"
      >
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t("network.deleteSessionTitle") }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{
              t("network.deleteSessionDescription", {
                name: deleteTarget?.name ?? "",
              })
            }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel :disabled="deleting">{{ t("common.cancel") }}</AlertDialogCancel>
          <Button
            variant="destructive"
            class="text-white"
            :disabled="deleting"
            @click="confirmDelete"
          >
            {{ deleting ? t("network.deleting") : t("network.delete") }}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
