<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import AboutOverlay from "@/components/about/AboutOverlay.vue";
import AlertsWorkspace from "@/components/alerts/AlertsWorkspace.vue";
import TrendOverlay from "@/components/chart/TrendOverlay.vue";
import DevToolsWorkspace from "@/components/tools/DevToolsWorkspace.vue";
import SecurityToolsWorkspace from "@/components/tools/SecurityToolsWorkspace.vue";
import NetworkToolsWorkspace from "@/components/tools/NetworkToolsWorkspace.vue";
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
import DebugDialog from "@/components/debug/DebugDialog.vue";
import ActivityRail from "@/components/layout/ActivityRail.vue";
import SessionSidebar from "@/components/layout/SessionSidebar.vue";
import SessionTabs from "@/components/layout/SessionTabs.vue";
import SessionToolbar from "@/components/layout/SessionToolbar.vue";
import StatusBar from "@/components/layout/StatusBar.vue";
import WriteDialog from "@/components/definition/WriteDialog.vue";
import RegisterGrid from "@/components/grid/RegisterGrid.vue";
import InspectorPanel from "@/components/inspector/InspectorPanel.vue";
import FrameMonitor from "@/components/monitor/FrameMonitor.vue";
import NetworkWorkspace from "@/components/network/NetworkWorkspace.vue";
import ScanWorkspace from "@/components/scan/ScanWorkspace.vue";
import EditSessionDialog from "@/components/session/EditSessionDialog.vue";
import NewSessionDialog from "@/components/session/NewSessionDialog.vue";
import SettingsDialog from "@/components/settings/SettingsDialog.vue";
import { useRealtimeEvents } from "@/composables/useRealtimeEvents";
import { buildAppConfig, parseAppConfig } from "@/lib/app-config";
import type { AppConfigFile } from "@/lib/app-config";
import { clamp } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { useAlertsStore } from "@/stores/alerts";
import { useMqttStore } from "@/stores/mqtt";
import { useNetworkStore } from "@/stores/network";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import { useToolboxStore } from "@/stores/toolbox";
import type { WriteTarget } from "@/types/ui";

useRealtimeEvents();

const settings = useSettingsStore();
const sessions = useSessionsStore();
const network = useNetworkStore();
const alerts = useAlertsStore();
const toolbox = useToolboxStore();
const mqtt = useMqttStore();
const { t } = useI18n();
const initialParams = new URLSearchParams(window.location.search);
const initialDialog = initialParams.get("dialog");
const initialPageParam = initialParams.get("page");
const initialTool = initialParams.get("tool");
type PageView =
  "workspace" | "network" | "scan" | "alerts" | "networkTools" | "devTools" | "securityTools";
const initialPage: PageView =
  initialPageParam === "net-tools"
    ? "networkTools"
    : initialPageParam === "dev-tools"
      ? "devTools"
      : initialPageParam === "sec-tools"
        ? "securityTools"
        : initialDialog === "network" || initialDialog === "scan" || initialDialog === "alerts"
          ? initialDialog
          : "workspace";
const page = ref<PageView>(initialPage);
const newSessionOpen = ref(initialDialog === "session");
const settingsOpen = ref(initialDialog === "settings");
const writeOpen = ref(false);
const writeTarget = ref<WriteTarget | null>(null);
const trendOpen = ref(false);
const editSessionOpen = ref(false);
const debugOpen = ref(false);
const aboutOpen = ref(false);
const configImportOpen = ref(false);
const importingConfig = ref(false);
const pendingConfig = ref<AppConfigFile | null>(null);
const railView = computed<
  | "workspace"
  | "network"
  | "scan"
  | "alerts"
  | "networkTools"
  | "devTools"
  | "securityTools"
  | "settings"
  | "about"
>(() => {
  if (settingsOpen.value) return "settings";
  if (aboutOpen.value) return "about";
  return page.value;
});

const monitorStyle = computed(() => ({
  height: settings.monitorCollapsed ? "38px" : `${settings.monitorHeight}px`,
}));

