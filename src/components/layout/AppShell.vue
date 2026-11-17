<script setup lang="ts">
import { computed, ref } from "vue";
import { toast } from "vue-sonner";
import AboutOverlay from "@/components/about/AboutOverlay.vue";
import TrendOverlay from "@/components/chart/TrendOverlay.vue";
import ActivityRail from "@/components/layout/ActivityRail.vue";
import SessionSidebar from "@/components/layout/SessionSidebar.vue";
import SessionTabs from "@/components/layout/SessionTabs.vue";
import SessionToolbar from "@/components/layout/SessionToolbar.vue";
import StatusBar from "@/components/layout/StatusBar.vue";
import WriteDialog from "@/components/definition/WriteDialog.vue";
import RegisterGrid from "@/components/grid/RegisterGrid.vue";
import InspectorPanel from "@/components/inspector/InspectorPanel.vue";
import FrameMonitor from "@/components/monitor/FrameMonitor.vue";
import ScanDialog from "@/components/scan/ScanDialog.vue";
import EditSessionDialog from "@/components/session/EditSessionDialog.vue";
import NewSessionDialog from "@/components/session/NewSessionDialog.vue";
import SettingsDialog from "@/components/settings/SettingsDialog.vue";
import { useRealtimeEvents } from "@/composables/useRealtimeEvents";
import { clamp } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { WriteTarget } from "@/types/ui";

useRealtimeEvents();

const settings = useSettingsStore();
const sessions = useSessionsStore();
const initialDialog = new URLSearchParams(window.location.search).get("dialog");
const newSessionOpen = ref(initialDialog === "session");
const settingsOpen = ref(initialDialog === "settings");
const scanOpen = ref(initialDialog === "scan");
const writeOpen = ref(false);
const writeTarget = ref<WriteTarget | null>(null);
const trendOpen = ref(false);
const editSessionOpen = ref(false);
const aboutOpen = ref(false);

const monitorStyle = computed(() => ({
  height: settings.monitorCollapsed ? "79px" : `${settings.monitorHeight}px`,
}));

function openWrite(target: WriteTarget): void {
  writeTarget.value = target;
  writeOpen.value = true;
}

function openWriteFromToolbar(): void {
  const definition = sessions.activeDefinition;
  if (!definition) {
    toast.info("请先选择或新建一个读取定义");
    return;
  }
  const firstValue =
    sessions.activeSnapshot?.values.find((value) => value.numeric !== null) ??
    sessions.activeSnapshot?.values[0];
  const address = firstValue?.address ?? definition.startAddress;
  openWrite({
    address,
    alias: definition.aliases[address] ?? `寄存器 ${address}`,
    current: firstValue?.formatted ?? "—",
    dataType: definition.dataType,
  });
}

function toggleMonitor(): void {
  settings.monitorCollapsed = !settings.monitorCollapsed;
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
        @open-settings="settingsOpen = true"
        @open-scan="scanOpen = true"
        @open-about="aboutOpen = true"
      />
      <SessionSidebar @new-session="newSessionOpen = true" />

      <main class="flex min-w-0 flex-1 flex-col">
        <SessionTabs />
        <SessionToolbar
          @write="openWriteFromToolbar"
          @edit-session="editSessionOpen = true"
          @open-settings="settingsOpen = true"
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
    </div>

    <NewSessionDialog v-model:open="newSessionOpen" />
    <EditSessionDialog v-model:open="editSessionOpen" />
    <SettingsDialog v-model:open="settingsOpen" />
    <ScanDialog v-model:open="scanOpen" />
    <WriteDialog v-model:open="writeOpen" :target="writeTarget" />
    <TrendOverlay v-model:open="trendOpen" />
    <AboutOverlay v-model:open="aboutOpen" />
  </div>
</template>
