<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { FrameField } from "@/types/models";

const { t } = useI18n();

const props = withDefaults(
  defineProps<{
    bytes: number[];
    fields?: FrameField[];
    uppercase?: boolean;
  }>(),
  { fields: () => [], uppercase: true },
);

const toneClass: Record<FrameField["tone"], string> = {
  header: "text-muted-foreground",
  unit: "text-warn/90",
  function: "text-rx",
  data: "text-foreground",
  checksum: "text-error/65",
};

function hexByte(byte: number): string {
  const text = byte.toString(16).padStart(2, "0");
  return props.uppercase ? text.toUpperCase() : text;
}

function fieldLabel(label: string): string {
  const labels: Record<string, string> = {
    "事务 ID": t("monitor.fieldLabels.transactionId"),
    "协议 ID": t("monitor.fieldLabels.protocolId"),
    长度: t("monitor.fieldLabels.length"),
    "从站 ID": t("monitor.fieldLabels.unitId"),
    功能码: t("monitor.fieldLabels.function"),
    字节数: t("monitor.fieldLabels.byteCount"),
    "ASCII 帧": t("monitor.fieldLabels.asciiFrame"),
    "PDU 长度": t("monitor.fieldLabels.pduLength"),
    CRC16: t("monitor.fieldLabels.crc"),
    LRC: t("monitor.fieldLabels.lrc"),
    数据: t("monitor.data"),
  };
  return labels[label] ?? label;
}

interface Group {
  tone: FrameField["tone"];
  label: string;
  text: string;
}

const groups = computed<Group[]>(() => {
  if (!props.bytes.length) return [];
  if (!props.fields.length) {
    return [
      {
        tone: "data",
        label: t("monitor.frameData"),
        text: props.bytes.map(hexByte).join(" "),
      },
    ];
  }
  const tones = new Map<number, { tone: FrameField["tone"]; label: string }>();
  for (const field of props.fields) {
    for (let index = 0; index < field.length; index += 1) {
      tones.set(field.offset + index, {
        tone: field.tone,
        label: fieldLabel(field.label),
      });
    }
  }
  const result: Group[] = [];
  let current: Group | null = null;
  props.bytes.forEach((byte, index) => {
    const info = tones.get(index) ?? { tone: "data" as const, label: t("monitor.data") };
    if (!current || current.tone !== info.tone || current.label !== info.label) {
      current = { tone: info.tone, label: info.label, text: hexByte(byte) };
      result.push(current);
    } else {
      current.text += ` ${hexByte(byte)}`;
    }
  });
  return result;
});
</script>

<template>
  <span class="numeric inline text-[11.5px] leading-5">
    <template v-for="(group, index) in groups" :key="`${group.label}-${index}`">
      <span v-if="index > 0" class="mx-1 text-foreground/15">·</span>
      <span :class="toneClass[group.tone]" :title="group.label">{{ group.text }}</span>
    </template>
    <span v-if="!groups.length" class="text-muted-foreground">—</span>
  </span>
</template>
