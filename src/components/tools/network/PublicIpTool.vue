<script setup lang="ts">
import { Check, Copy, Globe2, MapPin, RefreshCw } from "@lucide/vue";
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import type { PublicIpInfo } from "@/types/tools";

const { t } = useI18n();
const { copy, isCopied } = useCopyFeedback();
const busy = ref(false);
const error = ref<string | null>(null);
const result = ref<PublicIpInfo | null>(null);

function locationText(info: PublicIpInfo): string {
  return [info.city, info.region, info.country].filter(Boolean).join(" · ") || "—";
}

async function lookup(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    result.value = await ipc.toolPublicIp();
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}

const fields = [
  { key: "asn", labelKey: "networkTools.publicip.asn" },
  { key: "org", labelKey: "networkTools.publicip.org" },
  { key: "timezone", labelKey: "networkTools.publicip.timezone" },
  { key: "provider", labelKey: "networkTools.publicip.provider" },
] as const;

onMounted(lookup);
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.publicip.title')"
    :description="t('networkTools.tools.publicip.description')"
    :icon="Globe2"
  >
    <div class="mx-auto max-w-[920px] space-y-4">
      <div
        class="flex flex-wrap items-center gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <span class="text-[11px] text-muted-foreground">
          {{ t("networkTools.publicip.hint") }}
        </span>
        <Button size="sm" class="ml-auto h-8 gap-1.5 text-[11px]" :disabled="busy" @click="lookup">
          <RefreshCw class="size-3.5" :class="{ 'animate-spin': busy }" />
          {{ busy ? t("networkTools.publicip.querying") : t("networkTools.publicip.refresh") }}
        </Button>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="space-y-3">
        <div class="rounded-xl border border-rx/25 bg-rx/[0.06] p-4">
          <div class="flex items-start gap-3">
            <span
              class="flex size-10 shrink-0 items-center justify-center rounded-xl border border-rx/30 bg-rx/10 text-rx"
            >
              <Globe2 class="size-5" />
            </span>
            <div class="min-w-0 flex-1">
              <p class="text-[11px] text-muted-foreground">
                {{ t("networkTools.publicip.ip") }}
              </p>
              <p class="numeric mt-0.5 break-all text-[24px] font-semibold tracking-tight">
                {{ result.ip }}
              </p>
              <p class="mt-1 flex items-center gap-1 text-[11.5px] text-muted-foreground">
                <MapPin class="size-3" />
                {{ locationText(result) }}
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              class="h-8 shrink-0 gap-1 border-hairline-strong bg-transparent text-[11px]"
              @click="copy('ip', result.ip)"
            >
              <Check v-if="isCopied('ip')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("ip") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
        </div>

        <div class="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
          <div
            v-for="field in fields"
            :key="field.key"
            class="rounded-xl border border-hairline bg-well-soft p-3"
          >
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t(field.labelKey) }}
            </p>
            <p class="numeric mt-1 break-all text-[12px]">
              {{ result[field.key] || "—" }}
            </p>
          </div>
        </div>

        <div class="grid gap-2 sm:grid-cols-2">
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("networkTools.publicip.coordinates") }}
            </p>
            <p class="numeric mt-1 text-[12px]">
              {{ result.latitude ?? "—" }}, {{ result.longitude ?? "—" }}
            </p>
          </div>
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("networkTools.publicip.isp") }}
            </p>
            <p class="mt-1 text-[12px]">{{ result.isp || result.org || "—" }}</p>
          </div>
        </div>
      </div>

      <div
        v-else-if="!busy && !error"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.publicip.empty") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
