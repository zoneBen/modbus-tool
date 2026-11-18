<script setup lang="ts">
import {
  ArrowLeft,
  Cable,
  CheckCircle2,
  Cloud,
  Cpu,
  MonitorCog,
  ScanLine,
  ShieldCheck,
  Sparkles,
} from "@lucide/vue";
import { computed, onBeforeUnmount, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Button } from "@/components/ui/button";

const open = defineModel<boolean>("open", { default: false });
const { t } = useI18n();

const capabilities = computed(() => [
  {
    icon: Cpu,
    title: t("about.capabilities.embeddedTitle"),
    text: t("about.capabilities.embeddedText"),
    tone: "text-signal border-signal/25 bg-signal/10",
  },
  {
    icon: Cable,
    title: t("about.capabilities.busTitle"),
    text: t("about.capabilities.busText"),
    tone: "text-rx border-rx/25 bg-rx/10",
  },
  {
    icon: MonitorCog,
    title: t("about.capabilities.desktopTitle"),
    text: t("about.capabilities.desktopText"),
    tone: "text-warn border-warn/25 bg-warn/10",
  },
  {
    icon: Cloud,
    title: t("about.capabilities.cloudTitle"),
    text: t("about.capabilities.cloudText"),
    tone: "text-sim border-sim/25 bg-sim/10",
  },
]);

const chips = computed(() => [
  t("about.chips.embedded"),
  t("about.chips.modbus"),
  t("about.chips.desktop"),
  t("about.chips.acquisition"),
  t("about.chips.cloud"),
  t("about.chips.xinchuang"),
]);

const steps = computed(() => [
  t("about.steps.requirement"),
  t("about.steps.prototype"),
  t("about.steps.batch"),
  t("about.steps.maintenance"),
]);

const localizationTracks = computed(() => [
  {
    icon: MonitorCog,
    label: t("about.tracks.os"),
    items: [
      t("about.tracks.kylin"),
      t("about.tracks.uos"),
      t("about.tracks.openEuler"),
      t("about.tracks.zhongke"),
    ],
  },
  {
    icon: Cpu,
    label: t("about.tracks.cpu"),
    items: [
      t("about.tracks.loongson"),
      t("about.tracks.phytium"),
      t("about.tracks.kunpeng"),
      t("about.tracks.hygon"),
    ],
  },
  {
    icon: ShieldCheck,
    label: t("about.tracks.delivery"),
    items: [
      t("about.tracks.crossCompile"),
      t("about.tracks.packaging"),
      t("about.tracks.peripheral"),
      t("about.tracks.privateDeploy"),
    ],
  },
]);

function close(): void {
  open.value = false;
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") close();
}

watch(
  open,
  (isOpen) => {
    if (isOpen) window.addEventListener("keydown", onKeydown);
    else window.removeEventListener("keydown", onKeydown);
  },
  { immediate: true },
);

onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="animate-enter fixed inset-0 z-50 overflow-y-auto bg-background text-foreground"
      role="dialog"
      aria-modal="true"
      :aria-label="t('about.dialogLabel')"
    >
      <div
        class="pointer-events-none fixed inset-0 instrument-grid opacity-60"
        aria-hidden="true"
      />
      <div
        class="pointer-events-none fixed -left-40 top-[-18%] size-[520px] rounded-full bg-signal/10 blur-[120px]"
        aria-hidden="true"
      />
      <div
        class="pointer-events-none fixed -right-32 top-[18%] size-[440px] rounded-full bg-rx/10 blur-[120px]"
        aria-hidden="true"
      />
      <div
        class="pointer-events-none fixed bottom-[-20%] left-[35%] size-[420px] rounded-full bg-sim/10 blur-[130px]"
        aria-hidden="true"
      />

      <header class="sticky top-0 z-10 border-b border-hairline bg-background/75 backdrop-blur-xl">
        <div class="mx-auto flex h-14 max-w-6xl items-center gap-3 px-5">
          <Button variant="outline" size="sm" class="h-8 gap-1.5 text-[12px]" @click="close">
            <ArrowLeft class="size-3.5" />
            {{ t("about.back") }}
          </Button>
          <span class="text-[12px] text-muted-foreground">{{ t("about.subtitle") }}</span>
          <span class="numeric ml-auto text-[11px] text-muted-foreground">
            {{ t("app.version") }}
          </span>
        </div>
      </header>

      <main class="relative mx-auto max-w-6xl px-5 py-10 lg:py-14">
        <section class="grid items-center gap-10 lg:grid-cols-[1.25fr_0.75fr]">
          <div>
            <span
              class="inline-flex items-center gap-1.5 rounded-full border border-signal/25 bg-signal/10 px-3 py-1 text-[11px] font-medium text-signal"
            >
              <Sparkles class="size-3" />
              {{ t("about.eyebrow") }}
            </span>
            <h1 class="mt-5 text-[38px] font-semibold leading-[1.08] tracking-tight sm:text-[52px]">
              {{ t("about.titleA") }}
              <span class="text-gradient-signal block">{{ t("about.titleB") }}</span>
            </h1>
            <p class="mt-5 max-w-2xl text-[14px] leading-7 text-muted-foreground">
              {{ t("about.intro") }}
            </p>
            <div class="mt-6 flex flex-wrap items-center gap-2.5">
              <span
                v-for="item in chips"
                :key="item"
                class="rounded-lg border border-hairline bg-overlay-1 px-2.5 py-1.5 text-[11px] text-muted-foreground"
              >
                {{ item }}
              </span>
            </div>
          </div>

          <div class="relative">
            <div
              class="absolute -inset-3 rounded-[28px] bg-gradient-to-br from-signal/20 via-rx/10 to-sim/20 blur-2xl"
              aria-hidden="true"
            />
            <div class="glass relative rounded-3xl p-5 shadow-2xl shadow-black/30">
              <div class="rounded-2xl bg-white p-3 shadow-inner">
                <img
                  src="/wechat_qrcode.png"
                  :alt="t('about.qrAlt')"
                  class="aspect-square w-full rounded-xl object-contain"
                />
              </div>
              <div class="mt-4 text-center">
                <p class="flex items-center justify-center gap-1.5 text-[13px] font-medium">
                  <ScanLine class="size-4 text-signal" />
                  {{ t("about.scanWechat") }}
                </p>
                <p class="mt-1 text-[11px] leading-relaxed text-muted-foreground">
                  {{ t("about.contactHint") }}
                </p>
              </div>
            </div>
          </div>
        </section>

        <section class="mt-14">
          <div class="flex items-end justify-between gap-4">
            <div>
              <p class="label-caps text-signal">Capabilities</p>
              <h2 class="mt-1 text-[22px] font-semibold tracking-tight">
                {{ t("about.capabilitiesTitle") }}
              </h2>
            </div>
            <span class="hidden text-[11px] text-muted-foreground sm:block">
              {{ t("about.capabilitiesSubtitle") }}
            </span>
          </div>
          <div class="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <div
              v-for="item in capabilities"
              :key="item.title"
              class="group rounded-2xl border border-hairline bg-overlay-1 p-4 transition-all duration-300 hover:-translate-y-1 hover:border-hairline-strong hover:bg-overlay-2"
            >
              <span
                class="flex size-10 items-center justify-center rounded-xl border transition-transform duration-300 group-hover:scale-110"
                :class="item.tone"
              >
                <component :is="item.icon" class="size-[18px]" />
              </span>
              <p class="mt-3 text-[13px] font-medium">{{ item.title }}</p>
              <p class="mt-1.5 text-[11px] leading-relaxed text-muted-foreground">
                {{ item.text }}
              </p>
            </div>
          </div>
        </section>

        <section
          class="relative mt-12 overflow-hidden rounded-3xl border border-hairline bg-overlay-1"
        >
          <span
            class="pointer-events-none absolute -top-7 right-2 select-none text-[110px] font-black leading-none tracking-tighter text-signal/[0.05]"
            aria-hidden="true"
          >
            {{ t("about.xinchuangWord") }}
          </span>
          <div class="relative grid lg:grid-cols-[0.88fr_1.12fr]">
            <div class="border-b border-hairline p-6 lg:border-b-0 lg:border-r lg:p-8">
              <span
                class="inline-flex items-center gap-1.5 rounded-full border border-signal/25 bg-signal/10 px-2.5 py-1 text-[10.5px] font-medium text-signal"
              >
                <ShieldCheck class="size-3" />
                {{ t("about.xinchuangLabel") }}
              </span>
              <h2 class="mt-4 text-[24px] font-semibold leading-tight tracking-tight">
                {{ t("about.xinchuangTitleA") }}
                <span class="text-gradient-signal block">{{ t("about.xinchuangTitleB") }}</span>
              </h2>
              <p class="mt-3 text-[12px] leading-6 text-muted-foreground">
                {{ t("about.xinchuangIntro") }}
              </p>
              <p
                class="mt-4 rounded-xl border border-hairline bg-well/50 px-3 py-2.5 text-[10.5px] leading-5 text-muted-foreground"
              >
                {{ t("about.xinchuangNote") }}
              </p>
            </div>
            <div class="grid gap-3 p-5 sm:grid-cols-3 lg:p-6">
              <div
                v-for="track in localizationTracks"
                :key="track.label"
                class="rounded-2xl border border-hairline bg-well/45 p-4 transition-colors duration-300 hover:border-signal/25 hover:bg-overlay-2"
              >
                <p class="flex items-center gap-2 text-[11.5px] font-medium">
                  <span
                    class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
                  >
                    <component :is="track.icon" class="size-3.5" />
                  </span>
                  {{ track.label }}
                </p>
                <div class="mt-3 flex flex-wrap gap-1.5">
                  <span
                    v-for="item in track.items"
                    :key="item"
                    class="rounded-md border border-hairline bg-overlay-1 px-1.5 py-1 text-[10px] leading-none text-muted-foreground"
                  >
                    {{ item }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </section>

        <section class="mt-12 grid gap-4 lg:grid-cols-[1.1fr_0.9fr]">
          <div class="rounded-2xl border border-hairline bg-overlay-1 p-5">
            <p class="label-caps">{{ t("about.deliveryProcess") }}</p>
            <div class="mt-4 grid gap-3 sm:grid-cols-4">
              <div v-for="(step, index) in steps" :key="step" class="relative">
                <span class="numeric text-[11px] text-signal">0{{ index + 1 }}</span>
                <p class="mt-1 text-[12px] font-medium">{{ step }}</p>
                <div
                  v-if="index < steps.length - 1"
                  class="absolute right-2 top-2 hidden h-px w-8 bg-gradient-to-r from-hairline-strong to-transparent sm:block"
                />
              </div>
            </div>
          </div>
          <div
            class="rounded-2xl border border-signal/20 bg-gradient-to-br from-signal/10 via-overlay-1 to-rx/10 p-5"
          >
            <p class="flex items-center gap-1.5 text-[13px] font-medium">
              <CheckCircle2 class="size-4 text-signal" />
              {{ t("about.scenariosTitle") }}
            </p>
            <ul class="mt-3 space-y-2 text-[11.5px] leading-relaxed text-muted-foreground">
              <li>· {{ t("about.scenarios.acquisition") }}</li>
              <li>· {{ t("about.scenarios.tooling") }}</li>
              <li>· {{ t("about.scenarios.hardware") }}</li>
              <li>· {{ t("about.scenarios.retrofit") }}</li>
            </ul>
          </div>
        </section>

        <footer
          class="mt-12 flex flex-col items-center gap-2 border-t border-hairline pt-6 text-center"
        >
          <p class="text-[12px] text-muted-foreground">
            {{ t("about.footerBuild") }}
          </p>
          <p class="text-[11px] text-muted-foreground">
            {{ t("about.footerContact") }}
          </p>
        </footer>
      </main>
    </div>
  </Teleport>
</template>