function openDebug(): void {
  const session = sessions.activeSession;
  if (!session) return;
  if (session.role !== "master") {
    toast.info(t("debug.masterOnly"));
    return;
  }
  if (session.status !== "connected") {
    toast.info(t("debug.prerequisiteConnect"));
    return;
  }
  if (session.polling) {
    toast.info(t("debug.prerequisitePolling"));
    return;
  }
  debugOpen.value = true;
}

function openWrite(target: WriteTarget): void {
  writeTarget.value = target;
  writeOpen.value = true;
}

function openWriteFromToolbar(): void {
  const definition = sessions.activeDefinition;
  if (!definition) {
    toast.info(t("toolbar.selectDefinition"));
    return;
  }
  const firstValue =
    sessions.activeSnapshot?.values.find((value) => value.numeric !== null) ??
    sessions.activeSnapshot?.values[0];
  const address = firstValue?.address ?? definition.startAddress;
  openWrite({
    address,
    alias: definition.aliases[address] ?? t("grid.register", { address }),
    current: firstValue?.formatted ?? "—",
    dataType: definition.dataType,
  });
}

function toggleMonitor(): void {
  settings.monitorCollapsed = !settings.monitorCollapsed;
}

function configErrorMessage(message: string): string {
  if (
    message === "invalidJson" ||
    message === "invalidConfigKind" ||
    message === "unsupportedConfigVersion" ||
    message === "invalidConfigShape"
  ) {
    return t(`config.errors.${message}`);
  }
  return message;
}

