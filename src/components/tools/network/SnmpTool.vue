<script setup lang="ts">
import { Check, Copy, Network, Play } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type {
  SnmpInput,
  SnmpOperation,
  SnmpResult,
  SnmpValueType,
  SnmpVersion,
} from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();

const host = toolbox.toolPref("network.snmp.host", "192.168.1.1");
const port = toolbox.toolPref("network.snmp.port", "161");
const version = toolbox.toolPref<SnmpVersion>("network.snmp.version", "2c");
const community = toolbox.toolPref("network.snmp.community", "public");
const operation = toolbox.toolPref<SnmpOperation>("network.snmp.operation", "get");
const oidsText = toolbox.toolPref("network.snmp.oids", "1.3.6.1.2.1.1.1.0\n1.3.6.1.2.1.1.3.0");
const valueType = toolbox.toolPref<SnmpValueType>("network.snmp.valueType", "string");
const value = toolbox.toolPref("network.snmp.value", "");
const timeoutMs = toolbox.toolPref("network.snmp.timeout", "3000");
const retries = toolbox.toolPref("network.snmp.retries", "1");
const v3User = toolbox.toolPref("network.snmp.v3.user", "");
const v3Level = toolbox.toolPref("network.snmp.v3.level", "authNoPriv");
const v3Auth = toolbox.toolPref("network.snmp.v3.auth", "sha1");
const v3Password = ref("");
const v3Context = toolbox.toolPref("network.snmp.v3.context", "");

const busy = ref(false);
const error = ref<string | null>(null);
const result = ref<SnmpResult | null>(null);

const versionOptions = [
  { value: "1" as const, label: "v1" },
  { value: "2c" as const, label: "v2c" },
  { value: "3" as const, label: "v3" },
];
const operationOptions = computed(() => [
  { value: "get" as const, label: "GET" },
  { value: "walk" as const, label: "WALK" },
  { value: "set" as const, label: "SET" },
]);
const valueTypeOptions: Array<{ value: SnmpValueType; label: string }> = [
  { value: "string", label: "OCTET STRING" },
  { value: "integer", label: "INTEGER" },
  { value: "oid", label: "OID" },
  { value: "ipaddress", label: "IpAddress" },
  { value: "counter32", label: "Counter32" },
  { value: "gauge32", label: "Gauge32" },
  { value: "timeticks", label: "TimeTicks" },
  { value: "counter64", label: "Counter64" },
  { value: "hex", label: "Hex" },
];
const levelOptions = [
  { value: "noAuthNoPriv" as const, label: "noAuthNoPriv" },
  { value: "authNoPriv" as const, label: "authNoPriv" },
];
const authOptions = [
  { value: "md5" as const, label: "MD5" },
  { value: "sha1" as const, label: "SHA-1" },
  { value: "sha256" as const, label: "SHA-256" },
];

