<script setup lang="ts">
import { Radar } from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import type { ArpCapabilities, ArpScanResult } from "@/types/tools";

const { t } = useI18n();
const capabilities = ref<ArpCapabilities | null>(null);
const adapterName = ref("");
const interfaceName = ref("");
const cidr = ref("192.168.1.0/24");
const timeoutMs = ref("2500");
const concurrency = ref("32");
const enhanced = ref(false);
const busy = ref(false);
const loading = ref(true);
const result = ref<ArpScanResult | null>(null);
const error = ref<string | null>(null);

const adapterOptions = computed(() =>
  (capabilities.value?.adapters ?? []).map((adapter) => ({
    label: `${adapter.description} · ${adapter.ip} · ${adapter.mac}`,
    value: adapter.name,
  })),
);

const interfaceOptions = computed(() =>
  (capabilities.value?.npcapInterfaces ?? []).map((item) => ({
    label: item.description || item.name,
    value: item.name,
  })),
);

const selectedAdapter = computed(
  () => capabilities.value?.adapters.find((adapter) => adapter.name === adapterName.value) ?? null,
);

const enhancedAvailable = computed(() =>
  Boolean(capabilities.value?.npcapAvailable && capabilities.value?.elevated),
);

const enhancedHint = computed(() => {
  if (capabilities.value?.npcapAvailable && capabilities.value?.elevated) return "";
  if (!capabilities.value?.npcapAvailable) {
    return capabilities.value?.reason || t("networkTools.arp.npcapMissing");
  }
  return t("networkTools.arp.adminRequired");
});

onMounted(async () => {
  try {
    capabilities.value = await ipc.toolArpCapabilities();
    const first = capabilities.value.adapters[0];
    if (first) {
      adapterName.value = first.name;
    }
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  } finally {
    loading.value = false;
  }
});

watch(adapterName, () => {
  const adapter = selectedAdapter.value;
  if (!adapter) return;
  cidr.value = adapter.cidr;
  interfaceName.value = adapter.npcapName ?? "";
});

watch(
  () => capabilities.value?.npcapAvailable,
  (available) => {
    if (!available) enhanced.value = false;
  },
);

function sourceLabel(source: string): string {
  return t(`networkTools.arp.sources.${source}`);
}

async function run(): Promise<void> {
  if (busy.value) return;
  if (!adapterName.value) {
    error.value = t("networkTools.arp.noAdapter");
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    result.value = await ipc.toolArpScan({
      cidr: cidr.value.trim(),
      timeoutMs: Math.min(10_000, Math.max(500, Number(timeoutMs.value) || 2500)),
      concurrency: Math.min(64, Math.max(1, Number(concurrency.value) || 32)),
      enhanced: enhanced.value,
      adapterName: adapterName.value,
      interfaceName: enhanced.value ? interfaceName.value || undefined : undefined,
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.arp.title')"
    :description="t('networkTools.tools.arp.description')"
    :icon="Radar"
  >
    <div class="mx-auto max-w-[1040px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.arp.adapter")
            }}</Label>
            <FormSelect v-model="adapterName" :options="adapterOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.arp.cidr")
            }}</Label>
            <Input
              v-model="cidr"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.arp.timeout")
            }}</Label>
            <Input
              v-model="timeoutMs"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.arp.concurrency")
            }}</Label>
            <Input
              v-model="concurrency"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="col-span-2 flex items-end">
            <Button
              size="sm"
              class="h-9 w-full gap-1.5 text-[12px]"
              :disabled="busy || loading"
              @click="run"
            >
              <Radar class="size-3.5" />
              {{ busy ? t("tools.running") : t("networkTools.arp.start") }}
            </Button>
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-4 rounded-lg border border-hairline px-3 py-2">
          <label class="flex items-center gap-2 text-[11px] text-muted-foreground">
            <Switch v-model="enhanced" :disabled="!enhancedAvailable" />
            {{ t("networkTools.arp.enhanced") }}
          </label>
          <span class="min-w-0 flex-1 text-[10.5px] text-muted-foreground">
            {{ enhancedHint || t("networkTools.arp.enhancedHint") }}
          </span>
          <div v-if="enhanced && interfaceOptions.length" class="w-[260px]">
            <FormSelect
              v-model="interfaceName"
              :options="interfaceOptions"
              :placeholder="t('networkTools.arp.interface')"
            />
          </div>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="space-y-3">
        <div
          class="flex flex-wrap items-center gap-2 rounded-xl border border-hairline bg-well-soft px-3 py-2.5 text-[11px]"
        >
          <span class="numeric text-muted-foreground">
            {{ result.cidr }} · {{ result.entries.length }} {{ t("networkTools.arp.hosts") }} ·
            {{ result.elapsedMs }} ms
          </span>
          <span
            v-if="result.conflicts"
            class="rounded border border-error/25 bg-error/10 px-1.5 py-0.5 text-error"
          >
            {{ t("networkTools.arp.conflicts", { count: result.conflicts }) }}
          </span>
          <span
            v-else
            class="rounded border border-signal/25 bg-signal/10 px-1.5 py-0.5 text-signal"
          >
            {{ t("networkTools.arp.noConflicts") }}
          </span>
          <span v-if="result.enhanced" class="text-muted-foreground">
            {{ t("networkTools.arp.enhancedUsed") }}
          </span>
        </div>

        <div class="overflow-x-auto rounded-xl border border-hairline">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">IP</th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">MAC</th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.arp.interfaceLabel") }}
                </th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.arp.source") }}
                </th>
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.arp.statusLabel") }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(entry, index) in result.entries"
                :key="`${entry.ip}-${entry.mac}-${entry.source}-${index}`"
                class="border-b border-hairline last:border-0"
                :class="entry.conflict ? 'bg-error/[0.04]' : ''"
              >
                <td class="numeric px-3 py-1.5 text-foreground">{{ entry.ip }}</td>
                <td class="numeric px-2 py-1.5 text-muted-foreground">{{ entry.mac }}</td>
                <td class="px-2 py-1.5 text-muted-foreground">{{ entry.interface }}</td>
                <td class="px-2 py-1.5 text-muted-foreground">{{ sourceLabel(entry.source) }}</td>
                <td class="px-3 py-1.5">
                  <span
                    class="rounded border px-1 py-0.5 text-[10px]"
                    :class="
                      entry.conflict
                        ? 'border-error/25 bg-error/10 text-error'
                        : 'border-signal/25 bg-signal/10 text-signal'
                    "
                  >
                    {{
                      entry.conflict ? t("networkTools.arp.conflict") : t("networkTools.arp.normal")
                    }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div
        v-else-if="!busy && !loading"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.arp.hint") }}
      </div>
      <div v-else-if="loading" class="px-4 py-10 text-center text-[11px] text-muted-foreground">
        {{ t("tools.loading") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
