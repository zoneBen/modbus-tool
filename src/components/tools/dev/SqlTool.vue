<script setup lang="ts">
import { Check, Copy, Database, WandSparkles } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { formatSql } from "@/lib/sql-format";
import { useToolboxStore } from "@/stores/toolbox";

const KEYWORDS =
  /(\b(?:SELECT|FROM|WHERE|GROUP|BY|ORDER|HAVING|LIMIT|OFFSET|UNION|ALL|INSERT|INTO|VALUES|UPDATE|SET|DELETE|RETURNING|WITH|JOIN|LEFT|RIGHT|FULL|OUTER|INNER|CROSS|ON|AND|OR|NOT|NULL|AS|DISTINCT|CASE|WHEN|THEN|ELSE|END|ASC|DESC)\b)/i;

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const input = toolbox.toolPref(
  "dev.sql.input",
  "select u.id,u.name,count(o.id) as orders from users u left join orders o on o.user_id=u.id where u.active=1 group by u.id,u.name order by orders desc limit 20;",
);
const indent = toolbox.toolPref("dev.sql.indent", "2");
const formatted = ref("");
const error = ref<string | null>(null);

const indentOptions = [
  { value: "2", label: "2" },
  { value: "4", label: "4" },
];

function format(): void {
  try {
    formatted.value = formatSql(input.value, Number(indent.value) || 2);
    error.value = null;
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
}

const lines = computed(() =>
  formatted.value.split("\n").map((line) =>
    line
      .split(KEYWORDS)
      .filter((part) => part !== "")
      .map((part) => ({ text: part, keyword: KEYWORDS.test(part) })),
  ),
);

format();
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.sql.title')"
    :description="t('devTools.tools.sql.description')"
    :icon="Database"
  >
    <div class="mx-auto max-w-[1080px] space-y-3">
      <div
        class="flex flex-wrap items-end gap-2 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.sql.indent") }}</Label>
          <div class="w-24">
            <FormSelect v-model="indent" :options="indentOptions" />
          </div>
        </div>
        <Button size="sm" class="h-9 gap-1.5 text-[11px]" @click="format">
          <WandSparkles class="size-3.5" />
          {{ t("devTools.sql.format") }}
        </Button>
        <Button
          variant="outline"
          size="sm"
          class="h-9 gap-1.5 border-hairline-strong bg-transparent text-[11px]"
          :disabled="!formatted"
          @click="copy('sql', formatted)"
        >
          <Check v-if="isCopied('sql')" class="size-3 text-signal" />
          <Copy v-else class="size-3" />
          {{ isCopied("sql") ? t("common.copiedShort") : t("tools.copy") }}
        </Button>
      </div>

      <div class="grid gap-3 lg:grid-cols-2">
        <div class="space-y-1.5">
          <p class="text-[11px] font-medium">{{ t("devTools.sql.input") }}</p>
          <textarea
            v-model="input"
            class="scrollbar-thin numeric h-[460px] w-full resize-none rounded-xl border border-hairline bg-well px-3 py-2.5 text-[11.5px] leading-relaxed outline-none focus:border-signal/50"
          />
        </div>
        <div class="space-y-1.5">
          <p class="text-[11px] font-medium">{{ t("devTools.sql.output") }}</p>
          <div
            class="scrollbar-thin numeric h-[460px] overflow-auto rounded-xl border border-hairline bg-well px-3 py-2.5 text-[11.5px] leading-relaxed"
          >
            <div v-for="(parts, index) in lines" :key="index" class="whitespace-pre-wrap break-all">
              <span
                v-for="(part, partIndex) in parts"
                :key="partIndex"
                :class="part.keyword ? 'font-medium text-rx' : 'text-foreground'"
                >{{ part.text }}</span
              >
            </div>
            <p v-if="error" class="text-error">{{ error }}</p>
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
