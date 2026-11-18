<script setup lang="ts">
import { Network, Radio, Send, Server, ArrowLeftRight } from "@lucide/vue";
import type { Component } from "vue";
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
import { defaultNetworkSession, useNetworkStore } from "@/stores/network";
import type { NetworkMode } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const network = useNetworkStore();
const { t } = useI18n();

interface ModeOption {
  value: NetworkMode;
  label: string;
  hint: string;
  icon: Component;
}

const modeOptions = computed<ModeOption[]>(() => [
  {
    value: "tcp-server",
    label: t("network.modes.tcpServer"),
    hint: t("network.serverHint"),
    icon: Server,
  },
  {
    value: "tcp-client",
    label: t("network.modes.tcpClient"),
    hint: t("network.clientHint"),
    icon: ArrowLeftRight,
  },
  {
    value: "udp-server",
    label: t("network.modes.udpServer"),
    hint: t("network.serverHint"),
    icon: Radio,
  },
  {
    value: "udp-client",
    label: t("network.modes.udpClient"),
    hint: t("network.clientHint"),
    icon: Send,
  },
  {
    value: "tcp-relay",
    label: t("network.modes.tcpRelay"),
    hint: t("network.relayHint"),
    icon: Network,
  },
]);

const mode = ref<NetworkMode>("tcp-server");
const name = ref("");
const localHost = ref("0.0.0.0");
const localPort = ref(8080);
const remoteHost = ref("127.0.0.1");
const remotePort = ref(502);
const nameTouched = ref(false);

const isServer = computed(() => mode.value === "tcp-server" || mode.value === "udp-server");
const isRelay = computed(() => mode.value === "tcp-relay");
const isClient = computed(() => mode.value === "tcp-client" || mode.value === "udp-client");
const current = computed(
  () => modeOptions.value.find((option) => option.value === mode.value) ?? modeOptions.value[0],
);

function defaultName(): string {
  const count = network.sessions.filter((session) => session.mode === mode.value).length + 1;
  return `${current.value?.label ?? "Network"} ${String(count).padStart(2, "0")}`;
}

function applyModeDefaults(): void {
  const defaults = defaultNetworkSession(mode.value, "");
  localHost.value = defaults.localHost;
  localPort.value = defaults.localPort;
  remoteHost.value = defaults.remoteHost;
  remotePort.value = defaults.remotePort;
  if (!nameTouched.value) name.value = defaultName();
}

watch(
  () => open.value,
  (isOpen) => {
    if (!isOpen) return;
    mode.value = "tcp-server";
    nameTouched.value = false;
    applyModeDefaults();
  },
  { immediate: true },
);

watch(mode, () => {
  if (open.value) applyModeDefaults();
});

function fail(message: string): void {
  toast.error(message);
}

function create(): void {
  const sessionName = name.value.trim();
  if (!sessionName) {
    fail(t("network.nameRequired"));
    return;
  }
  if (isServer.value || isRelay.value) {
    const parsed = Number(localPort.value);
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
      fail(t("network.errors.listenPortRange"));
      return;
    }
  }
  if (isClient.value || isRelay.value) {
    if (!remoteHost.value.trim()) {
      fail(t("network.errors.remoteHostRequired"));
      return;
    }
    const parsed = Number(remotePort.value);
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
      fail(t("network.errors.remotePortRange"));
      return;
    }
  }
  const base = defaultNetworkSession(mode.value, sessionName);
  network.createSession({
    ...base,
    name: sessionName,
    localHost: isServer.value || isRelay.value ? localHost.value.trim() || "0.0.0.0" : "0.0.0.0",
    localPort: isServer.value || isRelay.value ? Number(localPort.value) : 0,
    remoteHost: isServer.value ? "" : remoteHost.value.trim(),
    remotePort: isServer.value ? 0 : Number(remotePort.value),
  });
  open.value = false;
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[540px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-rx/25 bg-rx/10 text-rx"
          >
            <Network class="size-3.5" />
          </span>
          {{ t("network.newSession") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ t("network.newSessionHint") }}
        </DialogDescription>
      </DialogHeader>

      <div class="scrollbar-thin max-h-[68vh] space-y-4 overflow-y-auto px-5 py-4">
        <div class="space-y-2">
          <Label class="text-[11px] text-muted-foreground">{{ t("network.mode") }}</Label>
          <div class="grid grid-cols-2 gap-1.5" role="group" :aria-label="t('network.mode')">
            <button
              v-for="option in modeOptions"
              :key="option.value"
              type="button"
              class="flex items-center gap-2 rounded-lg border px-2.5 py-2 text-left transition-colors"
              :class="[
                option.value === 'tcp-relay' ? 'col-span-2' : '',
                mode === option.value
                  ? 'border-rx/35 bg-rx/10 text-rx'
                  : 'border-hairline bg-well text-muted-foreground hover:border-hairline-strong hover:text-foreground',
              ]"
              :aria-pressed="mode === option.value"
              @click="mode = option.value"
            >
              <component :is="option.icon" class="size-3.5 shrink-0" />
              <span class="truncate text-[11px] font-medium">{{ option.label }}</span>
            </button>
          </div>
          <p class="text-[10.5px] leading-relaxed text-muted-foreground/80">
            {{ current?.hint }}
          </p>
        </div>

        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("network.sessionName") }}</Label>
          <Input
            v-model="name"
            :placeholder="t('network.sessionNamePlaceholder')"
            class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            @input="nameTouched = true"
            @keydown.enter="create"
          />
        </div>

        <div v-if="isServer" class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("network.listenHost") }}</Label>
            <Input
              v-model="localHost"
              :placeholder="t('network.localHostPlaceholder')"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("network.listenPort") }}</Label>
            <Input
              v-model.number="localPort"
              type="number"
              min="1"
              max="65535"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
        </div>

        <div v-else-if="isClient" class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("network.targetHost") }}</Label>
            <Input
              v-model="remoteHost"
              :placeholder="t('network.remoteHostPlaceholder')"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("network.targetPort") }}</Label>
            <Input
              v-model.number="remotePort"
              type="number"
              min="1"
              max="65535"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
        </div>

        <div v-else class="space-y-3">
          <div class="grid grid-cols-2 gap-3">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{ t("network.listenHost") }}</Label>
              <Input
                v-model="localHost"
                :placeholder="t('network.localHostPlaceholder')"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{ t("network.listenPort") }}</Label>
              <Input
                v-model.number="localPort"
                type="number"
                min="1"
                max="65535"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{
                t("network.upstreamHost")
              }}</Label>
              <Input
                v-model="remoteHost"
                :placeholder="t('network.remoteHostPlaceholder')"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{
                t("network.upstreamPort")
              }}</Label>
              <Input
                v-model.number="remotePort"
                type="number"
                min="1"
                max="65535"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>
        </div>
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
        <Button size="sm" class="h-8 gap-1.5 text-[12px]" @click="create">
          <Network class="size-3.5" />
          {{ t("network.createSession") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