async function query(): Promise<void> {
  if (busy.value) return;
  const oids = oidsText.value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (!host.value.trim()) {
    error.value = t("networkTools.snmp.hostRequired");
    return;
  }
  if (!oids.length) {
    error.value = t("networkTools.snmp.oidsRequired");
    return;
  }
  busy.value = true;
  error.value = null;
  result.value = null;
  try {
    const input: SnmpInput = {
      host: host.value.trim(),
      port: Math.min(65535, Math.max(1, Number(port.value) || 161)),
      version: version.value,
      community: community.value,
      operation: operation.value,
      oids,
      valueType: valueType.value,
      value: operation.value === "set" ? value.value : undefined,
      timeoutMs: Math.min(20_000, Math.max(500, Number(timeoutMs.value) || 3000)),
      retries: Math.min(5, Math.max(0, Number(retries.value) || 0)),
      v3:
        version.value === "3"
          ? {
              username: v3User.value.trim(),
              securityLevel: v3Level.value as "noAuthNoPriv" | "authNoPriv",
              authProtocol: v3Auth.value as "md5" | "sha1" | "sha256",
              authPassword: v3Level.value === "authNoPriv" ? v3Password.value : undefined,
              contextName: v3Context.value.trim() || undefined,
            }
          : undefined,
    };
    result.value = await ipc.toolSnmpQuery(input);
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.snmp.title')"
    :description="t('networkTools.tools.snmp.description')"
    :icon="Network"
  >
    <div class="mx-auto max-w-[1080px] space-y-4">
      <div class="grid gap-2 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-12">
        <div class="space-y-1.5 lg:col-span-4">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.snmp.host") }}</Label>
          <Input
            v-model="host"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.snmp.port") }}</Label>
          <Input
            v-model="port"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.snmp.version")
          }}</Label>
          <FormSelect v-model="version" :options="versionOptions" />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.snmp.operation")
          }}</Label>
          <FormSelect v-model="operation" :options="operationOptions" />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.snmp.timeout")
          }}</Label>
          <Input
            v-model="timeoutMs"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>

        <div v-if="version !== '3'" class="space-y-1.5 lg:col-span-4">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.snmp.community")
          }}</Label>
          <Input
            v-model="community"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <template v-else>
          <div class="space-y-1.5 lg:col-span-3">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.snmp.v3User")
            }}</Label>
            <Input
              v-model="v3User"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5 lg:col-span-3">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.snmp.securityLevel")
            }}</Label>
            <FormSelect v-model="v3Level" :options="levelOptions" />
          </div>
          <div class="space-y-1.5 lg:col-span-2">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.snmp.authProtocol")
            }}</Label>
            <FormSelect v-model="v3Auth" :options="authOptions" />
          </div>
          <div class="space-y-1.5 lg:col-span-4">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.snmp.authPassword")
            }}</Label>
            <Input
              v-model="v3Password"
              type="password"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              autocomplete="off"
            />
          </div>
          <div class="space-y-1.5 lg:col-span-4">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.snmp.context")
            }}</Label>
            <Input
              v-model="v3Context"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
        </template>

        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.snmp.retries")
          }}</Label>
          <Input
            v-model="retries"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>

        <div class="space-y-1.5 lg:col-span-7">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.snmp.oids") }}</Label>
          <textarea
            v-model="oidsText"
            class="scrollbar-thin numeric h-24 w-full resize-y rounded-lg border border-hairline-strong bg-well px-2 py-1.5 text-[11.5px] outline-none focus:border-signal/50"
            :placeholder="t('networkTools.snmp.oidsPlaceholder')"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-3">
          <template v-if="operation === 'set'">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.snmp.valueType")
            }}</Label>
            <FormSelect v-model="valueType" :options="valueTypeOptions" />
            <Label class="mt-2 block text-[11px] text-muted-foreground">{{
              t("networkTools.snmp.value")
            }}</Label>
            <Input
              v-model="value"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </template>
          <p v-else class="text-[10.5px] leading-relaxed text-muted-foreground">
            {{ t("networkTools.snmp.help") }}
          </p>
        </div>
        <div class="flex items-start lg:col-span-2">
          <Button size="sm" class="h-9 w-full gap-1.5 text-[12px]" :disabled="busy" @click="query">
            <Play class="size-3.5" />
            {{ busy ? t("tools.running") : t("networkTools.snmp.query") }}
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
        <div class="flex items-center gap-2 border-b border-hairline px-3 py-2 text-[11px]">
          <span class="text-muted-foreground">
            {{ result.operation.toUpperCase() }} · SNMP {{ result.version }} ·
            {{ result.variables.length }} {{ t("networkTools.snmp.varbinds") }}
          </span>
          <span class="numeric ml-auto text-muted-foreground">{{ result.elapsedMs }} ms</span>
        </div>
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline text-left text-muted-foreground">
                <th class="px-3 py-2 font-medium">OID</th>
                <th class="px-2 py-2 font-medium">{{ t("networkTools.snmp.type") }}</th>
                <th class="px-2 py-2 font-medium">{{ t("networkTools.snmp.value") }}</th>
                <th class="px-3 py-2"></th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(variable, index) in result.variables"
                :key="`${variable.oid}-${index}`"
                class="border-b border-hairline last:border-0"
              >
                <td class="numeric px-3 py-1.5 text-foreground">{{ variable.oid }}</td>
                <td class="px-2 py-1.5 text-muted-foreground">{{ variable.valueType }}</td>
                <td class="numeric break-all px-2 py-1.5 text-muted-foreground">
                  {{ variable.value }}
                </td>
                <td class="px-3 py-1.5 text-right">
                  <button
                    type="button"
                    class="inline-flex items-center gap-1 text-muted-foreground hover:text-foreground"
                    @click="copy(`var:${index}`, `${variable.oid} = ${variable.value}`)"
                  >
                    <Check v-if="isCopied(`var:${index}`)" class="size-3 text-signal" />
                    <Copy v-else class="size-3" />
                    <span v-if="isCopied(`var:${index}`)" class="text-[9px] text-signal">{{
                      t("common.copiedShort")
                    }}</span>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
