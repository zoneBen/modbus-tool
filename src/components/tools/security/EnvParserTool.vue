<script setup lang="ts">
import { Check, Copy, FileCode2 } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { parseEnv, resolvedEnvText } from "@/lib/env-parse";
import { useToolboxStore } from "@/stores/toolbox";

const SAMPLE = `# .env.example
APP_ENV=production
API_BASE=https://api.example.com
DB_HOST=127.0.0.1
DB_PORT=5432
DATABASE_URL=postgres://\${DB_HOST}:\${DB_PORT}/app
FEATURE_FLAG="on"
`;

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const text = toolbox.toolPref("security.env.text", SAMPLE);
const parsed = computed(() => parseEnv(text.value));
const resolved = computed(() => resolvedEnvText(parsed.value));
</script>

<template>
  <ToolPanelFrame
    :title="t('securityTools.tools.env.title')"
    :description="t('securityTools.tools.env.description')"
    :icon="FileCode2"
  >
    <div class="mx-auto max-w-[1080px] space-y-4">
      <div class="grid gap-3 lg:grid-cols-2">
        <div class="space-y-1.5">
          <p class="text-[11px] font-medium">{{ t("securityTools.env.source") }}</p>
          <textarea
            v-model="text"
            class="scrollbar-thin numeric h-[360px] w-full resize-none rounded-xl border border-hairline bg-well px-3 py-2.5 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
          />
        </div>
        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <p class="text-[11px] font-medium">{{ t("securityTools.env.resolved") }}</p>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
              :disabled="!resolved"
              @click="copy('env', resolved)"
            >
              <Check v-if="isCopied('env')" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              {{ isCopied("env") ? t("common.copiedShort") : t("tools.copy") }}
            </Button>
          </div>
          <pre
            class="scrollbar-thin numeric h-[360px] overflow-auto whitespace-pre-wrap break-all rounded-xl border border-hairline bg-well-soft px-3 py-2.5 text-[11.5px] leading-relaxed"
            >{{ resolved }}</pre>
        </div>
      </div>

      <div class="rounded-xl border border-hairline">
        <div class="border-b border-hairline px-3 py-2 text-[11px] font-medium">
          {{ t("securityTools.env.variables", { count: Object.keys(parsed.values).length }) }}
        </div>
        <div class="scrollbar-thin max-h-[280px] overflow-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="sticky top-0 bg-surface">
              <tr class="border-b border-hairline text-left text-muted-foreground">
                <th class="w-12 px-2 py-1.5 font-medium">#</th>
                <th class="px-2 py-1.5 font-medium">KEY</th>
                <th class="px-2 py-1.5 font-medium">VALUE</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="entry in parsed.entries.filter((item) => !item.comment)"
                :key="`${entry.line}-${entry.key}`"
                class="border-b border-hairline/60 last:border-0"
              >
                <td class="numeric px-2 py-1 text-[9px] text-muted-foreground">{{ entry.line }}</td>
                <td class="numeric px-2 py-1">
                  <span :class="entry.error ? 'text-error' : 'text-rx'">{{ entry.key }}</span>
                  <span v-if="entry.error" class="ml-1 text-[9px] text-error">{{
                    t(`securityTools.env.errors.${entry.error}`)
                  }}</span>
                </td>
                <td class="numeric break-all px-2 py-1 text-muted-foreground">
                  {{ entry.value || "—" }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <p
        v-if="parsed.unresolved.length"
        class="rounded-lg border border-warn/25 bg-warn/10 px-3 py-2 text-[10.5px] text-warn"
      >
        {{ t("securityTools.env.unresolved") }}:
        <span class="numeric">{{ parsed.unresolved.join(", ") }}</span>
      </p>
    </div>
  </ToolPanelFrame>
</template>
