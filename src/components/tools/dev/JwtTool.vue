<script setup lang="ts">
import { Check, Copy, KeyRound, ShieldCheck, ShieldX } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { checkJwtClaims, decodeJwt, verifyJwt } from "@/lib/jwt";
import type { JwtParts, JwtVerifyResult } from "@/lib/jwt";
import { useToolboxStore } from "@/stores/toolbox";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const token = toolbox.toolPref("dev.jwt.token", "");
const key = toolbox.toolPref("dev.jwt.key", "");
const busy = ref(false);
const verifyResult = ref<JwtVerifyResult | null>(null);

const decoded = computed<{ parts?: JwtParts; error?: string }>(() => {
  if (!token.value.trim()) return {};
  try {
    return { parts: decodeJwt(token.value) };
  } catch (error: unknown) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
});

const claims = computed(() =>
  decoded.value.parts ? checkJwtClaims(decoded.value.parts.payload) : null,
);

const headerText = computed(() =>
  decoded.value.parts ? JSON.stringify(decoded.value.parts.header, null, 2) : "",
);
const payloadText = computed(() =>
  decoded.value.parts ? JSON.stringify(decoded.value.parts.payload, null, 2) : "",
);

async function verify(): Promise<void> {
  if (busy.value || !token.value.trim()) return;
  busy.value = true;
  try {
    verifyResult.value = await verifyJwt(token.value.trim(), key.value);
  } catch (error: unknown) {
    verifyResult.value = {
      valid: false,
      algorithm: String(decoded.value.parts?.header.alg ?? ""),
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.jwt.title')"
    :description="t('devTools.tools.jwt.description')"
    :icon="KeyRound"
  >
    <div class="mx-auto max-w-[1080px] space-y-4">
      <div class="space-y-2 rounded-xl border border-hairline bg-well-soft p-3">
        <Label class="text-[11px] text-muted-foreground">{{ t("devTools.jwt.token") }}</Label>
        <textarea
          v-model="token"
          class="scrollbar-thin numeric h-24 w-full resize-y rounded-lg border border-hairline-strong bg-well px-2.5 py-2 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
          placeholder="eyJhbGciOiJIUzI1NiIs..."
          @input="verifyResult = null"
        />
        <div class="flex flex-wrap items-end gap-2">
          <div class="min-w-[260px] flex-1 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("devTools.jwt.key") }}</Label>
            <Input
              v-model="key"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              :placeholder="t('devTools.jwt.keyPlaceholder')"
            />
          </div>
          <Button
            size="sm"
            class="h-9 gap-1.5 text-[11px]"
            :disabled="busy || !token.trim()"
            @click="verify"
          >
            <ShieldCheck class="size-3.5" />
            {{ busy ? t("tools.running") : t("devTools.jwt.verify") }}
          </Button>
        </div>
        <p class="text-[10px] leading-relaxed text-muted-foreground">
          {{ t("devTools.jwt.keyNote") }}
        </p>
      </div>

      <p
        v-if="decoded.error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ decoded.error === "jwtParts" ? t("devTools.jwt.invalid") : decoded.error }}
      </p>

      <div
        v-if="verifyResult"
        class="rounded-xl border p-3 text-[11.5px]"
        :class="
          verifyResult.valid
            ? 'border-signal/25 bg-signal/[0.06] text-signal'
            : 'border-error/25 bg-error/10 text-error'
        "
      >
        <div class="flex items-center gap-2">
          <ShieldCheck v-if="verifyResult.valid" class="size-4" />
          <ShieldX v-else class="size-4" />
          <span class="font-medium">
            {{ t(`devTools.jwt.${verifyResult.message}`) }}
          </span>
          <span class="numeric ml-auto text-[10px] opacity-80">{{ verifyResult.algorithm }}</span>
        </div>
      </div>

      <div v-if="decoded.parts" class="grid gap-3 lg:grid-cols-2">
        <div class="rounded-xl border border-hairline">
          <div class="flex items-center gap-2 border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">Header</span>
            <span
              class="numeric rounded border border-hairline px-1.5 py-0.5 text-[9px] text-muted-foreground"
            >
              {{ decoded.parts.header.alg }}
            </span>
            <Button
              variant="ghost"
              size="sm"
              class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
              @click="copy('header', headerText)"
            >
              <Check v-if="isCopied('header')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("header") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
          <pre
            class="scrollbar-thin numeric max-h-[220px] overflow-auto whitespace-pre-wrap break-all px-3 py-2 text-[11px] leading-relaxed"
            >{{ headerText }}</pre>
        </div>

        <div class="rounded-xl border border-hairline">
          <div class="flex items-center gap-2 border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">Payload</span>
            <Button
              variant="ghost"
              size="sm"
              class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
              @click="copy('payload', payloadText)"
            >
              <Check v-if="isCopied('payload')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("payload") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
          <pre
            class="scrollbar-thin numeric max-h-[220px] overflow-auto whitespace-pre-wrap break-all px-3 py-2 text-[11px] leading-relaxed"
            >{{ payloadText }}</pre>
        </div>

        <div v-if="claims" class="lg:col-span-2 flex flex-wrap gap-2">
          <span
            class="rounded-lg border px-2 py-1 text-[10.5px]"
            :class="
              claims.expired
                ? 'border-error/25 bg-error/10 text-error'
                : 'border-signal/25 bg-signal/10 text-signal'
            "
          >
            exp: {{ claims.expiresAt ? claims.expiresAt.toLocaleString() : "—" }} ({{
              claims.expired ? t("devTools.jwt.expired") : t("devTools.jwt.notExpired")
            }})
          </span>
          <span
            v-if="claims.notBefore"
            class="rounded-lg border border-warn/25 bg-warn/10 px-2 py-1 text-[10.5px] text-warn"
          >
            nbf: {{ t("devTools.jwt.notYetValid") }}
          </span>
          <span
            v-if="claims.issuedAt"
            class="rounded-lg border border-hairline bg-well px-2 py-1 text-[10.5px] text-muted-foreground"
          >
            iat: {{ claims.issuedAt.toLocaleString() }}
          </span>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
