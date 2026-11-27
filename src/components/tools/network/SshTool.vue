<script setup lang="ts">
import { Check, Copy, FolderOpen, KeyRound, Play, Terminal } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { SshResult } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();

const host = toolbox.toolPref("network.ssh.host", "");
const port = toolbox.toolPref("network.ssh.port", "22");
const username = toolbox.toolPref("network.ssh.username", "root");
const auth = toolbox.toolPref<"password" | "key">("network.ssh.auth", "password");
const keyPath = toolbox.toolPref("network.ssh.keyPath", "");
const command = toolbox.toolPref("network.ssh.command", "uname -a");
const timeoutMs = toolbox.toolPref("network.ssh.timeout", "15000");
const password = ref("");
const passphrase = ref("");
const busy = ref(false);
const error = ref<string | null>(null);
const result = ref<SshResult | null>(null);

const authOptions = computed(() => [
  { value: "password" as const, label: t("networkTools.ssh.authPassword") },
  { value: "key" as const, label: t("networkTools.ssh.authKey") },
]);
const output = computed(() => {
  if (!result.value) return "";
  const stderr = result.value.stderr ? `\n[stderr]\n${result.value.stderr}` : "";
  return `${result.value.stdout}${stderr}`.trim();
});

async function browseKey(): Promise<void> {
  const path = await ipc.pickFile();
  if (path) keyPath.value = path;
}

async function connect(): Promise<void> {
  if (busy.value) return;
  if (!host.value.trim() || !username.value.trim()) {
    error.value = t("networkTools.ssh.hostRequired");
    return;
  }
  busy.value = true;
  error.value = null;
  result.value = null;
  try {
    result.value = await ipc.toolSshExecute({
      host: host.value.trim(),
      port: Math.min(65535, Math.max(1, Number(port.value) || 22)),
      username: username.value.trim(),
      auth: auth.value,
      password: auth.value === "password" ? password.value : undefined,
      keyPath: auth.value === "key" ? keyPath.value.trim() : undefined,
      keyPassphrase: auth.value === "key" ? passphrase.value || undefined : undefined,
      command: command.value,
      timeoutMs: Math.min(60_000, Math.max(2_000, Number(timeoutMs.value) || 15_000)),
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.ssh.title')"
    :description="t('networkTools.tools.ssh.description')"
    :icon="Terminal"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div class="grid gap-2 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-12">
        <div class="space-y-1.5 lg:col-span-5">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.ssh.host") }}</Label>
          <Input
            v-model="host"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            placeholder="192.168.1.10"
            @keydown.enter="connect"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.ssh.port") }}</Label>
          <Input
            v-model="port"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-3">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.ssh.username")
          }}</Label>
          <Input
            v-model="username"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{ t("networkTools.ssh.auth") }}</Label>
          <FormSelect v-model="auth" :options="authOptions" />
        </div>

        <div v-if="auth === 'password'" class="space-y-1.5 lg:col-span-6">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.ssh.password")
          }}</Label>
          <Input
            v-model="password"
            type="password"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            autocomplete="off"
          />
        </div>
        <template v-else>
          <div class="space-y-1.5 lg:col-span-4">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ssh.keyPath")
            }}</Label>
            <div class="flex gap-1.5">
              <Input
                v-model="keyPath"
                class="numeric h-9 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
                placeholder="~/.ssh/id_ed25519"
              />
              <Button
                variant="outline"
                size="sm"
                class="h-9 shrink-0 border-hairline-strong bg-transparent"
                @click="browseKey"
              >
                <FolderOpen class="size-3.5" />
              </Button>
            </div>
          </div>
          <div class="space-y-1.5 lg:col-span-2">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ssh.passphrase")
            }}</Label>
            <Input
              v-model="passphrase"
              type="password"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              autocomplete="off"
            />
          </div>
        </template>

        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.ssh.timeout")
          }}</Label>
          <Input
            v-model="timeoutMs"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-10">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.ssh.command")
          }}</Label>
          <Input
            v-model="command"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            placeholder="uname -a"
            @keydown.enter="connect"
          />
        </div>
        <div class="flex items-end lg:col-span-2">
          <Button
            size="sm"
            class="h-9 w-full gap-1.5 text-[12px]"
            :disabled="busy"
            @click="connect"
          >
            <KeyRound v-if="busy" class="size-3.5 animate-pulse" />
            <Play v-else class="size-3.5" />
            {{ busy ? t("tools.running") : t("networkTools.ssh.connect") }}
          </Button>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="overflow-hidden rounded-xl border border-hairline">
        <div class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2">
          <span
            class="rounded border px-1.5 py-0.5 text-[10px]"
            :class="
              result.exitStatus === 0
                ? 'border-signal/25 bg-signal/10 text-signal'
                : 'border-warn/25 bg-warn/10 text-warn'
            "
          >
            {{ t("networkTools.ssh.authenticated") }}
          </span>
          <span class="numeric text-[11px] text-muted-foreground">
            {{ result.username }}@{{ result.host }}:{{ result.port }} · {{ result.elapsedMs }} ms ·
            exit {{ result.exitStatus ?? "—" }}
          </span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-7 gap-1 text-[11px] text-muted-foreground"
            :disabled="!output"
            @click="copy('output', output)"
          >
            <Check v-if="isCopied('output')" class="size-3 text-signal" />
            <Copy v-else class="size-3" />
            {{ isCopied("output") ? t("common.copiedShort") : t("tools.copy") }}
          </Button>
        </div>
        <pre
          class="scrollbar-thin numeric max-h-[420px] overflow-auto whitespace-pre-wrap break-all bg-well px-3 py-3 text-[11.5px] leading-relaxed"
          >{{ output || "—" }}</pre>
      </div>

      <div
        v-else-if="!busy && !error"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.ssh.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