async function exportConfig(): Promise<void> {
  const config = buildAppConfig(
    settings.snapshot(),
    sessions.persistedSessions(),
    network.exportSessions(),
    toolbox.exportConfig(),
    mqtt.exportConfig(),
  );
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  try {
    const saved = await ipc.saveTextFile(
      `modbus-tool-config-${stamp}.json`,
      JSON.stringify(config, null, 2),
      "json",
    );
    if (saved) {
      toast.success(t("config.exported"), {
        description: t("config.exportedHint", {
          sessions: config.sessions.length,
          network: config.networkSessions.length,
        }),
      });
    }
  } catch (cause: unknown) {
    toast.error(t("config.exportFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  }
}

async function importConfig(): Promise<void> {
  try {
    const text = await ipc.openTextFile();
    if (!text) return;
    pendingConfig.value = parseAppConfig(text);
    configImportOpen.value = true;
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    toast.error(t("config.importFailed"), {
      description: configErrorMessage(message),
    });
  }
}

function cancelConfigImport(): void {
  if (importingConfig.value) return;
  configImportOpen.value = false;
  pendingConfig.value = null;
}

async function confirmConfigImport(): Promise<void> {
  const config = pendingConfig.value;
  if (!config || importingConfig.value) return;
  importingConfig.value = true;
  try {
    settings.applySnapshot(config.settings);
    const errors = await sessions.replaceWorkspace(config.sessions);
    await network.replaceSessions(config.networkSessions);
    toolbox.replaceConfig(config.tools);
    await mqtt.replaceConfig(config.mqtt);
    alerts.clear();
    configImportOpen.value = false;
    pendingConfig.value = null;
    page.value = "workspace";
    if (errors.length) {
      toast.warning(t("config.importedWithErrors"), {
        description: errors[0],
      });
    } else {
      toast.success(t("config.imported"), {
        description: t("config.importedHint", {
          sessions: config.sessions.length,
          network: config.networkSessions.length,
        }),
      });
    }
  } catch (cause: unknown) {
    toast.error(t("config.importFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  } finally {
    importingConfig.value = false;
  }
}

let resizeState: { startY: number; startHeight: number } | null = null;

function onResize(event: PointerEvent): void {
  if (!resizeState) return;
  const next = resizeState.startHeight + (resizeState.startY - event.clientY);
  settings.monitorHeight = clamp(next, 160, 520);
}

function stopResize(): void {
  resizeState = null;
  window.removeEventListener("pointermove", onResize);
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
}

function startResize(event: PointerEvent): void {
  if (settings.monitorCollapsed) return;
  event.preventDefault();
  resizeState = { startY: event.clientY, startHeight: settings.monitorHeight };
  document.body.style.cursor = "row-resize";
  document.body.style.userSelect = "none";
  window.addEventListener("pointermove", onResize);
  window.addEventListener("pointerup", stopResize, { once: true });
}
</script>

<template>
  <div
    class="relative flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground"
    :class="{ 'no-blur': settings.reduceEffects }"
  >
    <div class="pointer-events-none fixed inset-0 instrument-grid opacity-45" aria-hidden="true" />
    <div
      class="pointer-events-none fixed inset-x-0 top-0 h-40 bg-gradient-to-b from-overlay-1 to-transparent"
      aria-hidden="true"
    />

    <div class="relative flex min-h-0 flex-1">
      <ActivityRail
        :active-view="railView"
        @open-workspace="page = 'workspace'"
        @open-settings="settingsOpen = true"
        @open-network="page = 'network'"
        @open-scan="page = 'scan'"
        @open-alerts="page = 'alerts'"
        @open-network-tools="page = 'networkTools'"
        @open-dev-tools="page = 'devTools'"
        @open-security-tools="page = 'securityTools'"
        @open-about="aboutOpen = true"
        @import-config="importConfig"
        @export-config="exportConfig"
      />

      <template v-if="page === 'workspace'">
        <SessionSidebar @new-session="newSessionOpen = true" />

        <main class="flex min-w-0 flex-1 flex-col">
          <SessionTabs />
          <SessionToolbar
            @write="openWriteFromToolbar"
            @debug="openDebug"
            @edit-session="editSessionOpen = true"
          />

          <div class="flex min-h-0 flex-1">
            <RegisterGrid
              class="min-w-0 flex-1"
              @write="openWrite"
              @new-session="newSessionOpen = true"
              @open-trend="trendOpen = true"
            />
            <InspectorPanel />
          </div>

          <div
            v-if="!settings.monitorCollapsed"
            class="group relative h-[3px] shrink-0 cursor-row-resize border-y border-hairline bg-overlay-1 transition-colors hover:bg-signal/15"
            role="separator"
            aria-orientation="horizontal"
            @pointerdown="startResize"
          >
            <div
              class="absolute left-1/2 top-1/2 h-[2px] w-9 -translate-x-1/2 -translate-y-1/2 rounded-full bg-overlay-3 transition-colors group-hover:bg-signal/60"
            />
          </div>

          <FrameMonitor
            :collapsed="settings.monitorCollapsed"
            :style="monitorStyle"
            @toggle="toggleMonitor"
          />

          <StatusBar />
        </main>
      </template>

      <NetworkWorkspace v-else-if="page === 'network'" />
      <ScanWorkspace v-else-if="page === 'scan'" />
      <AlertsWorkspace v-else-if="page === 'alerts'" />
      <NetworkToolsWorkspace v-else-if="page === 'networkTools'" :initial-tool="initialTool" />
      <DevToolsWorkspace v-else-if="page === 'devTools'" :initial-tool="initialTool" />
      <SecurityToolsWorkspace v-else-if="page === 'securityTools'" :initial-tool="initialTool" />
    </div>

    <NewSessionDialog v-model:open="newSessionOpen" />
    <EditSessionDialog v-model:open="editSessionOpen" />
    <DebugDialog v-model:open="debugOpen" />
    <SettingsDialog v-model:open="settingsOpen" />
    <WriteDialog v-model:open="writeOpen" :target="writeTarget" />
    <TrendOverlay v-model:open="trendOpen" />
    <AboutOverlay v-model:open="aboutOpen" />

    <AlertDialog
      :open="configImportOpen"
      @update:open="
        (open) => {
          if (!open) cancelConfigImport();
        }
      "
    >
      <AlertDialogContent
        class="border-glass-border bg-[color:var(--glass-strong)] backdrop-blur-2xl"
      >
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t("config.importTitle") }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{
              t("config.importDescription", {
                sessions: pendingConfig?.sessions.length ?? 0,
                network: pendingConfig?.networkSessions.length ?? 0,
              })
            }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel :disabled="importingConfig">
            {{ t("common.cancel") }}
          </AlertDialogCancel>
          <Button class="gap-1.5" :disabled="importingConfig" @click="confirmConfigImport">
            {{ importingConfig ? t("config.importing") : t("config.importConfirm") }}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
