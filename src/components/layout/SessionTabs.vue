<script setup lang="ts">
import { Plus } from "@lucide/vue";
import StatusPill from "@/components/common/StatusPill.vue";
import TransportGlyph from "@/components/common/TransportGlyph.vue";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useSessionsStore } from "@/stores/sessions";

const emit = defineEmits<{
  (event: "new-session"): void;
}>();

const sessions = useSessionsStore();

function select(sessionId: string): void {
  sessions.setActiveSession(sessionId);
}
</script>

<template>
  <div class="flex h-[46px] shrink-0 items-stretch border-b border-white/[0.06] bg-black/10">
    <div class="scrollbar-thin flex min-w-0 flex-1 items-stretch overflow-x-auto">
      <button
        v-for="session in sessions.sessions"
        :key="session.id"
        type="button"
        class="relative flex min-w-[172px] max-w-[230px] items-center gap-2 border-r border-white/[0.04] px-3.5 text-left transition-colors"
        :class="
          session.id === sessions.activeSessionId ? 'bg-white/[0.045]' : 'hover:bg-white/[0.025]'
        "
        @click="select(session.id)"
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
          <p class="truncate text-[11.5px] font-medium">{{ session.name }}</p>
          <p class="numeric truncate text-[9px] text-muted-foreground/55">{{ session.endpoint }}</p>
        </div>
        <StatusPill :status="session.status" :polling="session.polling" compact />
        <span
          v-if="session.id === sessions.activeSessionId"
          class="absolute inset-x-0 bottom-0 h-[1.5px] bg-gradient-to-r from-signal/70 via-signal to-rx/70"
        />
      </button>
    </div>

    <div class="flex shrink-0 items-center gap-1.5 border-l border-white/[0.06] px-2.5">
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
  </div>
</template>
