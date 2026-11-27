<script setup lang="ts">
import {
  Check,
  Copy,
  Eye,
  EyeOff,
  KeyRound,
  Lock,
  RefreshCw,
  Save,
  Trash2,
  Unlock,
} from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { useToolboxStore } from "@/stores/toolbox";
import { useVaultStore } from "@/stores/vault";

const { t } = useI18n();
const toolbox = useToolboxStore();
const vault = useVaultStore();
const { copy, isCopied } = useCopyFeedback();

const length = toolbox.toolPref("security.password.length", 20);
const count = toolbox.toolPref("security.password.count", 5);
const upper = toolbox.toolPref("security.password.upper", true);
const lower = toolbox.toolPref("security.password.lower", true);
const digits = toolbox.toolPref("security.password.digits", true);
const symbols = toolbox.toolPref("security.password.symbols", true);
const excludeAmbiguous = toolbox.toolPref("security.password.excludeAmbiguous", true);
const generated = ref<string[]>([]);
const saveTarget = ref<string | null>(null);
const entryName = ref("");
const entryUsername = ref("");
const entryUrl = ref("");
const entryNote = ref("");
const unlockPassword = ref("");
const unlockError = ref<string | null>(null);
const revealed = ref<Set<string>>(new Set());

const AMBIGUOUS = /[Il1O0o]/g;
const charsets = computed(() => {
  let value = "";
  if (lower.value) value += "abcdefghijklmnopqrstuvwxyz";
  if (upper.value) value += "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  if (digits.value) value += "0123456789";
  if (symbols.value) value += "!@#$%^&*()-_=+[]{};:,.?/";
  return excludeAmbiguous.value ? value.replace(AMBIGUOUS, "") : value;
});

const entropy = computed(() => {
  if (!charsets.value.length || length.value <= 0) return 0;
  return Math.round(length.value * Math.log2(charsets.value.length));
});
const strength = computed(() => {
  if (entropy.value >= 100) return "strong";
  if (entropy.value >= 70) return "good";
  if (entropy.value >= 45) return "fair";
  return "weak";
});
const strengthWidth = computed(() => `${Math.min(100, Math.round((entropy.value / 120) * 100))}%`);

