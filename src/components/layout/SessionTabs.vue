<script setup lang="ts">
import { Plus, X } from "@lucide/vue";
import { computed, ref } from "vue";
import { toast } from "vue-sonner";
import StatusPill from "@/components/common/StatusPill.vue";
import TransportGlyph from "@/components/common/TransportGlyph.vue";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useSessionsStore } from "@/stores/sessions";
import type { Session } from "@/types/models";

const emit = defineEmits<{
  (event: "new-session"): void;
}>();

const sessions = useSessionsStore();
const pendingDelete = ref<Session | null>(null);
const deleteOpen = computed({
  get: () => pendingDelete.value !== null,
  set: (open: boolean) => {
    if (!open) pendingDelete.value = null;
  },
});

function select(sessionId: string): void {
  sessions.setActiveSession(sessionId);
}

async function confirmDelete(): Promise<void> {
  const target = pendingDelete.value;
  if (!target) return;
  try {
    await sessions.deleteSession(target.id);
    toast.success(`已删除会话「${target.name}」`);
  } catch (error: unknown) {
    toast.error("删除会话失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    pendingDelete.value = null;
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
          :aria-label="`删除会话 ${session.name}`"
          @click.stop="pendingDelete = session"
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
      <TooltipProvider :delay-duration="180">
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              variant="ghost"
              size="icon-sm"
              class="rounded-lg text-muted-foreground hover:text-foreground"
              aria-label="新建会话"
              @click="emit('new-session')"
            >
              <Plus class="size-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">新建会话</TooltipContent>
        </Tooltip>
      </TooltipProvider>
      <StatusPill
        v-if="sessions.activeSession"
        :status="sessions.activeSession.status"
        :polling="sessions.activeSession.polling"
        class="mr-1"
      />
    </div>

    <AlertDialog v-model:open="deleteOpen">
      <AlertDialogContent
        class="border-glass-border bg-[color:var(--glass-strong)] backdrop-blur-2xl"
      >
        <AlertDialogHeader>
          <AlertDialogTitle>删除会话</AlertDialogTitle>
          <AlertDialogDescription>
            将关闭「{{ pendingDelete?.name }}」并移除其全部读取定义与已缓存数据，此操作无法撤销。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDelete"
          >
            删除会话
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
