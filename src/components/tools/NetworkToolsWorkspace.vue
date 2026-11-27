<script setup lang="ts">
import {
  Activity,
  ChartLine,
  Gauge,
  Globe2,
  KeyRound,
  Lock,
  Network,
  Radar,
  Radio,
  Route,
  Search,
  Server,
  Terminal,
  Wifi,
} from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ArpTool from "@/components/tools/network/ArpTool.vue";
import DnsTool from "@/components/tools/network/DnsTool.vue";
import FileServerTool from "@/components/tools/network/FileServerTool.vue";
import HttpTool from "@/components/tools/network/HttpTool.vue";
import LanDiscoveryTool from "@/components/tools/network/LanDiscoveryTool.vue";
import LatencyTool from "@/components/tools/network/LatencyTool.vue";
import MqttTool from "@/components/tools/network/MqttTool.vue";
import PingTool from "@/components/tools/network/PingTool.vue";
import PublicIpTool from "@/components/tools/network/PublicIpTool.vue";
import SnmpTool from "@/components/tools/network/SnmpTool.vue";
import SpeedTestTool from "@/components/tools/network/SpeedTestTool.vue";
import SshTool from "@/components/tools/network/SshTool.vue";
import SslCertTool from "@/components/tools/network/SslCertTool.vue";
import TracerouteTool from "@/components/tools/network/TracerouteTool.vue";
import TelnetTool from "@/components/tools/network/TelnetTool.vue";
import ToolPlaceholder from "@/components/tools/ToolPlaceholder.vue";
import ToolboxWorkspace from "@/components/tools/ToolboxWorkspace.vue";
import type { ToolboxItem } from "@/components/tools/ToolboxWorkspace.vue";
import { NETWORK_TOOLS, isNetworkToolId } from "@/lib/tool-catalog";
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
  arp: Radar,
  ping: Activity,
  latency: ChartLine,
  traceroute: Route,
  dns: Search,
  telnet: Terminal,
  http: Globe2,
  fileserver: Server,
  mqtt: Radio,
  publicip: Globe2,
  ssh: KeyRound,
  discovery: Wifi,
  speedtest: Gauge,
  ssl: Lock,
  snmp: Network,
} as const;

if (isNetworkToolId(props.initialTool)) {
  toolbox.setNetworkTool(props.initialTool);
}

const items = computed<ToolboxItem[]>(() =>
  NETWORK_TOOLS.map((tool) => ({
    id: tool.id,
    title: t(tool.labelKey),
    description: t(tool.descriptionKey),
    icon: icons[tool.id],
    disabled: tool.phase > 2,
    badge: tool.phase > 2 ? t("tools.comingSoon") : undefined,
  })),
);

const active = computed<string>({
  get: () => toolbox.config.networkTool,
  set: (value) => {
    if (isNetworkToolId(value)) toolbox.setNetworkTool(value);
  },
});

const activeItem = computed(() => items.value.find((item) => item.id === active.value));
</script>

<template>
  <ToolboxWorkspace
    v-model:active="active"
    :title="t('networkTools.title')"
    :subtitle="t('networkTools.subtitle')"
    :items="items"
    accent="rx"
  >
    <ArpTool v-if="active === 'arp'" />
    <DnsTool v-else-if="active === 'dns'" />
    <TelnetTool v-else-if="active === 'telnet'" />
    <HttpTool v-else-if="active === 'http'" />
    <PingTool v-else-if="active === 'ping'" />
    <LatencyTool v-else-if="active === 'latency'" />
    <TracerouteTool v-else-if="active === 'traceroute'" />
    <FileServerTool v-else-if="active === 'fileserver'" />
    <MqttTool v-else-if="active === 'mqtt'" />
    <PublicIpTool v-else-if="active === 'publicip'" />
    <SshTool v-else-if="active === 'ssh'" />
    <LanDiscoveryTool v-else-if="active === 'discovery'" />
    <SpeedTestTool v-else-if="active === 'speedtest'" />
    <SslCertTool v-else-if="active === 'ssl'" />
    <SnmpTool v-else-if="active === 'snmp'" />
    <ToolPlaceholder
      v-else
      :title="activeItem?.title ?? t('networkTools.title')"
      :description="activeItem?.description ?? ''"
    />
  </ToolboxWorkspace>
</template>
