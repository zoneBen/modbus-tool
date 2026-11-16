<script setup lang="ts">
import { Tag } from "@lucide/vue";
import { computed, ref, watch } from "vue";
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
import { toDefinitionInput } from "@/lib/definition";
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
  if (JSON.stringify(aliases) === JSON.stringify(definition.aliases)) {
    open.value = false;
    return;
  }
  saving.value = true;
  try {
    await sessions.updateDefinition(definition.id, { ...toDefinitionInput(definition), aliases });
    toast.success("别名已更新", {
      description: nextAlias
        ? `地址 ${addressLabel.value} → ${nextAlias}`
        : `地址 ${addressLabel.value} 已恢复显示地址`,
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error("更新别名失败", {
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
          编辑别名 / 点位
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          地址 {{ addressLabel }} · {{ definition?.name ?? "未选择定义" }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-2 px-5 py-4">
        <Label class="text-[11px] text-muted-foreground">别名</Label>
        <Input
          v-model="alias"
          class="h-10 rounded-lg border-hairline-strong bg-well text-[13px]"
          placeholder="例如 电机温度"
          @keydown.enter="save"
        />
        <p class="text-[11px] leading-relaxed text-muted-foreground">
          留空保存会移除该地址的别名，表格恢复显示地址。
        </p>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-hairline-strong bg-transparent text-[12px]"
          >
            取消
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 min-w-[76px] text-[12px]" :disabled="saving" @click="save">
          {{ saving ? "保存中…" : "保存别名" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
