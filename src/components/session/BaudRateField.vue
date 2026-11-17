<script setup lang="ts">
import { useId } from "vue";
import { Input } from "@/components/ui/input";

const model = defineModel<number>({ required: true });
const listId = useId();

const commonRates = [
  1200, 2400, 4800, 9600, 14400, 19200, 28800, 38400, 57600, 115200, 128000, 230400, 256000, 460800,
  921600,
];

function update(value: string | number): void {
  const parsed = Number(value);
  model.value = Number.isFinite(parsed) ? Math.max(0, Math.trunc(parsed)) : 0;
}
</script>

<template>
  <div>
    <Input
      :model-value="String(model)"
      type="number"
      min="1"
      step="1"
      inputmode="numeric"
      :list="listId"
      class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
      @update:model-value="update"
    />
    <datalist :id="listId">
      <option v-for="rate in commonRates" :key="rate" :value="rate" />
    </datalist>
  </div>
</template>
