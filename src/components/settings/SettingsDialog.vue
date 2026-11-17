<script setup lang="ts">
import { Languages, Settings2 } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
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
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";

const open = defineModel<boolean>("open", { default: false });
const settings = useSettingsStore();
const sessions = useSessionsStore();
const { t } = useI18n();
const addressBases = [0, 1] as const;

const themeOptions = computed(() => [
  { value: "dark" as const, label: t("dialog.themeDark") },
  { value: "light" as const, label: t("dialog.themeLight") },
  { value: "system" as const, label: t("dialog.themeSystem") },
]);

const languageOptions = computed(() => [
  { value: "zh-CN" as const, label: t("dialog.languageZh") },
  { value: "en-US" as const, label: t("dialog.languageEn") },
]);

const toggles = computed(() => [
  {
    key: "reduceEffects" as const,
    title: t("dialog.reduceEffects"),
    hint: t("dialog.reduceEffectsHint"),
  },
  {
    key: "compactRows" as const,
    title: t("dialog.compactRows"),
    hint: t("dialog.compactRowsHint"),
  },
  {
    key: "monitorFollow" as const,
    title: t("dialog.monitorFollow"),
    hint: t("dialog.monitorFollowHint"),
  },
  {
    key: "showRawColumn" as const,
    title: t("dialog.showRaw"),
    hint: t("dialog.showRawHint"),
  },
]);
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[460px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-hairline-strong bg-overlay-2"
          >
            <Settings2 class="size-3.5 text-muted-foreground" />
          </span>
          {{ t("dialog.settingsTitle") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ t("dialog.settingsHint") }}
        </DialogDescription>
      </DialogHeader>

      <div class="px-5 py-2">
        <div class="flex items-center justify-between gap-4 py-3">
          <div>
            <p class="text-[12px] font-medium text-foreground">{{ t("dialog.theme") }}</p>
            <p class="mt-0.5 text-[11px] text-muted-foreground">
              {{ t("dialog.themeHint") }}
            </p>
          </div>
          <div class="flex rounded-lg border border-hairline-strong bg-well p-0.5">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              type="button"
              class="rounded-md px-2 py-1 text-[11px] transition-colors"
              :class="
                settings.theme === option.value
                  ? 'bg-overlay-3 text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              "
              :aria-pressed="settings.theme === option.value"
              @click="settings.setTheme(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
        </div>

        <div class="flex items-center justify-between gap-4 border-t border-hairline py-3">
          <div class="min-w-0">
            <p class="flex items-center gap-1.5 text-[12px] font-medium text-foreground">
              <Languages class="size-3 text-muted-foreground" />
              {{ t("dialog.language") }}
            </p>
            <p class="mt-0.5 text-[11px] leading-relaxed text-muted-foreground">
              {{ t("dialog.languageHint") }}
            </p>
            <p v-if="settings.locale === 'en-US'" class="mt-1 text-[11px] text-warn">
              {{ t("dialog.languagePreview") }}
            </p>
          </div>
          <div class="flex shrink-0 rounded-lg border border-hairline-strong bg-well p-0.5">
            <button
              v-for="option in languageOptions"
              :key="option.value"
              type="button"
              class="rounded-md px-2 py-1 text-[11px] transition-colors"
              :class="
                settings.locale === option.value
                  ? 'bg-overlay-3 text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              "
              :aria-pressed="settings.locale === option.value"
              @click="settings.setLocale(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
        </div>

        <div
          v-for="item in toggles"
          :key="item.key"
          class="flex items-center justify-between gap-4 border-t border-hairline py-3"
        >
          <div>
            <p class="text-[12px] font-medium text-foreground">{{ item.title }}</p>
            <p class="mt-0.5 text-[11px] leading-relaxed text-muted-foreground">
              {{ item.hint }}
            </p>
          </div>
          <Switch
            :model-value="settings[item.key]"
            @update:model-value="(checked) => (settings[item.key] = Boolean(checked))"
          />
        </div>

        <div class="flex items-center justify-between gap-4 border-t border-hairline py-3">
          <div>
            <p class="text-[12px] font-medium text-foreground">{{ t("dialog.addressBase") }}</p>
            <p class="mt-0.5 text-[11px] text-muted-foreground">
              {{ t("dialog.addressBaseHint") }}
            </p>
          </div>
          <div class="flex rounded-lg border border-hairline-strong bg-well p-0.5">
            <button
              v-for="base in addressBases"
              :key="base"
              type="button"
              class="numeric w-7 rounded-md py-1 text-[11px] transition-colors"
              :class="
                settings.addressBase === base
                  ? 'bg-overlay-3 text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="settings.addressBase = base"
            >
              {{ base }}
            </button>
          </div>
        </div>
      </div>

      <Separator class="bg-overlay-2" />

      <div class="flex items-center justify-between px-5 py-3 text-[11px] text-muted-foreground">
        <div class="flex items-center gap-2">
          <span class="font-medium text-foreground">Modbus Tool</span>
          <span class="numeric">v0.2.0 · Preview</span>
        </div>
        <span
          v-if="sessions.demo"
          class="rounded-full border border-sim/25 bg-sim/10 px-1.5 py-0.5 text-[11px] text-sim"
        >
          DEMO RUNTIME
        </span>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button size="sm" class="h-8 text-[12px]">{{ t("common.done") }}</Button>
        </DialogClose>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
