<script setup lang="ts">
import { Cable, Radio, Server, Wifi } from "@lucide/vue";
import { computed } from "vue";
import type { SessionRole, TransportKind } from "@/types/models";

const props = defineProps<{
  transport: TransportKind;
  role?: SessionRole;
}>();

const icon = computed(() => {
  if (props.role === "slave") return Server;
  if (props.transport === "serial") return Cable;
  if (props.transport === "tcp") return Wifi;
  return Radio;
});
</script>

<template>
  <component :is="icon" />
</template>
