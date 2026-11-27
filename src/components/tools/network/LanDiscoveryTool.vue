<script setup lang="ts">
import { Check, Copy, Radar, Search } from "@lucide/vue";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { DiscoveredDevice, DiscoveryResult } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const timeoutMs = toolbox.toolPref("network.discovery.timeout", "4000");
const mdns = toolbox.toolPref("network.discovery.mdns", true);
const ssdp = toolbox.toolPref("network.discovery.ssdp", true);
const busy = ref(false);
const error = ref<string | null>(null);
const result = ref<DiscoveryResult | null>(null);

const sourceTone: Record<string, string> = {
  mdns: "border-rx/25 bg-rx/10 text-rx",
  ssdp: "border-sim/25 bg-sim/10 text-sim",
  upnp: "border-signal/25 bg-signal/10 text-signal",
};

async function scan(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    result.value = await ipc.toolLanDiscovery({
      timeoutMs: Math.min(15_000, Math.max(1_000, Number(timeoutMs.value) || 4000)),
      mdns: mdns.value,
      ssdp: ssdp.value,
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}

function deviceAddress(device: DiscoveredDevice): string {
  const address = device.address || device.host || "";
  return device.port ? `${address}:${device.port}` : address;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.discovery.title')"
    :description="t('networkTools.tools.discovery.description')"
    :icon="Radar"
  >
    <div class="mx-auto max-w-[1080px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-4 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.discovery.timeout")
          }}</Label>
          <Input
            v-model="timeoutMs"
            class="numeric h-9 w-28 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <label class="flex items-center gap-2 text-[11.5px]">
          <Switch v-model="mdns" />
          <span>mDNS</span>
        </label>
        <label class="flex items-center gap-2 text-[11.5px]">
          <Switch v-model="ssdp" />
          <span>SSDP / UPnP</span>
        </label>
        <Button size="sm" class="ml-auto h-9 gap-1.5 text-[12px]" :disabled="busy" @click="scan">
          <Search class="size-3.5" />
          {{ busy ? t("tools.running") : t("networkTools.discovery.scan") }}
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
            {{ t("networkTools.discovery.found", { count: result.devices.length }) }}
          </span>
          <span class="numeric ml-auto text-muted-foreground">{{ result.elapsedMs }} ms</span>
        </div>
        <div v-if="result.devices.length" class="overflow-x-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline text-left text-muted-foreground">
                <th class="px-3 py-2 font-medium">{{ t("networkTools.discovery.name") }}</th>
                <th class="px-2 py-2 font-medium">{{ t("networkTools.discovery.address") }}</th>
                <th class="px-2 py-2 font-medium">{{ t("networkTools.discovery.service") }}</th>
                <th class="px-2 py-2 font-medium">{{ t("networkTools.discovery.source") }}</th>
                <th class="px-3 py-2 font-medium">Model</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="device in result.devices"
                :key="`${device.source}-${device.name}-${device.address}`"
                class="border-b border-hairline last:border-0"
              >
                <td class="px-3 py-2">
                  <p class="font-medium text-foreground">{{ device.name }}</p>
                  <p v-if="device.manufacturer" class="text-[10px] text-muted-foreground">
                    {{ device.manufacturer }}
                  </p>
                </td>
                <td class="numeric px-2 py-2 text-muted-foreground">
                  <div class="flex items-center gap-1">
                    {{ deviceAddress(device) || "—" }}
                    <button
                      v-if="deviceAddress(device)"
                      type="button"
                      class="flex items-center gap-1 text-muted-foreground hover:text-foreground"
                      @click="copy(`device:${device.address}`, deviceAddress(device))"
                    >
                      <Check
                        v-if="isCopied(`device:${device.address}`)"
                        class="size-3 text-signal"
                      />
                      <Copy v-else class="size-3" />
                      <span
                        v-if="isCopied(`device:${device.address}`)"
                        class="text-[9px] text-signal"
                        >{{ t("common.copiedShort") }}</span
                      >
                    </button>
                  </div>
                </td>
                <td class="break-all px-2 py-2 text-muted-foreground">
                  {{ device.service || "—" }}
                </td>
                <td class="px-2 py-2">
                  <span
                    class="rounded border px-1.5 py-0.5 text-[9px]"
                    :class="sourceTone[device.source]"
                  >
                    {{ device.source }}
                  </span>
                </td>
                <td class="px-3 py-2 text-muted-foreground">{{ device.model || "—" }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p v-else class="px-3 py-8 text-center text-[11px] text-muted-foreground">
          {{ t("networkTools.discovery.empty") }}
        </p>
      </div>

      <div
        v-else-if="!busy"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.discovery.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
