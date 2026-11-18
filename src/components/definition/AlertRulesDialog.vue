<script setup lang="ts">
import { BellRing, Plus, Trash2 } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { resolvePoints, toDefinitionInput } from "@/lib/definition";
import { formatAddress } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type {
  AlertOperator,
  AlertRule,
  AlertSeverity,
  Definition,
  RegisterPoint,
} from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  definition: Definition | null;
  addresses: number[];
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();
const rules = ref<AlertRule[]>([]);
const bulkRule = ref<AlertRule>(emptyRule());
const saving = ref(false);

const operatorOptions = computed<Array<{ label: string; value: AlertOperator }>>(() => [
  { label: t("alertRules.operators.gt"), value: "gt" },
  { label: t("alertRules.operators.lt"), value: "lt" },
  { label: t("alertRules.operators.gte"), value: "gte" },
  { label: t("alertRules.operators.lte"), value: "lte" },
  { label: t("alertRules.operators.eq"), value: "eq" },
  { label: t("alertRules.operators.ne"), value: "ne" },
]);

const severityOptions = computed<Array<{ label: string; value: AlertSeverity }>>(() => [
  { label: t("alertRules.severityInfo"), value: "info" },
  { label: t("alertRules.severityWarning"), value: "warning" },
  { label: t("alertRules.severityCritical"), value: "critical" },
]);

const bulk = computed(() => props.addresses.length > 1);
const addressLabel = computed(() =>
  props.addresses.map((address) => formatAddress(address, settings.addressBase)).join(", "),
);

function emptyRule(): AlertRule {
  return {
    id: newRuleId(),
    description: "",
    severity: "warning",
    operator: "gt",
    value: 0,
    delayMs: 0,
    enabled: true,
  };
}

