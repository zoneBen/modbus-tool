<script setup lang="ts">
import { Check, Copy, RefreshCw, ServerCog } from "@lucide/vue";
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { PortEntry, PortListResult } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const filter = toolbox.toolPref("security.ports.filter", "");
const listeningOnly = toolbox.toolPref("security.ports.listeningOnly", true);
const busy = ref(false);
const error = ref<string | null>(null);
const result = ref<PortListResult | null>(null);

async function refresh(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    result.value = await ipc.toolPortList({
      listeningOnly: listeningOnly.value,
      filter: filter.value.trim() || undefined,
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}

function address(entry: PortEntry): string {
  const host = entry.localAddr.includes(":") ? `[${entry.localAddr}]` : entry.localAddr;
  return `${host}:${entry.localPort}`;
}

function copyRow(entry: PortEntry): string {
  return `${entry.protocol}\t${address(entry)}\t${entry.state}\t${entry.pid}\t${entry.process ?? ""}`;
}

onMounted(refresh);
</script>

<template>
  <ToolPanelFrame
    :title="t('securityTools.tools.ports.title')"
    :description="t('securityTools.tools.ports.description')"
    :icon="ServerCog"
  >
    <div class="mx-auto max-w-[1120px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="min-w-[220px] flex-1 space-y-1.5">
          <p class="text-[11px] text-muted-foreground">{{ t("securityTools.ports.filter") }}</p>
          <Input
            v-model="filter"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            :placeholder="t('securityTools.ports.filterPlaceholder')"
            @keydown.enter="refresh"
          />
        </div>
        <label class="flex items-center gap-2 pb-2 text-[11px]">
          <Switch v-model="listeningOnly" />
          {{ t("securityTools.ports.listeningOnly") }}
        </label>
        <Button size="sm" class="h-9 gap-1.5 text-[11px]" :disabled="busy" @click="refresh">
          <RefreshCw class="size-3.5" :class="{ 'animate-spin': busy }" />
          {{ busy ? t("tools.running") : t("securityTools.ports.refresh") }}
        </Button>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="rounded-xl border border-hairline">
        <div class="flex items-center gap-2 border-b border-hairline px-3 py-2 text-[11px]">
          <span class="text-muted-foreground">
            {{ t("securityTools.ports.count", { count: result.entries.length }) }}
          </span>
          <span
            v-if="result.truncated"
            class="rounded border border-warn/25 bg-warn/10 px-1.5 py-0.5 text-[9px] text-warn"
          >
            {{ t("securityTools.ports.truncated") }}
          </span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
            :disabled="!result.entries.length"
            @click="copy('ports', result.entries.map(copyRow).join('\n'))"
          >
            <Check v-if="isCopied('ports')" class="size-3 text-signal" />
            <Copy v-else class="size-3" />
            {{ isCopied("ports") ? t("common.copiedShort") : t("tools.copy") }}
          </Button>
        </div>
        <div class="scrollbar-thin max-h-[520px] overflow-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="sticky top-0 bg-surface">
              <tr class="border-b border-hairline text-left text-muted-foreground">
                <th class="px-2 py-2 font-medium">PROTO</th>
                <th class="px-2 py-2 font-medium">{{ t("securityTools.ports.address") }}</th>
                <th class="px-2 py-2 font-medium">PID</th>
                <th class="px-2 py-2 font-medium">{{ t("securityTools.ports.process") }}</th>
                <th class="px-2 py-2 font-medium">{{ t("securityTools.ports.state") }}</th>
                <th class="w-10 px-2 py-2"></th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(entry, index) in result.entries"
                :key="`${entry.protocol}-${entry.localAddr}-${entry.localPort}-${entry.pid}-${index}`"
                class="border-b border-hairline/60 last:border-0 hover:bg-overlay-1"
              >
                <td class="px-2 py-1">
                  <span
                    class="rounded border border-hairline px-1 py-0.5 text-[9px]"
                    :class="entry.protocol === 'TCP' ? 'text-rx' : 'text-sim'"
                    >{{ entry.protocol }}</span
                  >
                </td>
                <td class="numeric px-2 py-1 text-foreground">{{ address(entry) }}</td>
                <td class="numeric px-2 py-1 text-muted-foreground">{{ entry.pid || "—" }}</td>
                <td class="px-2 py-1 text-muted-foreground">{{ entry.process || "—" }}</td>
                <td class="numeric px-2 py-1 text-muted-foreground">{{ entry.state }}</td>
                <td class="px-2 py-1">
                  <button
                    type="button"
                    class="text-muted-foreground hover:text-foreground"
                    @click="copy(`port:${index}`, copyRow(entry))"
                  >
                    <Check v-if="isCopied(`port:${index}`)" class="size-3 text-signal" />
                    <Copy v-else class="size-3" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <p
            v-if="!result.entries.length"
            class="px-3 py-8 text-center text-[11px] text-muted-foreground"
          >
            {{ t("securityTools.ports.empty") }}
          </p>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
