<script setup lang="ts">
import { Check, Copy, Dices, RefreshCw } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  v1 as uuidv1,
  v3 as uuidv3,
  v4 as uuidv4,
  v5 as uuidv5,
  validate as validateUuid,
} from "uuid";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { useToolboxStore } from "@/stores/toolbox";

type UuidVersion = "1" | "3" | "4" | "5";
const NAMESPACES = {
  DNS: "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  URL: "6ba7b811-9dad-11d1-80b4-00c04fd430c8",
  OID: "6ba7b812-9dad-11d1-80b4-00c04fd430c8",
  X500: "6ba7b814-9dad-11d1-80b4-00c04fd430c8",
};

const { t } = useI18n();
const toolbox = useToolboxStore();
const { copy, isCopied } = useCopyFeedback();
const version = toolbox.toolPref<UuidVersion>("dev.uuid.version", "4");
const namespace = toolbox.toolPref<keyof typeof NAMESPACES>("dev.uuid.namespace", "DNS");
const name = toolbox.toolPref("dev.uuid.name", "example.com");
const count = toolbox.toolPref("dev.uuid.count", "8");
const randomLength = toolbox.toolPref("dev.uuid.randomLength", "16");
const randomKind = toolbox.toolPref<"alnum" | "hex" | "numeric" | "symbol">(
  "dev.uuid.randomKind",
  "alnum",
);
const uuids = ref<string[]>([]);
const randomValues = ref<string[]>([]);

const versionOptions = [
  { value: "1" as const, label: "UUID v1 (time)" },
  { value: "3" as const, label: "UUID v3 (MD5)" },
  { value: "4" as const, label: "UUID v4 (random)" },
  { value: "5" as const, label: "UUID v5 (SHA-1)" },
];
const namespaceOptions = (Object.keys(NAMESPACES) as Array<keyof typeof NAMESPACES>).map((key) => ({
  value: key,
  label: key,
}));
const kindOptions = computed(() => [
  { value: "alnum" as const, label: t("devTools.uuid.kinds.alnum") },
  { value: "hex" as const, label: t("devTools.uuid.kinds.hex") },
  { value: "numeric" as const, label: t("devTools.uuid.kinds.numeric") },
  { value: "symbol" as const, label: t("devTools.uuid.kinds.symbol") },
]);

function generateUuid(): string {
  if (version.value === "1") return uuidv1();
  if (version.value === "3") return uuidv3(name.value, NAMESPACES[namespace.value]);
  if (version.value === "5") return uuidv5(name.value, NAMESPACES[namespace.value]);
  return uuidv4();
}

function generate(): void {
  const amount = Math.min(200, Math.max(1, Number(count.value) || 8));
  uuids.value = Array.from({ length: amount }, generateUuid);
}

const charset = computed(() => {
  if (randomKind.value === "hex") return "0123456789abcdef";
  if (randomKind.value === "numeric") return "0123456789";
  if (randomKind.value === "symbol") return "!@#$%^&*()-_=+[]{};:,.?/";
  return "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
});

function randomString(length: number): string {
  const values = new Uint32Array(length);
  crypto.getRandomValues(values);
  return Array.from(values, (value) => charset.value[value % charset.value.length]).join("");
}

function generateRandom(): void {
  const amount = Math.min(200, Math.max(1, Number(count.value) || 8));
  const length = Math.min(256, Math.max(1, Number(randomLength.value) || 16));
  randomValues.value = Array.from({ length: amount }, () => randomString(length));
}

const uuidValid = computed(() => uuids.value.map((value) => validateUuid(value)));

