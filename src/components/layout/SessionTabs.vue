<script setup lang="ts">
import { X } from "@lucide/vue";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import StatusPill from "@/components/common/StatusPill.vue";
import TransportGlyph from "@/components/common/TransportGlyph.vue";
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
import { useSessionsStore } from "@/stores/sessions";
import type { Session } from "@/types/models";

const sessions = useSessionsStore();
const { t } = useI18n();
const pendingDelete = ref<Session | null>(null);
const deleteOpen = ref(false);
const deleting = ref(false);

function select(sessionId: string): void {
  sessions.setActiveSession(sessionId);
}

function requestDelete(session: Session): void {
  pendingDelete.value = session;
  deleteOpen.value = true;
}

function closeDelete(): void {
  deleteOpen.value = false;
  pendingDelete.value = null;
}

async function confirmDelete(): Promise<void> {
  const target = pendingDelete.value;
  if (!target || deleting.value) return;
  deleting.value = true;
  try {
    await sessions.deleteSession(target.id);
    toast.success(t("workspace.deletedSession", { name: target.name }));
    closeDelete();
  } catch (error: unknown) {
    toast.error(t("workspace.deleteSessionFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    deleting.value = false;
  }
}
</script>

<template>
  <div class="flex h-[46px] shrink-0 items-stretch border-b border-hairline bg-well-soft">
    <div class="scrollbar-thin flex min-w-0 flex-1 items-stretch overflow-x-auto">
      <div
        v-for="session in sessions.sessions"
        :key="session.id"
        role="button"
        tabindex="0"
        class="group relative flex min-w-[190px] max-w-[250px] cursor-pointer items-center gap-2 border-r border-hairline px-3.5 text-left transition-colors"
        :class="session.id === sessions.activeSessionId ? 'bg-overlay-2' : 'hover:bg-overlay-1'"
        @click="select(session.id)"
        @keydown.enter="select(session.id)"
        @keydown.space.prevent="select(session.id)"
      >
        <TransportGlyph
          :transport="session.transport"
          :role="session.role"
          class="size-[14px] shrink-0"
          :class="
            session.role === 'slave'
              ? 'text-sim'
              : session.transport === 'serial'
                ? 'text-rx'
                : 'text-signal-dim'
          "
        />
        <div class="min-w-0 flex-1">
          <p class="truncate text-[12px] font-medium">{{ session.name }}</p>
          <p class="numeric truncate text-[11px] text-muted-foreground">
            {{ session.endpoint }}
          </p>
        </div>
        <StatusPill :status="session.status" :polling="session.polling" compact />
        <button
          type="button"
          class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground opacity-0 transition-all hover:bg-overlay-3 hover:text-foreground group-hover:opacity-100"
          :class="{ 'opacity-100': session.id === sessions.activeSessionId }"
          :aria-label="`${t('workspace.deleteSession')} ${session.name}`"
          @click.stop="requestDelete(session)"
        >
          <X class="size-3" />
        </button>
        <span
          v-if="session.id === sessions.activeSessionId"
          class="absolute inset-x-0 bottom-0 h-[1.5px] bg-gradient-to-r from-signal/70 via-signal to-rx/70"
        />
      </div>
    </div>

    <div class="flex shrink-0 items-center gap-1.5 border-l border-hairline px-2.5">
      <StatusPill
        v-if="sessions.activeSession"
        :status="sessions.activeSession.status"
        :polling="sessions.activeSession.polling"
        class="mr-1"
      />
    </div>

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
          <AlertDialogTitle>{{ t("workspace.deleteSessionTitle") }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{
              t("workspace.deleteSessionDescription", {
                name: pendingDelete?.name ?? "",
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
            {{ deleting ? t("workspace.deletingSession") : t("workspace.deleteSession") }}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
