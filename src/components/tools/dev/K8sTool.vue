<script setup lang="ts">
import { Check, CheckCircle2, CircleAlert, Copy, Layers } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { validateK8s } from "@/lib/k8s-validate";
import { useToolboxStore } from "@/stores/toolbox";

const SAMPLE = `apiVersion: apps/v1
kind: Deployment
metadata:
  name: demo
  namespace: default
spec:
  replicas: 2
  selector:
    matchLabels:
      app: demo
  template:
    metadata:
      labels:
        app: demo
    spec:
      containers:
        - name: demo
          image: nginx:1.27
---
apiVersion: v1
kind: Service
metadata:
  name: demo
spec:
  selector:
    app: demo
  ports:
    - port: 80
`;

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const text = toolbox.toolPref("dev.k8s.text", SAMPLE);
const report = computed(() => validateK8s(text.value));

const issueText = computed(() =>
  report.value.issues
    .map((issue) => `[${issue.level}] #${issue.document} ${issue.path}: ${issue.message}`)
    .join("\n"),
);

function issueMessage(message: string): string {
  const key = `devTools.k8s.issues.${message}`;
  const translated = t(key);
  return translated === key ? message : translated;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.k8s.title')"
    :description="t('devTools.tools.k8s.description')"
    :icon="Layers"
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
          {{ report.valid ? t("devTools.k8s.valid") : t("devTools.k8s.invalid") }}
        </span>
        <span class="text-[11px] text-muted-foreground">
          {{ t("devTools.k8s.resources", { count: report.resources.length }) }}
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

      <div class="grid gap-3 lg:grid-cols-[minmax(0,1fr)_380px]">
        <textarea
          v-model="text"
          class="scrollbar-thin numeric h-[500px] w-full resize-none rounded-xl border border-hairline bg-well px-3 py-2.5 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
        />
        <div class="space-y-2">
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="mb-1.5 text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("devTools.k8s.resourceList") }}
            </p>
            <div class="space-y-1">
              <div
                v-for="resource in report.resources"
                :key="`${resource.document}-${resource.kind}-${resource.name}`"
                class="flex items-center gap-2 rounded-md bg-overlay-1 px-2 py-1"
              >
                <span class="numeric w-6 text-[9px] text-muted-foreground"
                  >#{{ resource.document }}</span
                >
                <span
                  class="rounded border border-hairline px-1 py-0.5 text-[9px] text-muted-foreground"
                  >{{ resource.kind }}</span
                >
                <span class="min-w-0 flex-1 truncate text-[10.5px]">{{ resource.name }}</span>
                <span class="numeric text-[9px] text-muted-foreground">{{
                  resource.apiVersion
                }}</span>
              </div>
              <p v-if="!report.resources.length" class="text-[11px] text-muted-foreground">—</p>
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
              <p class="numeric text-[9px] opacity-70">#{{ issue.document }} · {{ issue.path }}</p>
              <p class="mt-0.5">{{ issueMessage(issue.message) }}</p>
            </div>
            <p v-if="!report.issues.length" class="px-2 py-6 text-center text-[11px] text-signal">
              {{ t("devTools.k8s.noIssues") }}
            </p>
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