generate();
generateRandom();
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.uuid.title')"
    :description="t('devTools.tools.uuid.description')"
    :icon="Dices"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div class="grid gap-2 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-12">
        <div class="space-y-1.5 lg:col-span-3">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.uuid.version") }}</Label>
          <FormSelect v-model="version" :options="versionOptions" />
        </div>
        <div v-if="version === '3' || version === '5'" class="space-y-1.5 lg:col-span-3">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.uuid.namespace")
          }}</Label>
          <FormSelect v-model="namespace" :options="namespaceOptions" />
        </div>
        <div v-if="version === '3' || version === '5'" class="space-y-1.5 lg:col-span-4">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.uuid.name") }}</Label>
          <Input
            v-model="name"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.uuid.count") }}</Label>
          <Input
            v-model="count"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="flex items-end lg:col-span-2">
          <Button size="sm" class="h-9 w-full gap-1 text-[11px]" @click="generate">
            <RefreshCw class="size-3.5" />
            {{ t("devTools.uuid.generate") }}
          </Button>
        </div>
      </div>

      <div class="rounded-xl border border-hairline">
        <div class="flex items-center gap-2 border-b border-hairline px-3 py-2">
          <span class="text-[11px] font-medium">UUID</span>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
            @click="copy('uuids', uuids.join('\n'))"
          >
            <Check v-if="isCopied('uuids')" class="size-3 text-signal" />
            <Copy v-else class="size-3" />
            {{ isCopied("uuids") ? t("common.copiedShort") : t("common.copyAll") }}
          </Button>
        </div>
        <div class="divide-y divide-hairline/60">
          <div
            v-for="(value, index) in uuids"
            :key="value"
            class="flex items-center gap-2 px-3 py-1.5"
          >
            <span class="numeric w-6 text-[9px] text-muted-foreground">{{ index + 1 }}</span>
            <span
              class="numeric min-w-0 flex-1 break-all text-[11.5px]"
              :class="uuidValid[index] ? '' : 'text-error'"
            >
              {{ value }}
            </span>
            <button
              type="button"
              class="flex items-center gap-1 text-muted-foreground hover:text-foreground"
              @click="copy(`uuid:${index}`, value)"
            >
              <Check v-if="isCopied(`uuid:${index}`)" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              <span v-if="isCopied(`uuid:${index}`)" class="text-[9px] text-signal">{{
                t("common.copiedShort")
              }}</span>
            </button>
          </div>
        </div>
      </div>

      <div class="grid gap-2 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-12">
        <div class="space-y-1.5 lg:col-span-3">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.uuid.randomKind")
          }}</Label>
          <FormSelect v-model="randomKind" :options="kindOptions" />
        </div>
        <div class="space-y-1.5 lg:col-span-2">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.uuid.randomLength")
          }}</Label>
          <Input
            v-model="randomLength"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="flex items-end lg:col-span-2">
          <Button
            variant="outline"
            size="sm"
            class="h-9 w-full gap-1 border-hairline-strong bg-transparent text-[11px]"
            @click="generateRandom"
          >
            <Dices class="size-3.5" />
            {{ t("devTools.uuid.random") }}
          </Button>
        </div>
        <div class="lg:col-span-5 flex items-center justify-end">
          <Button
            variant="ghost"
            size="sm"
            class="h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
            @click="copy('random', randomValues.join('\n'))"
          >
            <Check v-if="isCopied('random')" class="size-3 text-signal" />
            <Copy v-else class="size-3" />
            {{ isCopied("random") ? t("common.copiedShort") : t("common.copyAll") }}
          </Button>
        </div>
        <div class="lg:col-span-12 grid gap-1 sm:grid-cols-2">
          <div
            v-for="(value, index) in randomValues"
            :key="`${value}-${index}`"
            class="numeric flex items-center gap-2 rounded-md bg-overlay-1 px-2 py-1 text-[11px]"
          >
            <span class="min-w-0 flex-1 break-all">{{ value }}</span>
            <button
              type="button"
              class="flex items-center gap-1 text-muted-foreground hover:text-foreground"
              @click="copy(`rand:${index}`, value)"
            >
              <Check v-if="isCopied(`rand:${index}`)" class="size-3 text-signal" />
              <Copy v-else class="size-3" />
              <span v-if="isCopied(`rand:${index}`)" class="text-[9px] text-signal">{{
                t("common.copiedShort")
              }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
