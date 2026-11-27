<script setup lang="ts">
import {
  Binary,
  Boxes,
  Braces,
  CalendarClock,
  Clock,
  Code2,
  Database,
  Dices,
  FileCode2,
  FileDiff,
  Hash,
  Images,
  KeyRound,
  Languages,
  Layers,
  QrCode,
  Regex,
  Search,
} from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import BaseTool from "@/components/tools/dev/BaseTool.vue";
import CodecTool from "@/components/tools/dev/CodecTool.vue";
import ComposeTool from "@/components/tools/dev/ComposeTool.vue";
import CronTool from "@/components/tools/dev/CronTool.vue";
import DiffTool from "@/components/tools/dev/DiffTool.vue";
import EncodingTool from "@/components/tools/dev/EncodingTool.vue";
import FormatterTool from "@/components/tools/dev/FormatterTool.vue";
import HashTool from "@/components/tools/dev/HashTool.vue";
import ImageTool from "@/components/tools/dev/ImageTool.vue";
import JwtTool from "@/components/tools/dev/JwtTool.vue";
import K8sTool from "@/components/tools/dev/K8sTool.vue";
import QrTool from "@/components/tools/dev/QrTool.vue";
import RegexTool from "@/components/tools/dev/RegexTool.vue";
import SearchTool from "@/components/tools/dev/SearchTool.vue";
import SqlTool from "@/components/tools/dev/SqlTool.vue";
import TimestampTool from "@/components/tools/dev/TimestampTool.vue";
import UuidTool from "@/components/tools/dev/UuidTool.vue";
import YamlTool from "@/components/tools/dev/YamlTool.vue";
import ToolPlaceholder from "@/components/tools/ToolPlaceholder.vue";
import ToolboxWorkspace from "@/components/tools/ToolboxWorkspace.vue";
import type { ToolboxItem } from "@/components/tools/ToolboxWorkspace.vue";
import { DEV_TOOLS, isDevToolId } from "@/lib/tool-catalog";
import { useToolboxStore } from "@/stores/toolbox";

const props = withDefaults(
  defineProps<{
    initialTool?: string | null;
  }>(),
  { initialTool: null },
);

const toolbox = useToolboxStore();
const { t } = useI18n();

const icons = {
  formatter: Braces,
  yaml: FileCode2,
  timestamp: Clock,
  base: Binary,
  hash: Hash,
  search: Search,
  codec: Code2,
  regex: Regex,
  jwt: KeyRound,
  cron: CalendarClock,
  diff: FileDiff,
  compose: Boxes,
  k8s: Layers,
  sql: Database,
  uuid: Dices,
  encoding: Languages,
  image: Images,
  qr: QrCode,
} as const;

if (isDevToolId(props.initialTool)) {
  toolbox.setDevTool(props.initialTool);
}

const items = computed<ToolboxItem[]>(() =>
  DEV_TOOLS.map((tool) => ({
    id: tool.id,
    title: t(tool.labelKey),
    description: t(tool.descriptionKey),
    icon: icons[tool.id],
    disabled: tool.phase > 1,
    badge: tool.phase > 1 ? t("tools.comingSoon") : undefined,
  })),
);

const active = computed<string>({
  get: () => toolbox.config.devTool,
  set: (value) => {
    if (isDevToolId(value)) toolbox.setDevTool(value);
  },
});

const activeItem = computed(() => items.value.find((item) => item.id === active.value));
</script>

<template>
  <ToolboxWorkspace
    v-model:active="active"
    :title="t('devTools.title')"
    :subtitle="t('devTools.subtitle')"
    :items="items"
    accent="sim"
  >
    <FormatterTool v-if="active === 'formatter'" />
    <YamlTool v-else-if="active === 'yaml'" />
    <TimestampTool v-else-if="active === 'timestamp'" />
    <BaseTool v-else-if="active === 'base'" />
    <HashTool v-else-if="active === 'hash'" />
    <SearchTool v-else-if="active === 'search'" />
    <CodecTool v-else-if="active === 'codec'" />
    <RegexTool v-else-if="active === 'regex'" />
    <JwtTool v-else-if="active === 'jwt'" />
    <CronTool v-else-if="active === 'cron'" />
    <DiffTool v-else-if="active === 'diff'" />
    <ComposeTool v-else-if="active === 'compose'" />
    <K8sTool v-else-if="active === 'k8s'" />
    <SqlTool v-else-if="active === 'sql'" />
    <UuidTool v-else-if="active === 'uuid'" />
    <EncodingTool v-else-if="active === 'encoding'" />
    <ImageTool v-else-if="active === 'image'" />
    <QrTool v-else-if="active === 'qr'" />
    <ToolPlaceholder
      v-else
      :title="activeItem?.title ?? t('devTools.title')"
      :description="activeItem?.description ?? ''"
    />
  </ToolboxWorkspace>
</template>
