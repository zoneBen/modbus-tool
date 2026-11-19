<script setup lang="ts">
import { Copy, Search } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ipc } from "@/lib/ipc";
import { useToolboxStore } from "@/stores/toolbox";
import { toolErrorText } from "@/lib/tool-error";
import type { DnsRecordType, DnsResult } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const name = toolbox.toolPref("network.dns.name", "example.com");
const recordType = toolbox.toolPref<DnsRecordType>("network.dns.recordType", "A");
const server = toolbox.toolPref("network.dns.server", "");
const timeoutMs = toolbox.toolPref("network.dns.timeout", "5000");
const busy = ref(false);
const result = ref<DnsResult | null>(null);
const error = ref<string | null>(null);

const recordTypes = ["A", "AAAA", "CNAME", "MX", "TXT", "NS", "SOA", "SRV", "PTR", "CAA"].map(
  (value) => ({ label: value, value: value as DnsRecordType }),
);

const statusTone = computed(() => {
  if (!result.value) return "";
  return result.value.status === "ok"
    ? "border-signal/25 bg-signal/10 text-signal"
    : "border-warn/25 bg-warn/10 text-warn";
});

async function query(): Promise<void> {
  if (busy.value) return;
  const queryName = name.value.trim();
  if (!queryName) {
    error.value = t("networkTools.dns.nameRequired");
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    result.value = await ipc.toolDnsQuery({
      name: queryName,
      recordType: recordType.value,
      server: server.value.trim() || undefined,
      timeoutMs: Math.min(30_000, Math.max(500, Number(timeoutMs.value) || 5000)),
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}

async function copyAnswers(): Promise<void> {
  if (!result.value?.answers.length) return;
  const text = result.value.answers
    .map((answer) => `${answer.name}\t${answer.ttl}\t${answer.recordType}\t${answer.value}`)
    .join("\n");
  try {
    await navigator.clipboard.writeText(text);
    toast.success(t("common.copied"));
  } catch {
    toast.error(t("tools.copyFailed"));
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.dns.title')"
    :description="t('networkTools.tools.dns.description')"
    :icon="Search"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div
        class="grid grid-cols-2 gap-3 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-4"
      >
        <div class="col-span-2 space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.dns.name") }}</Label>
          <Input
            v-model="name"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            placeholder="example.com / 8.8.8.8"
            @keydown.enter="query"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.dns.type") }}</Label>
          <FormSelect v-model="recordType" :options="recordTypes" />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.dns.timeout")
          }}</Label>
          <Input
            v-model="timeoutMs"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="col-span-3 space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.dns.server")
          }}</Label>
          <Input
            v-model="server"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            :placeholder="t('networkTools.dns.serverPlaceholder')"
            @keydown.enter="query"
          />
        </div>
        <div class="flex items-end">
          <Button size="sm" class="h-9 w-full gap-1.5 text-[12px]" :disabled="busy" @click="query">
            <Search class="size-3.5" />
            {{ busy ? t("tools.running") : t("networkTools.dns.query") }}
          </Button>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="rounded-xl border border-hairline">
        <div class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2.5">
          <span class="rounded border px-1.5 py-0.5 text-[10px]" :class="statusTone">
            {{ t(`networkTools.dns.statuses.${result.status}`) }}
          </span>
          <span class="numeric text-[11px] text-muted-foreground">
            {{ result.recordType }} · {{ result.server }} · {{ result.elapsedMs }} ms
          </span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-7 gap-1 text-[11px] text-muted-foreground"
            :disabled="!result.answers.length"
            @click="copyAnswers"
          >
            <Copy class="size-3" />
            {{ t("tools.copy") }}
          </Button>
        </div>
        <div v-if="result.answers.length" class="overflow-x-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.dns.name") }}
                </th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">TTL</th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.dns.type") }}
                </th>
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.dns.value") }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(answer, index) in result.answers"
                :key="`${answer.name}-${answer.recordType}-${index}`"
                class="border-b border-hairline last:border-0"
              >
                <td class="px-3 py-1.5 text-foreground">{{ answer.name }}</td>
                <td class="numeric px-2 py-1.5 text-muted-foreground">{{ answer.ttl }}</td>
                <td class="px-2 py-1.5 text-muted-foreground">{{ answer.recordType }}</td>
                <td class="numeric break-all px-3 py-1.5 text-muted-foreground">
                  {{ answer.value }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p v-else class="px-3 py-6 text-center text-[11px] text-muted-foreground">
          {{ t("networkTools.dns.noAnswers") }}
        </p>
      </div>

      <div
        v-else-if="!busy"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.dns.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
