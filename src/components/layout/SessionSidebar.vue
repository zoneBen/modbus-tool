<script setup lang="ts">
import { Database, Download, PencilLine, Plus, Trash2, Upload } from "@lucide/vue";
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
import { toDefinitionInput } from "@/lib/definition";
import {
  formatFunctionCode,
  formatInterval,
  formatLatency,
  formatNumber,
  formatRange,
} from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { Definition, DefinitionInput } from "@/types/models";

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
    toast.success(t("workspace.deletedDefinition", { name: target.name }));
    closeDeleteDefinition();
  } catch (error: unknown) {
    toast.error(t("workspace.deleteDefinitionFailed"), {
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

const ioBusy = ref(false);

async function exportDefinitions(): Promise<void> {
  const session = sessions.activeSession;
  if (!session || !sessions.activeDefinitions.length) {
    toast.info(t("workspace.exportEmpty"));
    return;
  }
  ioBusy.value = true;
  try {
    const payload = {
      version: 1,
      kind: session.role === "slave" ? "modbus-simulator-config" : "modbus-definition-bundle",
      session: session.name,
      exportedAt: new Date().toISOString(),
      definitions: sessions.activeDefinitions.map(toDefinitionInput),
    };
    const suffix = session.role === "slave" ? "simulator" : "definitions";
    const name = `${session.name.replace(/[\\/:*?"<>|\s]+/g, "_")}-${suffix}.json`;
    const saved = await ipc.saveTextFile(name, JSON.stringify(payload, null, 2), "json");
    if (saved) {
      toast.success(t("workspace.exportedDefinitions"), {
        description: t("workspace.exportedCount", {
          count: sessions.activeDefinitions.length,
        }),
      });
    }
  } catch (error: unknown) {
    toast.error(t("workspace.exportDefinitionsFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    ioBusy.value = false;
  }
}

async function importDefinitions(): Promise<void> {
  const session = sessions.activeSession;
  if (!session) return;
  ioBusy.value = true;
  try {
    const text = await ipc.openTextFile();
    if (!text) return;
    const parsed = JSON.parse(text) as { definitions?: unknown } | unknown[];
    const rawDefinitions = Array.isArray(parsed) ? parsed : parsed.definitions;
    if (!Array.isArray(rawDefinitions) || !rawDefinitions.length) {
      toast.error(t("workspace.invalidDefinitionFile"));
      return;
    }
    const definitions = rawDefinitions as DefinitionInput[];
    const errors: string[] = [];
    if (session.role === "slave") {
      const existing = session.definitions[0];
      if (existing && definitions[0]) {
        await sessions.updateDefinition(existing.id, definitions[0]);
      }
      toast.success(t("workspace.importedDefinitions"), {
        description: t("workspace.importedSimulatorHint"),
      });
    } else {
      for (const definition of definitions) {
        try {
          await sessions.createDefinition(definition);
        } catch (error: unknown) {
          errors.push(
            `${definition.name ?? t("workspace.unnamed")}: ${
              error instanceof Error ? error.message : String(error)
            }`,
          );
        }
      }
      if (errors.length) {
        toast.error(t("workspace.importDefinitionsFailed"), {
          description: errors[0],
        });
      } else {
        toast.success(t("workspace.importedDefinitions"), {
          description: t("workspace.importedCount", { count: definitions.length }),
        });
      }
    }
  } catch (error: unknown) {
    toast.error(t("workspace.importDefinitionsFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    ioBusy.value = false;
  }
}

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
          {{
            t("workspace.masterSlaveCount", {
              master: sessions.masterCount,
              slave: sessions.slaveCount,
            })
          }}
        </p>
      </div>
      <TooltipProvider :delay-duration="180">
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              variant="ghost"
              size="icon-sm"
              class="rounded-lg border border-hairline bg-overlay-1 text-muted-foreground hover:text-foreground"
              :aria-label="t('workspace.newSession')"
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
            isSlaveSession ? t("workspace.simulatorMapping") : t("workspace.definitions")
          }}</span>
          <div class="flex items-center gap-1">
            <button
              type="button"
              class="flex size-5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground disabled:opacity-40"
              :disabled="ioBusy"
              :aria-label="t('workspace.importDefinitions')"
              :title="t('workspace.importDefinitions')"
              @click="importDefinitions"
            >
              <Upload class="size-3" />
            </button>
            <button
              type="button"
              class="flex size-5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground disabled:opacity-40"
              :disabled="ioBusy || !sessions.activeDefinitions.length"
              :aria-label="t('workspace.exportDefinitions')"
              :title="t('workspace.exportDefinitions')"
              @click="exportDefinitions"
            >
              <Download class="size-3" />
            </button>
            <span class="numeric text-[11px] text-muted-foreground">
              {{ sessions.activeDefinitions.length.toString().padStart(2, "0") }}
            </span>
            <button
              v-if="!isSlaveSession"
              type="button"
              class="flex size-5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
              :aria-label="t('workspace.newDefinition')"
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
                  :aria-label="t('workspace.editDefinition')"
                  @click.stop="openEditDefinition(definition)"
                >
                  <PencilLine class="size-3" />
                </button>
                <button
                  type="button"
                  class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
                  :aria-label="t('workspace.deleteDefinition')"
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
              <span class="numeric">
                {{
                  isSlaveSession && definition.quantity === 0
                    ? t("workspace.mappingUnconfigured")
                    : formatRange(
                        definition.startAddress,
                        definition.quantity,
                        settings.addressBase,
                      )
                }}
              </span>
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
          <AlertDialogTitle>{{ t("workspace.deleteDefinitionTitle") }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{
              t("workspace.deleteDefinitionDescription", {
                name: deleteTarget?.name ?? "",
              })
            }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel :disabled="deletingDefinition">
            {{ t("common.cancel") }}
          </AlertDialogCancel>
          <Button
            variant="destructive"
            class="text-white"
            :disabled="deletingDefinition"
            @click="confirmDeleteDefinition"
          >
            {{
              deletingDefinition
                ? t("workspace.deletingDefinition")
                : t("workspace.deleteDefinition")
            }}
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
