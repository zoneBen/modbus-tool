<script setup lang="ts">
import { Check, Copy, Lock, Search, ShieldCheck, ShieldAlert } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { SslCertResult } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const host = toolbox.toolPref("network.ssl.host", "example.com");
const port = toolbox.toolPref("network.ssl.port", "443");
const serverName = toolbox.toolPref("network.ssl.serverName", "");
const busy = ref(false);
const error = ref<string | null>(null);
const result = ref<SslCertResult | null>(null);

const expiryTone = computed(() => (info: { expired: boolean; daysRemaining: number }) => {
  if (info.expired) return "border-error/25 bg-error/10 text-error";
  if (info.daysRemaining <= 30) return "border-warn/25 bg-warn/10 text-warn";
  return "border-signal/25 bg-signal/10 text-signal";
});

async function inspect(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  result.value = null;
  try {
    result.value = await ipc.toolSslCert({
      host: host.value.trim(),
      port: Math.min(65535, Math.max(1, Number(port.value) || 443)),
      serverName: serverName.value.trim() || undefined,
      timeoutMs: 10_000,
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  } finally {
    busy.value = false;
  }
}

function shortName(value: string): string {
  const common = value.match(/CN=([^,]+)/);
  return common ? common[1] : value;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.ssl.title')"
    :description="t('networkTools.tools.ssl.description')"
    :icon="Lock"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="min-w-[220px] flex-1 space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.ssl.host") }}</Label>
          <Input
            v-model="host"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            placeholder="example.com"
            @keydown.enter="inspect"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.ssl.port") }}</Label>
          <Input
            v-model="port"
            class="numeric h-9 w-24 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="min-w-[180px] flex-1 space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.ssl.serverName")
          }}</Label>
          <Input
            v-model="serverName"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            :placeholder="t('networkTools.ssl.serverNamePlaceholder')"
          />
        </div>
        <Button size="sm" class="h-9 gap-1.5 text-[12px]" :disabled="busy" @click="inspect">
          <Search class="size-3.5" />
          {{ busy ? t("tools.running") : t("networkTools.ssl.inspect") }}
        </Button>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="space-y-3">
        <div
          class="flex flex-wrap items-center gap-2 rounded-xl border border-hairline bg-well-soft px-3 py-2"
        >
          <Lock class="size-3.5 text-signal" />
          <span class="numeric text-[11.5px]"> {{ result.serverName }}:{{ result.port }} </span>
          <span
            class="numeric rounded border border-hairline px-1.5 py-0.5 text-[10px] text-muted-foreground"
          >
            {{ result.tlsVersion || "—" }}
          </span>
          <span class="numeric text-[10px] text-muted-foreground">{{ result.cipherSuite }}</span>
          <span class="numeric ml-auto text-[10px] text-muted-foreground">
            {{ result.elapsedMs }} ms
          </span>
        </div>

        <div
          v-for="(certificate, index) in result.chain"
          :key="certificate.serial"
          class="rounded-xl border border-hairline bg-well-soft p-3"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span
              class="flex size-7 items-center justify-center rounded-lg border"
              :class="
                index === 0
                  ? 'border-signal/25 bg-signal/10 text-signal'
                  : 'border-hairline bg-well text-muted-foreground'
              "
            >
              <ShieldCheck v-if="!certificate.expired" class="size-3.5" />
              <ShieldAlert v-else class="size-3.5" />
            </span>
            <div class="min-w-0 flex-1">
              <p class="truncate text-[12px] font-medium">
                {{ index === 0 ? t("networkTools.ssl.leaf") : t("networkTools.ssl.intermediate") }}
                · {{ shortName(certificate.subject) }}
              </p>
              <p class="truncate text-[10px] text-muted-foreground">
                {{ certificate.subject }}
              </p>
            </div>
            <span class="rounded border px-1.5 py-0.5 text-[10px]" :class="expiryTone(certificate)">
              {{
                certificate.expired
                  ? t("networkTools.ssl.expired")
                  : t("networkTools.ssl.daysLeft", { days: certificate.daysRemaining })
              }}
            </span>
            <button
              type="button"
              class="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground"
              @click="copy(`cert:${index}`, certificate.subject)"
            >
              <Check v-if="isCopied(`cert:${index}`)" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              <span v-if="isCopied(`cert:${index}`)" class="text-signal">{{
                t("common.copiedShort")
              }}</span>
            </button>
          </div>

          <div class="mt-2 grid gap-1.5 text-[10.5px] sm:grid-cols-2">
            <p class="text-muted-foreground">
              {{ t("networkTools.ssl.issuer") }}:
              <span class="text-foreground">{{ certificate.issuer }}</span>
            </p>
            <p class="numeric text-muted-foreground">
              {{ t("networkTools.ssl.serial") }}:
              <span class="text-foreground">{{ certificate.serial }}</span>
            </p>
            <p class="numeric text-muted-foreground">
              {{ t("networkTools.ssl.validFrom") }}:
              <span class="text-foreground">{{ certificate.notBefore }}</span>
            </p>
            <p class="numeric text-muted-foreground">
              {{ t("networkTools.ssl.validTo") }}:
              <span class="text-foreground">{{ certificate.notAfter }}</span>
            </p>
            <p class="text-muted-foreground">
              {{ t("networkTools.ssl.signature") }}:
              <span class="numeric text-foreground">{{ certificate.signatureAlgorithm }}</span>
            </p>
            <p class="text-muted-foreground">
              {{ t("networkTools.ssl.publicKey") }}:
              <span class="numeric text-foreground">{{ certificate.publicKeyAlgorithm }}</span>
            </p>
          </div>

          <div v-if="certificate.san.length" class="mt-2">
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">SAN</p>
            <div class="mt-1 flex flex-wrap gap-1">
              <span
                v-for="name in certificate.san"
                :key="name"
                class="numeric rounded border border-hairline bg-well px-1.5 py-0.5 text-[9.5px] text-muted-foreground"
              >
                {{ name }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else-if="!busy && !error"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.ssl.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
