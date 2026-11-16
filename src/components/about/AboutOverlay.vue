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
import { onBeforeUnmount, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Button } from "@/components/ui/button";

const open = defineModel<boolean>("open", { default: false });
const { t } = useI18n();

const capabilities = [
  {
    icon: Cpu,
    title: "嵌入式 / 固件",
    text: "STM32 / ESP32 / C 与 C++，RTOS 与裸机驱动，通信协议栈与低功耗设计。",
    tone: "text-signal border-signal/25 bg-signal/10",
  },
  {
    icon: Cable,
    title: "工业总线 / 硬件",
    text: "Modbus RTU / TCP / ASCII、RS-485、CAN、采集板卡与传感器接入。",
    tone: "text-rx border-rx/25 bg-rx/10",
  },
  {
    icon: MonitorCog,
    title: "上位机 / 桌面端",
    text: "Rust + Tauri、Vue、C# / Qt；设备调试、数据采集、实时曲线与报表。",
    tone: "text-warn border-warn/25 bg-warn/10",
  },
  {
    icon: Cloud,
    title: "云平台 / 可视化",
    text: "设备接入、数据看板、远程运维与移动端配套，打通端到端链路。",
    tone: "text-sim border-sim/25 bg-sim/10",
  },
];

const steps = ["需求梳理", "原型验证", "小批量交付", "长期维护"];

const localizationTracks = [
  {
    icon: MonitorCog,
    label: "国产操作系统",
    items: ["麒麟软件", "统信 UOS", "openEuler", "中科方德"],
  },
  {
    icon: Cpu,
    label: "国产 CPU 架构",
    items: ["龙芯 LoongArch", "飞腾 ARM64", "鲲鹏 ARM64", "海光 / 兆芯 x86_64"],
  },
  {
    icon: ShieldCheck,
    label: "信创交付能力",
    items: ["交叉编译", "国产化打包", "外设与总线联调", "私有化部署"],
  },
];

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
      aria-label="关于 Modbus Tool 与作者"
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
            返回
          </Button>
          <span class="text-[12px] text-muted-foreground">关于 · About</span>
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
              独立开发者 · 软硬件全栈交付
            </span>
            <h1 class="mt-5 text-[38px] font-semibold leading-[1.08] tracking-tight sm:text-[52px]">
              从芯片到云端
              <span class="text-gradient-signal block">一站式软硬件定制</span>
            </h1>
            <p class="mt-5 max-w-2xl text-[14px] leading-7 text-muted-foreground">
              提供嵌入式固件、工业总线、上位机 / 桌面软件、云平台与数据可视化的定制开发。 从一个
              Modbus 采集模块，到完整的设备调试与远程运维系统，按需求交付可落地的方案。
            </p>
            <div class="mt-6 flex flex-wrap items-center gap-2.5">
              <span
                v-for="item in [
                  '嵌入式固件',
                  'Modbus / RS-485',
                  'Tauri 桌面端',
                  '数据采集与曲线',
                  '云平台',
                  '国产信创适配',
                ]"
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
                  alt="作者微信二维码"
                  class="aspect-square w-full rounded-xl object-contain"
                />
              </div>
              <div class="mt-4 text-center">
                <p class="flex items-center justify-center gap-1.5 text-[13px] font-medium">
                  <ScanLine class="size-4 text-signal" />
                  扫码添加微信
                </p>
                <p class="mt-1 text-[11px] leading-relaxed text-muted-foreground">
                  沟通需求、评估方案、获取报价
                </p>
              </div>
            </div>
          </div>
        </section>

        <section class="mt-14">
          <div class="flex items-end justify-between gap-4">
            <div>
              <p class="label-caps text-signal">Capabilities</p>
              <h2 class="mt-1 text-[22px] font-semibold tracking-tight">能做什么</h2>
            </div>
            <span class="hidden text-[11px] text-muted-foreground sm:block">
              软硬件一体设计与交付
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
            信创
          </span>
          <div class="relative grid lg:grid-cols-[0.88fr_1.12fr]">
            <div class="border-b border-hairline p-6 lg:border-b-0 lg:border-r lg:p-8">
              <span
                class="inline-flex items-center gap-1.5 rounded-full border border-signal/25 bg-signal/10 px-2.5 py-1 text-[10.5px] font-medium text-signal"
              >
                <ShieldCheck class="size-3" />
                国产化 · 信创适配
              </span>
              <h2 class="mt-4 text-[24px] font-semibold leading-tight tracking-tight">
                面向国产软硬件环境
                <span class="text-gradient-signal block">交付与迁移</span>
              </h2>
              <p class="mt-3 text-[12px] leading-6 text-muted-foreground">
                可配合信创项目完成桌面端、设备端与数据链路的适配、打包和现场联调，让现有采集与调试能力平滑迁移到国产化环境。
              </p>
              <p
                class="mt-4 rounded-xl border border-hairline bg-well/50 px-3 py-2.5 text-[10.5px] leading-5 text-muted-foreground"
              >
                当前官方安装包为 Windows x64。麒麟、统信 UOS 及国产 CPU
                环境按项目需求提供适配构建与联调支持，具体平台和版本以评估结果为准。
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
            <p class="label-caps">交付流程</p>
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
              适合这些场景
            </p>
            <ul class="mt-3 space-y-2 text-[11.5px] leading-relaxed text-muted-foreground">
              <li>· 工业设备数据采集与 Modbus 协议对接</li>
              <li>· 调试工具、测试工装与产线软件定制</li>
              <li>· 硬件选型、固件开发与上位机配套</li>
              <li>· 现有设备联网、可视化与远程运维改造</li>
            </ul>
          </div>
        </section>

        <footer
          class="mt-12 flex flex-col items-center gap-2 border-t border-hairline pt-6 text-center"
        >
          <p class="text-[12px] text-muted-foreground">
            用 Rust + Tauri 构建的轻量 Modbus 调试工具
          </p>
          <p class="text-[11px] text-muted-foreground">
            软硬件定制 · 国产信创适配 · 微信扫码联系作者
          </p>
        </footer>
      </main>
    </div>
  </Teleport>
</template>
