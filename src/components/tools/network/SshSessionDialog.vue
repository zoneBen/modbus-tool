<script setup lang="ts">
import { FolderOpen, KeyRound, LockKeyhole, Server } from "@lucide/vue";
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ipc } from "@/lib/ipc";
import { useSshStore } from "@/stores/ssh";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{ profileId: string | null }>();
const emit = defineEmits<{ (event: "connect", profileId: string): void }>();

const { t } = useI18n();
const ssh = useSshStore();
const saving = ref(false);
const error = ref<string | null>(null);

const draft = reactive({
  name: "",
  host: "",
  port: "22",
  username: "root",
  auth: "password" as "password" | "key",
  keyPath: "",
});
const password = ref("");
const passphrase = ref("");

const authOptions = computed(() => [
  { value: "password" as const, label: t("networkTools.ssh.authPassword") },
  { value: "key" as const, label: t("networkTools.ssh.authKey") },
]);

watch(
  () => [open.value, props.profileId] as const,
  ([isOpen]) => {
    if (!isOpen) return;
    const profile = ssh.getProfile(props.profileId);
    error.value = null;
    saving.value = false;
    if (!profile) return;
    draft.name = profile.name;
    draft.host = profile.host;
    draft.port = String(profile.port);
    draft.username = profile.username;
    draft.auth = profile.auth;
    draft.keyPath = profile.keyPath;
    password.value = ssh.passwords[profile.id] ?? "";
    passphrase.value = ssh.passphrases[profile.id] ?? "";
  },
  { immediate: true },
);

async function browseKey(): Promise<void> {
  const path = await ipc.pickFile();
  if (path) draft.keyPath = path;
}

function persist(): string | null {
  const profile = ssh.getProfile(props.profileId);
  if (!profile) return null;
  ssh.updateProfile(profile.id, {
    name: draft.name.trim() || profile.name,
    host: draft.host.trim(),
    port: Math.min(65_535, Math.max(1, Number(draft.port) || 22)),
    username: draft.username.trim() || "root",
    auth: draft.auth,
    keyPath: draft.keyPath.trim(),
  });
  ssh.passwords[profile.id] = password.value;
  ssh.passphrases[profile.id] = passphrase.value;
  return profile.id;
}

function save(connect: boolean): void {
  if (!props.profileId) return;
  if (connect && (!draft.host.trim() || !draft.username.trim())) {
    error.value = t("networkTools.ssh.hostRequired");
    return;
  }
  if (connect && draft.auth === "key" && !draft.keyPath.trim()) {
    error.value = t("networkTools.ssh.keyRequired");
    return;
  }
  const id = persist();
  if (!id) return;
  open.value = false;
  if (connect) emit("connect", id);
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[560px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-rx/25 bg-rx/10 text-rx"
          >
            <Server class="size-3.5" />
          </span>
          {{ t("networkTools.ssh.dialogTitle") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ t("networkTools.ssh.dialogHint") }}
        </DialogDescription>
      </DialogHeader>

      <div class="scrollbar-thin max-h-[68vh] space-y-3 overflow-y-auto px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ssh.name")
            }}</Label>
            <Input
              v-model="draft.name"
              class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ssh.host")
            }}</Label>
            <Input
              v-model="draft.host"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              placeholder="192.168.1.10"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ssh.port")
            }}</Label>
            <Input
              v-model="draft.port"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ssh.username")
            }}</Label>
            <Input
              v-model="draft.username"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ssh.auth")
            }}</Label>
            <FormSelect v-model="draft.auth" :options="authOptions" />
          </div>
        </div>

        <div v-if="draft.auth === 'password'" class="space-y-1.5">
          <Label class="flex items-center gap-1 text-[11px] text-muted-foreground">
            <LockKeyhole class="size-3" />
            {{ t("networkTools.ssh.password") }}
          </Label>
          <Input
            v-model="password"
            type="password"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            autocomplete="off"
          />
          <p class="text-[10px] text-muted-foreground">{{ t("networkTools.ssh.passwordNote") }}</p>
        </div>

        <template v-else>
          <div class="space-y-1.5">
            <Label class="flex items-center gap-1 text-[11px] text-muted-foreground">
              <KeyRound class="size-3" />
              {{ t("networkTools.ssh.keyPath") }}
            </Label>
            <div class="flex gap-1.5">
              <Input
                v-model="draft.keyPath"
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
          <div class="space-y-1.5">
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

        <p
          v-if="error"
          class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
        >
          {{ error }}
        </p>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <Button variant="ghost" size="sm" class="h-8 text-[12px]" @click="open = false">
          {{ t("common.cancel") }}
        </Button>
        <Button
          variant="outline"
          size="sm"
          class="h-8 text-[12px]"
          :disabled="saving"
          @click="save(false)"
        >
          {{ t("networkTools.ssh.save") }}
        </Button>
        <Button size="sm" class="h-8 text-[12px]" :disabled="saving" @click="save(true)">
          {{ t("networkTools.ssh.saveConnect") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
