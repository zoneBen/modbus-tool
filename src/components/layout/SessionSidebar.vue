<script setup lang="ts">
import { Database, PencilLine, Plus, Trash2 } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import StatusPill from "@/components/common/StatusPill.vue";
import TransportGlyph from "@/components/common/TransportGlyph.vue";
import DefinitionDialog from "@/components/definition/DefinitionDialog.vue";
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
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import {
  formatFunctionCode,
  formatInterval,
  formatLatency,
  formatNumber,
  formatRange,
} from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { Definition } from "@/types/models";

const emit = defineEmits<{
  (event: "new-session"): void;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const isSlaveSession = computed(() => sessions.activeSession?.role === "slave");

const definitionDialogOpen = ref(false);
const editingDefinition = ref<Definition | null>(null);

function openCreateDefinition(): void {
  editingDefinition.value = null;
  definitionDialogOpen.value = true;
}

function openEditDefinition(definition: Definition): void {
  editingDefinition.value = definition;
  definitionDialogOpen.value = true;
}

const deleteTarget = ref<Definition | null>(null);
const deleteDialogOpen = ref(false);
const deletingDefinition = ref(false);

function requestDeleteDefinition(definition: Definition): void {
  deleteTarget.value = definition;
  deleteDialogOpen.value = true;
}

function closeDeleteDefinition(): void {
  deleteDialogOpen.value = false;
  deleteTarget.value = null;
}

async function confirmDeleteDefinition(): Promise<void> {
  const target = deleteTarget.value;
  if (!target || deletingDefinition.value) return;
  deletingDefinition.value = true;
  try {
    await sessions.deleteDefinition(target.id);
    toast.success(`已删除定义「${target.name}」`);
    closeDeleteDefinition();
  } catch (error: unknown) {
    toast.error("删除定义失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    deletingDefinition.value = false;
  }
}

const accentClass: Record<string, string> = {
  signal: "bg-signal",
  rx: "bg-rx",
  sim: "bg-sim",
};

function selectSession(sessionId: string): void {
  sessions.setActiveSession(sessionId);
}
</script>

<template>
  <aside class="flex h-full w-[268px] shrink-0 flex-col border-r border-hairline bg-overlay-1">
    <div class="flex h-[52px] shrink-0 items-center justify-between px-4">
      <div>
        <p class="text-[13px] font-semibold tracking-tight">{{ t("workspace.title") }}</p>
        <p class="text-[11px] text-muted-foreground">
          {{ sessions.masterCount }} 主站 · {{ sessions.slaveCount }} 从站
        </p>
      </div>
      <TooltipProvider :delay-duration="180">
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              variant="ghost"
              size="icon-sm"
              class="rounded-lg border border-hairline bg-overlay-1 text-muted-foreground hover:text-foreground"
              aria-label="新建会话"
              @click="emit('new-session')"
            >
              <Plus class="size-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{{ t("workspace.newSession") }}</TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </div>

    <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-2.5 pb-3">
      <div class="space-y-1.5">
        <button
          v-for="session in sessions.sessions"
          :key="session.id"
          type="button"
          class="group relative w-full rounded-xl border px-3 py-2.5 text-left transition-all duration-200"
          :class="
            session.id === sessions.activeSessionId
              ? 'border-hairline-strong bg-overlay-2 shadow-lg shadow-black/20'
              : 'border-transparent hover:border-hairline hover:bg-overlay-1'
          "
          @click="selectSession(session.id)"
        >
          <span
            v-if="session.id === sessions.activeSessionId"
            class="absolute inset-y-3 left-0 w-[2px] rounded-full"
            :class="accentClass[session.accent]"
          />
          <div class="flex items-center gap-2">
            <TransportGlyph
              :transport="session.transport"
              :role="session.role"
              class="size-[15px] shrink-0"
              :class="
                session.role === 'slave'
                  ? 'text-sim'
                  : session.transport === 'serial'
                    ? 'text-rx'
                    : 'text-signal-dim'
              "
            />
            <span class="min-w-0 flex-1 truncate text-[12.5px] font-medium">
              {{ session.name }}
            </span>
            <StatusPill :status="session.status" :polling="session.polling" compact />
          </div>
          <div class="mt-1 flex items-center gap-2 pl-[23px]">
            <span class="numeric min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
              {{ session.endpoint }}
            </span>
            <span
              v-if="session.role === 'master'"
              class="numeric text-[11px] text-muted-foreground"
            >
              {{ formatLatency(session.stats.latencyMs) }}
            </span>
            <span v-else class="text-[11px] font-medium uppercase tracking-wider text-sim/80">
              SLAVE {{ session.definitions[0]?.slaveId ?? 10 }}
            </span>
          </div>
          <div class="mt-1.5 flex items-center gap-3 pl-[23px] text-[11px] text-muted-foreground">
            <span
              >TX <span class="numeric">{{ formatNumber(session.stats.tx) }}</span></span
            >
            <span
              >RX <span class="numeric">{{ formatNumber(session.stats.rx) }}</span></span
            >
            <span v-if="session.stats.errors > 0" class="text-error/80">
              ERR <span class="numeric">{{ session.stats.errors }}</span>
            </span>
          </div>
        </button>
      </div>

      <div v-if="sessions.activeSession" class="mt-4">
        <div class="flex items-center justify-between px-2.5">
          <span class="label-caps">{{
            isSlaveSession ? "模拟映射" : t("workspace.definitions")
          }}</span>
          <div class="flex items-center gap-1">
            <span class="numeric text-[11px] text-muted-foreground">
              {{ sessions.activeDefinitions.length.toString().padStart(2, "0") }}
            </span>
            <button
              v-if="!isSlaveSession"
              type="button"
              class="flex size-5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
              aria-label="新建读取定义"
              @click="openCreateDefinition"
            >
              <Plus class="size-3" />
            </button>
          </div>
        </div>

        <div class="mt-1.5 space-y-0.5">
          <div
            v-for="definition in sessions.activeDefinitions"
            :key="definition.id"
            role="button"
            tabindex="0"
            class="group w-full cursor-pointer rounded-lg px-2.5 py-2 text-left transition-colors"
            :class="
              definition.id === sessions.activeDefinitionId ? 'bg-overlay-2' : 'hover:bg-overlay-1'
            "
            @click="sessions.setActiveDefinition(definition.id)"
            @keydown.enter="sessions.setActiveDefinition(definition.id)"
            @keydown.space.prevent="sessions.setActiveDefinition(definition.id)"
          >
            <div class="flex items-center gap-2">
              <span
                class="numeric rounded border border-hairline bg-well px-1 py-0.5 text-[11px] font-medium uppercase tracking-wide"
                :class="
                  definition.dataType === 'f32' || definition.dataType === 'f64'
                    ? 'text-rx'
                    : 'text-signal-dim'
                "
              >
                {{ definition.dataType.toUpperCase() }}
              </span>
              <span
                class="min-w-0 flex-1 truncate text-[12px]"
                :class="definition.enabled ? '' : 'text-muted-foreground'"
              >
                {{ definition.name }}
              </span>
              <template v-if="!isSlaveSession">
                <button
                  type="button"
                  class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
                  aria-label="编辑读取定义"
                  @click.stop="openEditDefinition(definition)"
                >
                  <PencilLine class="size-3" />
                </button>
                <button
                  type="button"
                  class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
                  aria-label="删除读取定义"
                  @click.stop="requestDeleteDefinition(definition)"
                >
                  <Trash2 class="size-3" />
                </button>
                <Switch
                  :model-value="definition.enabled"
                  class="scale-[0.72]"
                  @click.stop
                  @update:model-value="
                    (checked) => sessions.toggleDefinition(definition.id, Boolean(checked))
                  "
                />
              </template>
              <span
                v-else
                class="rounded-full border border-sim/25 bg-sim/10 px-1.5 py-0.5 text-[11px] text-sim"
              >
                SIM
              </span>
            </div>
            <div class="mt-1 flex items-center gap-2 pl-[38px] text-[11px] text-muted-foreground">
              <span class="numeric">{{ formatFunctionCode(definition.functionCode) }}</span>
              <span class="numeric">{{
                formatRange(definition.startAddress, definition.quantity, settings.addressBase)
              }}</span>
              <span class="ml-auto numeric">{{ formatInterval(definition.intervalMs) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <DefinitionDialog v-model:open="definitionDialogOpen" :definition="editingDefinition" />

    <AlertDialog
      :open="deleteDialogOpen"
      @update:open="
        (open) => {
          if (!open) closeDeleteDefinition();
        }
      "
    >
      <AlertDialogContent
        class="border-glass-border bg-[color:var(--glass-strong)] backdrop-blur-2xl"
      >
        <AlertDialogHeader>
          <AlertDialogTitle>删除读取定义</AlertDialogTitle>
          <AlertDialogDescription>
            将停止轮询并移除「{{ deleteTarget?.name }}」及其已缓存数据，此操作无法撤销。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel :disabled="deletingDefinition">取消</AlertDialogCancel>
          <Button
            variant="destructive"
            class="text-white"
            :disabled="deletingDefinition"
            @click="confirmDeleteDefinition"
          >
            {{ deletingDefinition ? "删除中…" : "删除定义" }}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <div class="flex shrink-0 items-center justify-between border-t border-hairline px-3.5 py-2.5">
      <div class="flex items-center gap-1.5 text-[11px] text-muted-foreground">
        <Database class="size-3" />
        <span>{{ t("workspace.autosave") }}</span>
      </div>
      <span
        v-if="sessions.demo"
        class="rounded-full border border-sim/25 bg-sim/10 px-1.5 py-0.5 text-[11px] font-medium text-sim"
      >
        DEMO
      </span>
    </div>
  </aside>
</template>
