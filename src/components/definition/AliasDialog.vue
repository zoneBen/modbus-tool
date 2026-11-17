<script setup lang="ts">
import { Tag } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
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
import { resolvePoints, toDefinitionInput } from "@/lib/definition";
import { formatAddress } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { Definition } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  definition: Definition | null;
  address: number;
  currentAlias: string;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();
const alias = ref("");
const saving = ref(false);

const addressLabel = computed(() => formatAddress(props.address, settings.addressBase));

watch(
  () => [open.value, props.currentAlias] as const,
  () => {
    if (open.value) alias.value = props.currentAlias;
  },
  { immediate: true },
);

async function save(): Promise<void> {
  const definition = props.definition;
  if (!definition || saving.value) return;
  const nextAlias = alias.value.trim();
  const aliases = { ...definition.aliases };
  if (nextAlias) aliases[props.address] = nextAlias;
  else delete aliases[props.address];
  const basePoints = definition.points.length ? definition.points : resolvePoints(definition);
  const points = basePoints.map((point) =>
    point.address === props.address ? { ...point, meaning: nextAlias } : point,
  );
  if (
    JSON.stringify(aliases) === JSON.stringify(definition.aliases) &&
    !points.some(
      (point) =>
        point.address === props.address &&
        point.meaning !==
          (definition.points.find((entry) => entry.address === props.address)?.meaning ?? ""),
    )
  ) {
    open.value = false;
    return;
  }
  saving.value = true;
  try {
    await sessions.updateDefinition(definition.id, {
      ...toDefinitionInput(definition),
      aliases,
      points,
    });
    toast.success(t("aliasEditor.updated"), {
      description: nextAlias
        ? t("aliasEditor.updatedDescription", {
            address: addressLabel.value,
            alias: nextAlias,
          })
        : t("aliasEditor.restored", { address: addressLabel.value }),
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error(t("aliasEditor.updateFailed"), {
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
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[400px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <Tag class="size-3.5" />
          </span>
          {{ t("aliasEditor.title") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{
            t("aliasEditor.description", {
              address: addressLabel,
              name: definition?.name ?? t("aliasEditor.noDefinition"),
            })
          }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-2 px-5 py-4">
        <Label class="text-[11px] text-muted-foreground">{{ t("aliasEditor.alias") }}</Label>
        <Input
          v-model="alias"
          class="h-10 rounded-lg border-hairline-strong bg-well text-[13px]"
          :placeholder="t('aliasEditor.placeholder')"
          @keydown.enter="save"
        />
        <p class="text-[11px] leading-relaxed text-muted-foreground">
          {{ t("aliasEditor.hint") }}
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
        <Button size="sm" class="h-8 min-w-[76px] text-[12px]" :disabled="saving" @click="save">
          {{ saving ? t("aliasEditor.saving") : t("aliasEditor.save") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
