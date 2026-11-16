<script setup lang="ts">
import AppShell from "@/components/layout/AppShell.vue";
import { Toaster } from "@/components/ui/sonner";
</script>

<template>
  <AppShell />
  <Toaster position="bottom-right" theme="dark" :duration="2600" />
</template>
