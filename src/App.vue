<script setup lang="ts">
import { watchEffect } from "vue";
import AppShell from "@/components/layout/AppShell.vue";
import { Toaster } from "@/components/ui/sonner";
import { useSettingsStore } from "@/stores/settings";

const settings = useSettingsStore();

watchEffect(() => settings.applyTheme());
</script>

<template>
  <AppShell />
  <Toaster position="bottom-right" :theme="settings.resolvedTheme" :duration="2600" />
</template>
