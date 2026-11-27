<script setup lang="ts">
import { Boxes, CheckCircle2, CircleAlert, Copy, Check } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { validateCompose } from "@/lib/compose-validate";
import { useToolboxStore } from "@/stores/toolbox";

const SAMPLE = `services:
  web:
    image: nginx:1.27
    ports:
      - "8080:80"
    depends_on:
      - api
  api:
    build: .
    environment:
      DATABASE_URL: postgres://db/app
    depends_on:
      db:
        condition: service_healthy
  db:
    image: postgres:16
`;

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const text = toolbox.toolPref("dev.compose.text", SAMPLE);
const report = computed(() => validateCompose(text.value));

const issueText = computed(() =>
  report.value.issues.map((issue) => `[${issue.level}] ${issue.path}: ${issue.message}`).join("\n"),
);

function issueMessage(message: string): string {
  const key = `devTools.compose.issues.${message.split(":")[0]}`;
  const translated = t(key);
  if (translated === key) return message;
  const detail = message.includes(":") ? message.slice(message.indexOf(":") + 1) : "";
  return detail ? `${translated} (${detail})` : translated;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.compose.title')"
    :description="t('devTools.tools.compose.description')"
    :icon="Boxes"
  >
    <div class="mx-auto max-w-[1080px] space-y-3">
      <div
        class="flex flex-wrap items-center gap-2 rounded-xl border border-hairline bg-well-soft px-3 py-2"
      >
        <span
          class="flex items-center gap-1.5 rounded border px-2 py-0.5 text-[10.5px]"
          :class="
            report.valid
              ? 'border-signal/25 bg-signal/10 text-signal'
              : 'border-error/25 bg-error/10 text-error'
          "
        >
          <CheckCircle2 v-if="report.valid" class="size-3" />
          <CircleAlert v-else class="size-3" />
          {{ report.valid ? t("devTools.compose.valid") : t("devTools.compose.invalid") }}
        </span>
        <span class="text-[11px] text-muted-foreground">
          {{ t("devTools.compose.services", { count: report.services.length }) }}
        </span>
        <Button
          variant="ghost"
          size="sm"
          class="ml-auto h-7 gap-1 text-[10.5px] text-muted-foreground"
          :disabled="!issueText"
          @click="copy('issues', issueText)"
        >
          <Check v-if="isCopied('issues')" class="size-3 text-signal" />
          <Copy v-else class="size-3" />
          {{ isCopied("issues") ? t("common.copiedShort") : t("tools.copy") }}
        </Button>
      </div>

      <div class="grid gap-3 lg:grid-cols-[minmax(0,1fr)_360px]">
        <textarea
          v-model="text"
          class="scrollbar-thin numeric h-[480px] w-full resize-none rounded-xl border border-hairline bg-well px-3 py-2.5 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
        />
        <div class="space-y-2">
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="mb-1.5 text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("devTools.compose.serviceList") }}
            </p>
            <div class="flex flex-wrap gap-1">
              <span
                v-for="service in report.services"
                :key="service"
                class="numeric rounded border border-hairline bg-well px-1.5 py-0.5 text-[10px] text-foreground"
              >
                {{ service }}
              </span>
              <span v-if="!report.services.length" class="text-[11px] text-muted-foreground"
                >—</span
              >
            </div>
          </div>

          <div
            class="scrollbar-thin max-h-[400px] space-y-1.5 overflow-auto rounded-xl border border-hairline p-2"
          >
            <div
              v-for="(issue, index) in report.issues"
              :key="index"
              class="rounded-lg border px-2 py-1.5 text-[10.5px]"
              :class="
                issue.level === 'error'
                  ? 'border-error/25 bg-error/[0.07] text-error'
                  : 'border-warn/25 bg-warn/[0.07] text-warn'
              "
            >
              <p class="numeric text-[9px] opacity-70">{{ issue.path }}</p>
              <p class="mt-0.5">{{ issueMessage(issue.message) }}</p>
            </div>
            <p v-if="!report.issues.length" class="px-2 py-6 text-center text-[11px] text-signal">
              {{ t("devTools.compose.noIssues") }}
            </p>
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