function newRuleId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `rule-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function pointFor(address: number): RegisterPoint {
  const definition = props.definition;
  if (!definition) throw new Error(t("alertRules.noDefinition"));
  const points = definition.points.length ? definition.points : resolvePoints(definition);
  const existing = points.find((point) => point.address === address);
  if (existing) return existing;
  return {
    address,
    dataType: definition.dataType,
    byteOrder: definition.byteOrder,
    meaning: definition.aliases[address] ?? "",
    unit: "",
    scale: 1,
    offset: 0,
    alerts: [],
  };
}

watch(
  () => [open.value, props.addresses.join(","), props.definition?.id] as const,
  () => {
    if (!open.value) return;
    if (bulk.value) {
      bulkRule.value = emptyRule();
      return;
    }
    const address = props.addresses[0];
    if (address === undefined) {
      rules.value = [];
      return;
    }
    rules.value = pointFor(address).alerts.map((rule) => ({
      ...rule,
      description: rule.description ?? "",
      severity: rule.severity ?? "warning",
      recovery: rule.recovery
        ? { ...rule.recovery, delayMs: rule.recovery.delayMs ?? 0 }
        : undefined,
    }));
  },
  { immediate: true },
);

function addRule(): void {
  rules.value = [...rules.value, emptyRule()];
}

function setRecovery(rule: AlertRule, enabled: boolean): void {
  rule.recovery = enabled ? { operator: "lt", value: 0, delayMs: 0 } : undefined;
}

function removeRule(index: number): void {
  rules.value = rules.value.filter((_, ruleIndex) => ruleIndex !== index);
}

function normalizeRule(rule: AlertRule): AlertRule | null {
  if (
    !Number.isFinite(rule.value) ||
    !Number.isFinite(rule.delayMs) ||
    (rule.recovery &&
      (!Number.isFinite(rule.recovery.value) || !Number.isFinite(rule.recovery.delayMs)))
  ) {
    return null;
  }
  return {
    ...rule,
    value: Number(rule.value),
    delayMs: Math.max(0, Math.trunc(rule.delayMs)),
    enabled: Boolean(rule.enabled),
    description: (rule.description ?? "").trim(),
    severity: rule.severity ?? "warning",
    recovery: rule.recovery
      ? {
          operator: rule.recovery.operator,
          value: Number(rule.recovery.value),
          delayMs: Math.max(0, Math.trunc(rule.recovery.delayMs)),
        }
      : undefined,
  };
}

async function save(): Promise<void> {
  const definition = props.definition;
  if (!definition || saving.value) return;
  const addresses = props.addresses;
  if (!addresses.length) return;

  const nextRules = bulk.value ? [] : rules.value.map(normalizeRule);
  if (nextRules.some((rule) => rule === null)) {
    toast.error(t("alertRules.invalid"));
    return;
  }
  const template = bulk.value ? normalizeRule(bulkRule.value) : null;
  if (bulk.value && !template) {
    toast.error(t("alertRules.invalid"));
    return;
  }

  const basePoints = definition.points.length ? definition.points : resolvePoints(definition);
  const points = [...basePoints];
  for (const address of addresses) {
    let index = points.findIndex((point) => point.address === address);
    if (index < 0) {
      points.push(pointFor(address));
      index = points.length - 1;
    }
    const existing = points[index];
    if (!existing) continue;
    const alerts = bulk.value
      ? [...existing.alerts, { ...template!, id: newRuleId() }]
      : (nextRules as AlertRule[]);
    points[index] = { ...existing, alerts };
  }
  points.sort((left, right) => left.address - right.address);

  saving.value = true;
  try {
    await sessions.updateDefinition(definition.id, {
      ...toDefinitionInput(definition),
      points,
    });
    toast.success(bulk.value ? t("alertRules.savedBulk") : t("alertRules.saved"), {
      description: t("alertRules.savedDescription", {
        address: addressLabel.value,
        count: bulk.value ? 1 : rules.value.length,
      }),
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error(t("alertRules.saveFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[620px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-warn/25 bg-warn/10 text-warn"
          >
            <BellRing class="size-3.5" />
          </span>
          {{ bulk ? t("alertRules.bulkTitle") : t("alertRules.title") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{
            t("alertRules.description", {
              name: definition?.name ?? t("alertRules.noDefinition"),
              address: addressLabel,
            })
          }}
        </DialogDescription>
      </DialogHeader>

      <div class="scrollbar-thin max-h-[62vh] space-y-3 overflow-y-auto px-5 py-4">
        <template v-if="bulk">
          <p
            class="rounded-lg border border-warn/20 bg-warn/5 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
          >
            {{ t("alertRules.bulkHint", { count: addresses.length }) }}
          </p>
          <div class="space-y-3 rounded-xl border border-hairline bg-overlay-1 p-3">
            <div class="grid grid-cols-[1fr_160px] gap-2">
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground">
                  {{ t("alertRules.alertDescription") }}
                </Label>
                <Input
                  v-model="bulkRule.description"
                  :placeholder="t('alertRules.descriptionPlaceholder')"
                  class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
                />
              </div>
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground">
                  {{ t("alertRules.severity") }}
                </Label>
                <FormSelect v-model="bulkRule.severity" :options="severityOptions" />
              </div>
            </div>
            <div class="grid grid-cols-[1fr_100px] gap-2">
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground">
                  {{ t("alertRules.operator") }}
                </Label>
                <FormSelect v-model="bulkRule.operator" :options="operatorOptions" />
              </div>
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground">
                  {{ t("alertRules.threshold") }}
                </Label>
                <Input
                  v-model.number="bulkRule.value"
                  type="number"
                  class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
                />
              </div>
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground">
                  {{ t("alertRules.delay") }}
                </Label>
                <Input
                  v-model.number="bulkRule.delayMs"
                  type="number"
                  min="0"
                  class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
                />
              </div>
              <div class="flex items-end justify-between gap-2 pb-1">
                <span class="text-[11px] text-muted-foreground">{{ t("alertRules.enabled") }}</span>
                <Switch v-model="bulkRule.enabled" />
              </div>
            </div>

            <div class="mt-3 rounded-lg border border-hairline bg-well/60 p-2.5">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <p class="text-[11px] font-medium text-foreground">
                    {{ t("alertRules.recoveryTitle") }}
                  </p>
                  <p class="mt-0.5 text-[10.5px] leading-relaxed text-muted-foreground">
                    {{
                      bulkRule.recovery
                        ? t("alertRules.recoveryHint")
                        : t("alertRules.recoveryDefault")
                    }}
                  </p>
                </div>
                <div class="flex shrink-0 items-center gap-2">
                  <span class="text-[10.5px] text-muted-foreground">
                    {{ t("alertRules.recoveryConfigure") }}
                  </span>
                  <Switch
                    :model-value="Boolean(bulkRule.recovery)"
                    @update:model-value="(checked) => setRecovery(bulkRule, Boolean(checked))"
                  />
                </div>
              </div>
              <div v-if="bulkRule.recovery" class="mt-2 grid grid-cols-[1fr_100px_120px] gap-2">
                <div class="space-y-1">
                  <Label class="text-[10.5px] text-muted-foreground">
                    {{ t("alertRules.recoveryOperator") }}
                  </Label>
                  <FormSelect v-model="bulkRule.recovery.operator" :options="operatorOptions" />
                </div>
                <div class="space-y-1">
                  <Label class="text-[10.5px] text-muted-foreground">
                    {{ t("alertRules.recoveryThreshold") }}
                  </Label>
                  <Input
                    v-model.number="bulkRule.recovery.value"
                    type="number"
                    class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[12px]"
                  />
                </div>
                <div class="space-y-1">
                  <Label class="text-[10.5px] text-muted-foreground">
                    {{ t("alertRules.recoveryDelay") }}
                  </Label>
                  <Input
                    v-model.number="bulkRule.recovery.delayMs"
                    type="number"
                    min="0"
                    class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[12px]"
                  />
                </div>
              </div>
            </div>
          </div>
        </template>

        <template v-else>
          <div v-if="rules.length" class="space-y-2">
            <div
              v-for="(rule, index) in rules"
              :key="rule.id"
              class="space-y-2 rounded-xl border border-hairline bg-overlay-1 p-3"
            >
              <div class="grid grid-cols-[1fr_160px] gap-2">
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground">
                    {{ t("alertRules.alertDescription") }}
                  </Label>
                  <Input
                    v-model="rule.description"
                    :placeholder="t('alertRules.descriptionPlaceholder')"
                    class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
                  />
                </div>
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground">
                    {{ t("alertRules.severity") }}
                  </Label>
                  <FormSelect v-model="rule.severity" :options="severityOptions" />
                </div>
              </div>
              <div class="grid grid-cols-[1fr_100px_120px_38px] items-end gap-2">
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground">
                    {{ t("alertRules.operator") }}
                  </Label>
                  <FormSelect v-model="rule.operator" :options="operatorOptions" />
                </div>
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground">
                    {{ t("alertRules.threshold") }}
                  </Label>
                  <Input
                    v-model.number="rule.value"
                    type="number"
                    class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
                  />
                </div>
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground">
                    {{ t("alertRules.delay") }}
                  </Label>
                  <Input
                    v-model.number="rule.delayMs"
                    type="number"
                    min="0"
                    class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
                  />
                </div>
                <button
                  type="button"
                  class="flex size-9 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
                  :aria-label="t('alertRules.deleteRule')"
                  @click="removeRule(index)"
                >
                  <Trash2 class="size-3.5" />
                </button>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-muted-foreground">
                  {{ t("alertRules.enabledRules") }}
                </span>
                <Switch v-model="rule.enabled" />
              </div>

              <div class="rounded-lg border border-hairline bg-well/60 p-2.5">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0">
                    <p class="text-[11px] font-medium text-foreground">
                      {{ t("alertRules.recoveryTitle") }}
                    </p>
                    <p class="mt-0.5 text-[10.5px] leading-relaxed text-muted-foreground">
                      {{
                        rule.recovery
                          ? t("alertRules.recoveryHint")
                          : t("alertRules.recoveryDefault")
                      }}
                    </p>
                  </div>
                  <div class="flex shrink-0 items-center gap-2">
                    <span class="text-[10.5px] text-muted-foreground">
                      {{ t("alertRules.recoveryConfigure") }}
                    </span>
                    <Switch
                      :model-value="Boolean(rule.recovery)"
                      @update:model-value="(checked) => setRecovery(rule, Boolean(checked))"
                    />
                  </div>
                </div>
                <div v-if="rule.recovery" class="mt-2 grid grid-cols-[1fr_100px_120px] gap-2">
                  <div class="space-y-1">
                    <Label class="text-[10.5px] text-muted-foreground">
                      {{ t("alertRules.recoveryOperator") }}
                    </Label>
                    <FormSelect v-model="rule.recovery.operator" :options="operatorOptions" />
                  </div>
                  <div class="space-y-1">
                    <Label class="text-[10.5px] text-muted-foreground">
                      {{ t("alertRules.recoveryThreshold") }}
                    </Label>
                    <Input
                      v-model.number="rule.recovery.value"
                      type="number"
                      class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[12px]"
                    />
                  </div>
                  <div class="space-y-1">
                    <Label class="text-[10.5px] text-muted-foreground">
                      {{ t("alertRules.recoveryDelay") }}
                    </Label>
                    <Input
                      v-model.number="rule.recovery.delayMs"
                      type="number"
                      min="0"
                      class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[12px]"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
          <p
            v-else
            class="rounded-lg border border-dashed border-hairline px-3 py-4 text-center text-[11px] text-muted-foreground"
          >
            {{ t("alertRules.empty") }}
          </p>
          <Button
            variant="outline"
            size="sm"
            class="h-8 gap-1.5 border-hairline-strong bg-overlay-1 text-[11.5px]"
            @click="addRule"
          >
            <Plus class="size-3" />
            {{ t("alertRules.addRule") }}
          </Button>
        </template>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          {{ t("alertRules.hint") }}
        </p>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-hairline-strong bg-transparent text-[12px]"
          >
            {{ t("common.cancel") }}
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 min-w-[96px] text-[12px]" :disabled="saving" @click="save">
          {{ saving ? t("alertRules.saving") : t("alertRules.save") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
