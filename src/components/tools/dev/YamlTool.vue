<script setup lang="ts">
import { AlertTriangle, CheckCircle2, FileCode2, FolderOpen, Play, ShieldCheck } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { ipc } from "@/lib/ipc";
import { validateYaml } from "@/lib/yaml-validate";
import type { YamlValidationResult } from "@/lib/yaml-validate";

const { t } = useI18n();
const MAX_INPUT_BYTES = 2 * 1024 * 1024;
const input = ref("");
const filePath = ref("");
const result = ref<YamlValidationResult | null>(null);
const error = ref<string | null>(null);
const inputBytes = computed(() => new TextEncoder().encode(input.value).length);
const lineCount = computed(() => (input.value ? input.value.split("\n").length : 0));
const issues = computed(() =>
  result.value ? [...result.value.errors, ...result.value.warnings] : [],
);

function validate(): void {
  error.value = null;
  result.value = null;
  if (!input.value.trim()) {
    error.value = t("devTools.yaml.empty");
    return;
  }
  if (inputBytes.value > MAX_INPUT_BYTES) {
    error.value = t("devTools.yaml.tooLarge", { max: 2 });
    return;
  }
  result.value = validateYaml(input.value);
}

async function browse(): Promise<void> {
  const path = await ipc.pickFile([
    { name: t("devTools.yaml.files"), extensions: ["yml", "yaml"] },
  ]);
  if (!path) return;
  try {
    const text = await ipc.readTextFile(path);
    if (new TextEncoder().encode(text).length > MAX_INPUT_BYTES) {
      error.value = t("devTools.yaml.tooLarge", { max: 2 });
      return;
    }
    filePath.value = path;
    input.value = text;
    validate();
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
}

function clear(): void {
  input.value = "";
  filePath.value = "";
  result.value = null;
  error.value = null;
}

function issueMessage(message: string): string {
  return message === "empty" ? t("devTools.yaml.empty") : message;
}
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.yaml.title')"
    :description="t('devTools.tools.yaml.description')"
    :icon="FileCode2"
  >
    <div class="mx-auto flex h-full max-w-[1240px] min-h-[520px] flex-col gap-3">
      <div
        class="flex flex-wrap items-center gap-3 rounded-xl border border-hairline bg-well-soft px-3 py-2"
      >
        <div class="flex min-w-0 items-center gap-2">
          <Label class="shrink-0 text-[11px] text-muted-foreground">{{
            t("devTools.yaml.file")
          }}</Label>
          <span class="numeric min-w-0 max-w-[420px] truncate text-[11px] text-muted-foreground">
            {{ filePath || t("devTools.yaml.pasteHint") }}
          </span>
        </div>
        <span class="numeric text-[10.5px] text-muted-foreground">
          {{ lineCount }} {{ t("devTools.yaml.lines") }} · {{ inputBytes }} B / 2 MB
        </span>
        <div class="ml-auto flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            class="h-8 gap-1 border-hairline-strong bg-transparent text-[11px]"
            @click="browse"
          >
            <FolderOpen class="size-3" />
            {{ t("devTools.yaml.browse") }}
          </Button>
          <Button size="sm" class="h-8 gap-1.5 text-[12px]" @click="validate">
            <ShieldCheck class="size-3.5" />
            {{ t("devTools.yaml.validate") }}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1 text-[11px] text-muted-foreground"
            :disabled="!input"
            @click="clear"
          >
            {{ t("common.clear") }}
          </Button>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div class="grid min-h-[420px] flex-1 gap-3 lg:grid-cols-[minmax(0,1fr)_420px]">
        <div class="flex min-h-[320px] flex-col rounded-xl border border-hairline bg-well">
          <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">{{ t("devTools.yaml.input") }}</span>
            <span class="text-[10px] text-muted-foreground">{{
              t("devTools.yaml.inputHint")
            }}</span>
          </div>
          <textarea
            v-model="input"
            spellcheck="false"
            class="numeric scrollbar-thin min-h-0 flex-1 resize-none bg-transparent px-3 py-2.5 text-[11.5px] leading-relaxed text-foreground outline-none"
            :placeholder="t('devTools.yaml.placeholder')"
            @input="result = null"
          />
        </div>

        <div class="flex min-h-[320px] flex-col rounded-xl border border-hairline bg-well">
          <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
            <span class="text-[11px] font-medium">{{ t("devTools.yaml.result") }}</span>
            <span v-if="result" class="numeric text-[10.5px] text-muted-foreground">
              {{ result.documents }} {{ t("devTools.yaml.documents") }} · {{ result.errors.length }}
              {{ t("devTools.yaml.errors") }}
            </span>
          </div>

          <div
            v-if="!result"
            class="flex flex-1 flex-col items-center justify-center gap-2 px-6 py-10 text-center"
          >
            <Play class="size-5 text-muted-foreground/60" />
            <p class="text-[11px] text-muted-foreground">{{ t("devTools.yaml.hint") }}</p>
          </div>

          <div v-else class="flex min-h-0 flex-1 flex-col">
            <div
              class="flex items-center gap-2 border-b border-hairline px-3 py-2.5 text-[11.5px]"
              :class="result.valid ? 'text-signal' : 'text-error'"
            >
              <CheckCircle2 v-if="result.valid" class="size-4 shrink-0" />
              <AlertTriangle v-else class="size-4 shrink-0" />
              <span class="font-medium">
                {{ result.valid ? t("devTools.yaml.valid") : t("devTools.yaml.invalid") }}
              </span>
              <span v-if="result.valid" class="text-muted-foreground">
                · {{ t("devTools.yaml.validHint", { count: result.documents }) }}
              </span>
            </div>

            <div class="scrollbar-thin min-h-0 flex-1 overflow-auto px-3 py-2">
              <p v-if="!issues.length" class="py-6 text-center text-[11px] text-muted-foreground">
                {{ t("devTools.yaml.noIssues") }}
              </p>
              <div
                v-for="(issue, index) in issues"
                :key="`${issue.line}-${issue.column}-${index}`"
                class="mb-2 rounded-lg border px-2.5 py-2"
                :class="
                  issue.severity === 'error'
                    ? 'border-error/20 bg-error/[0.06]'
                    : 'border-warn/20 bg-warn/[0.06]'
                "
              >
                <div class="flex items-center gap-2">
                  <span
                    class="numeric shrink-0 rounded border px-1.5 py-0.5 text-[10px]"
                    :class="
                      issue.severity === 'error'
                        ? 'border-error/25 text-error'
                        : 'border-warn/25 text-warn'
                    "
                  >
                    L{{ issue.line }}:{{ issue.column }}
                  </span>
                  <span class="text-[10px] uppercase tracking-wide text-muted-foreground">
                    {{ t(`devTools.yaml.severities.${issue.severity}`) }}
                  </span>
                </div>
                <p class="mt-1 break-words text-[11px] leading-relaxed text-foreground">
                  {{ issueMessage(issue.message) }}
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