function pick(): string {
  const pool = charsets.value;
  if (!pool) return "";
  const target = Math.min(256, Math.max(4, Number(length.value) || 16));
  const values = new Uint32Array(target);
  crypto.getRandomValues(values);
  let result = Array.from(values, (value) => pool[value % pool.length]).join("");
  // Ensure at least one character from every selected class.
  const required: string[] = [];
  if (lower.value) required.push("abcdefghijklmnopqrstuvwxyz");
  if (upper.value) required.push("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
  if (digits.value) required.push("0123456789");
  if (symbols.value) required.push("!@#$%^&*()-_=+[]{};:,.?/");
  required.forEach((set, index) => {
    const pool = excludeAmbiguous.value ? set.replace(AMBIGUOUS, "") : set;
    const position = (values[index] ?? index) % result.length;
    const char = pool[(values[index + 7] ?? index) % pool.length];
    result = result.slice(0, position) + char + result.slice(position + 1);
  });
  return result;
}

function generate(): void {
  const amount = Math.min(50, Math.max(1, Number(count.value) || 5));
  generated.value = Array.from({ length: amount }, pick).filter(Boolean);
}

async function unlock(): Promise<void> {
  unlockError.value = null;
  const ok = await vault.unlock(unlockPassword.value);
  if (!ok) {
    unlockError.value = t("securityTools.password.wrongPassword");
    return;
  }
  unlockPassword.value = "";
}

function startSave(value: string): void {
  saveTarget.value = value;
  entryName.value = "";
  entryUsername.value = "";
  entryUrl.value = "";
  entryNote.value = "";
}

async function confirmSave(): Promise<void> {
  if (!saveTarget.value) return;
  try {
    await vault.addEntry({
      name: entryName.value.trim() || t("securityTools.password.unnamed"),
      username: entryUsername.value.trim(),
      password: saveTarget.value,
      url: entryUrl.value.trim(),
      note: entryNote.value.trim(),
    });
    toast.success(t("securityTools.password.saved"));
    saveTarget.value = null;
  } catch (error: unknown) {
    toast.error(
      error instanceof Error ? t(`securityTools.password.${error.message}`) : String(error),
    );
  }
}

function toggleReveal(id: string): void {
  const next = new Set(revealed.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  revealed.value = next;
}

generate();
</script>

<template>
  <ToolPanelFrame
    :title="t('securityTools.tools.password.title')"
    :description="t('securityTools.tools.password.description')"
    :icon="KeyRound"
  >
    <div class="mx-auto max-w-[1040px] space-y-4">
      <div class="grid gap-3 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-12">
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("securityTools.password.length")
          }}</Label>
          <Input
            v-model.number="length"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("securityTools.password.count")
          }}</Label>
          <Input
            v-model.number="count"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="flex flex-wrap items-center gap-4 lg:col-span-6">
          <label class="flex items-center gap-2 text-[11px]"><Switch v-model="upper" /> A-Z</label>
          <label class="flex items-center gap-2 text-[11px]"><Switch v-model="lower" /> a-z</label>
          <label class="flex items-center gap-2 text-[11px]"><Switch v-model="digits" /> 0-9</label>
          <label class="flex items-center gap-2 text-[11px]"
            ><Switch v-model="symbols" /> !@#</label
          >
          <label class="flex items-center gap-2 text-[11px]">
            <Switch v-model="excludeAmbiguous" />
            {{ t("securityTools.password.excludeAmbiguous") }}
          </label>
        </div>
        <div class="flex items-end lg:col-span-2">
          <Button
            size="sm"
            class="h-9 w-full gap-1.5 text-[11px]"
            :disabled="!charsets"
            @click="generate"
          >
            <RefreshCw class="size-3.5" />
            {{ t("securityTools.password.generate") }}
          </Button>
        </div>

        <div class="lg:col-span-12">
          <div class="mb-1 flex items-center justify-between text-[10px] text-muted-foreground">
            <span>{{ t("securityTools.password.entropy", { bits: entropy }) }}</span>
            <span
              :class="
                strength === 'weak'
                  ? 'text-error'
                  : strength === 'fair'
                    ? 'text-warn'
                    : 'text-signal'
              "
            >
              {{ t(`securityTools.password.strength.${strength}`) }}
            </span>
          </div>
          <div class="h-1.5 overflow-hidden rounded-full bg-overlay-2">
            <div
              class="h-full rounded-full transition-all"
              :class="
                strength === 'weak' ? 'bg-error' : strength === 'fair' ? 'bg-warn' : 'bg-signal'
              "
              :style="{ width: strengthWidth }"
            />
          </div>
        </div>
      </div>

      <div class="rounded-xl border border-hairline">
        <div class="flex items-center gap-2 border-b border-hairline px-3 py-2">
          <span class="text-[11px] font-medium">{{ t("securityTools.password.generated") }}</span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
            :disabled="!generated.length"
            @click="copy('all', generated.join('\n'))"
          >
            <Check v-if="isCopied('all')" class="size-3 text-signal" />
            <Copy v-else class="size-3" />
            {{ isCopied("all") ? t("common.copiedShort") : t("common.copyAll") }}
          </Button>
        </div>
        <div class="divide-y divide-hairline/60">
          <div
            v-for="(value, index) in generated"
            :key="`${value}-${index}`"
            class="flex items-center gap-2 px-3 py-2"
          >
            <span class="numeric w-5 text-[9px] text-muted-foreground">{{ index + 1 }}</span>
            <span class="numeric min-w-0 flex-1 break-all text-[13px] tracking-wide">{{
              value
            }}</span>
            <button
              type="button"
              class="flex items-center gap-1 text-muted-foreground hover:text-foreground"
              @click="copy(`gen:${index}`, value)"
            >
              <Check v-if="isCopied(`gen:${index}`)" class="size-3.5 text-signal" />
              <Copy v-else class="size-3.5" />
              <span v-if="isCopied(`gen:${index}`)" class="text-[9px] text-signal">{{
                t("common.copiedShort")
              }}</span>
            </button>
            <button
              type="button"
              class="text-muted-foreground hover:text-signal"
              :title="t('securityTools.password.save')"
              @click="startSave(value)"
            >
              <Save class="size-3.5" />
            </button>
          </div>
          <p
            v-if="!generated.length"
            class="px-3 py-6 text-center text-[11px] text-muted-foreground"
          >
            {{ t("securityTools.password.noCharset") }}
          </p>
        </div>
      </div>

      <div
        v-if="saveTarget"
        class="space-y-2 rounded-xl border border-signal/25 bg-signal/[0.05] p-3"
      >
        <div class="flex items-center gap-2">
          <Save class="size-3.5 text-signal" />
          <p class="text-[11px] font-medium">{{ t("securityTools.password.saveTitle") }}</p>
          <span class="numeric ml-auto text-[10px] text-muted-foreground">{{ saveTarget }}</span>
        </div>
        <div class="grid gap-2 sm:grid-cols-2">
          <Input
            v-model="entryName"
            class="h-8 rounded-md border-hairline-strong bg-well text-[11px]"
            :placeholder="t('securityTools.password.name')"
          />
          <Input
            v-model="entryUsername"
            class="h-8 rounded-md border-hairline-strong bg-well text-[11px]"
            :placeholder="t('securityTools.password.username')"
          />
          <Input
            v-model="entryUrl"
            class="h-8 rounded-md border-hairline-strong bg-well text-[11px]"
            :placeholder="t('securityTools.password.url')"
          />
          <Input
            v-model="entryNote"
            class="h-8 rounded-md border-hairline-strong bg-well text-[11px]"
            :placeholder="t('securityTools.password.note')"
          />
        </div>
        <div class="flex items-center gap-2">
          <p v-if="!vault.configured" class="text-[10.5px] text-warn">
            {{ t("securityTools.password.setMasterFirst") }}
          </p>
          <p v-else-if="!vault.unlocked" class="text-[10.5px] text-warn">
            {{ t("securityTools.password.unlockFirst") }}
          </p>
          <Button
            size="sm"
            class="ml-auto h-8 text-[11px]"
            :disabled="!vault.unlocked"
            @click="confirmSave"
          >
            {{ t("securityTools.password.save") }}
          </Button>
          <Button variant="ghost" size="sm" class="h-8 text-[11px]" @click="saveTarget = null">
            {{ t("common.cancel") }}
          </Button>
        </div>
      </div>

      <div class="rounded-xl border border-hairline">
        <div class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2">
          <Lock class="size-3.5 text-muted-foreground" />
          <span class="text-[11px] font-medium">{{ t("securityTools.password.vault") }}</span>
          <span
            class="rounded border border-hairline px-1.5 py-0.5 text-[9px] text-muted-foreground"
          >
            {{ vault.entries.length }}
          </span>
          <template v-if="vault.configured && !vault.unlocked">
            <Input
              v-model="unlockPassword"
              type="password"
              class="ml-auto h-7 w-40 rounded-md border-hairline-strong bg-well text-[11px]"
              :placeholder="t('securityTools.password.masterPassword')"
              @keydown.enter="unlock"
            />
            <Button size="sm" class="h-7 gap-1 text-[10.5px]" @click="unlock">
              <Unlock class="size-3" />
              {{ t("securityTools.password.unlock") }}
            </Button>
          </template>
          <Button
            v-else-if="vault.unlocked"
            variant="ghost"
            size="sm"
            class="ml-auto h-7 gap-1 text-[10.5px] text-muted-foreground"
            @click="vault.lock()"
          >
            <Lock class="size-3" />
            {{ t("securityTools.password.lock") }}
          </Button>
        </div>

        <p
          v-if="unlockError"
          class="border-b border-error/20 bg-error/10 px-3 py-1.5 text-[10.5px] text-error"
        >
          {{ unlockError }}
        </p>
        <p v-if="!vault.configured" class="px-3 py-6 text-center text-[11px] text-muted-foreground">
          {{ t("securityTools.password.notConfigured") }}
        </p>
        <div
          v-else-if="!vault.unlocked"
          class="px-3 py-6 text-center text-[11px] text-muted-foreground"
        >
          {{ t("securityTools.password.lockedHint") }}
        </div>
        <div v-else class="divide-y divide-hairline/60">
          <div
            v-for="entry in vault.entries"
            :key="entry.id"
            class="flex items-center gap-2 px-3 py-2"
          >
            <div class="min-w-0 flex-1">
              <p class="truncate text-[11.5px] font-medium">{{ entry.name }}</p>
              <p class="truncate text-[10px] text-muted-foreground">
                {{ entry.username || "—" }}
                <span v-if="entry.url"> · {{ entry.url }}</span>
              </p>
            </div>
            <span class="numeric text-[11px]">
              {{ revealed.has(entry.id) ? entry.password : "••••••••••" }}
            </span>
            <button
              type="button"
              class="text-muted-foreground hover:text-foreground"
              @click="toggleReveal(entry.id)"
            >
              <EyeOff v-if="revealed.has(entry.id)" class="size-3.5" />
              <Eye v-else class="size-3.5" />
            </button>
            <button
              type="button"
              class="flex items-center gap-1 text-muted-foreground hover:text-foreground"
              @click="copy(`vault:${entry.id}`, entry.password)"
            >
              <Check v-if="isCopied(`vault:${entry.id}`)" class="size-3.5 text-signal" />
              <Copy v-else class="size-3.5" />
              <span v-if="isCopied(`vault:${entry.id}`)" class="text-[9px] text-signal">{{
                t("common.copiedShort")
              }}</span>
            </button>
            <button
              type="button"
              class="text-muted-foreground hover:text-error"
              @click="vault.removeEntry(entry.id)"
            >
              <Trash2 class="size-3.5" />
            </button>
          </div>
          <p
            v-if="!vault.entries.length"
            class="px-3 py-6 text-center text-[11px] text-muted-foreground"
          >
            {{ t("securityTools.password.vaultEmpty") }}
          </p>
        </div>
      </div>

      <p class="text-[10px] leading-relaxed text-muted-foreground">
        {{ t("securityTools.password.storageNote") }}
      </p>
    </div>
  </ToolPanelFrame>
</template>
