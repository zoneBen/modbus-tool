<script setup lang="ts">
import { FileCode2, FileEdit, KeyRound, ScrollText, ServerCog } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import EnvParserTool from "@/components/tools/security/EnvParserTool.vue";
import BatchRenameTool from "@/components/tools/security/BatchRenameTool.vue";
import LogAnalyzerTool from "@/components/tools/security/LogAnalyzerTool.vue";
import PasswordGeneratorTool from "@/components/tools/security/PasswordGeneratorTool.vue";
import PortViewerTool from "@/components/tools/security/PortViewerTool.vue";
import ToolPlaceholder from "@/components/tools/ToolPlaceholder.vue";
import ToolboxWorkspace from "@/components/tools/ToolboxWorkspace.vue";
import type { ToolboxItem } from "@/components/tools/ToolboxWorkspace.vue";
import { SECURITY_TOOLS, isSecurityToolId } from "@/lib/tool-catalog";
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
  password: KeyRound,
  rename: FileEdit,
  log: ScrollText,
  env: FileCode2,
  ports: ServerCog,
} as const;

if (isSecurityToolId(props.initialTool)) {
  toolbox.setSecurityTool(props.initialTool);
}

const items = computed<ToolboxItem[]>(() =>
  SECURITY_TOOLS.map((tool) => ({
    id: tool.id,
    title: t(tool.labelKey),
    description: t(tool.descriptionKey),
    icon: icons[tool.id],
    disabled: tool.phase > 2,
    badge: tool.phase > 2 ? t("tools.comingSoon") : undefined,
  })),
);

const active = computed<string>({
  get: () => toolbox.config.securityTool,
  set: (value) => {
    if (isSecurityToolId(value)) toolbox.setSecurityTool(value);
  },
});

const activeItem = computed(() => items.value.find((item) => item.id === active.value));
</script>

<template>
  <ToolboxWorkspace
    v-model:active="active"
    :title="t('securityTools.title')"
    :subtitle="t('securityTools.subtitle')"
    :items="items"
    accent="signal"
  >
    <PasswordGeneratorTool v-if="active === 'password'" />
    <BatchRenameTool v-else-if="active === 'rename'" />
    <LogAnalyzerTool v-else-if="active === 'log'" />
    <EnvParserTool v-else-if="active === 'env'" />
    <PortViewerTool v-else-if="active === 'ports'" />
    <ToolPlaceholder
      v-else
      :title="activeItem?.title ?? t('securityTools.title')"
      :description="activeItem?.description ?? ''"
    />
  </ToolboxWorkspace>
</template>
