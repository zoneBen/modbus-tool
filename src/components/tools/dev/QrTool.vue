<script setup lang="ts">
import { Check, Copy, Download, QrCode, Upload } from "@lucide/vue";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCopyFeedback } from "@/composables/useCopyFeedback";

const { t } = useI18n();
const { copy, isCopied } = useCopyFeedback();
const text = ref("https://example.com");
const size = ref("320");
const level = ref<"L" | "M" | "Q" | "H">("M");
const dark = ref("#0b1220");
const light = ref("#ffffff");
const imageUrl = ref("");
const error = ref<string | null>(null);
const decoded = ref<string | null>(null);
const decodeError = ref<string | null>(null);

const levelOptions = [
  { value: "L" as const, label: "L (7%)" },
  { value: "M" as const, label: "M (15%)" },
  { value: "Q" as const, label: "Q (25%)" },
  { value: "H" as const, label: "H (30%)" },
];

async function generate(): Promise<void> {
  if (!text.value) {
    error.value = t("devTools.qr.textRequired");
    return;
  }
  try {
    const QRCode = (await import("qrcode")).default;
    imageUrl.value = await QRCode.toDataURL(text.value, {
      width: Math.min(1024, Math.max(64, Number(size.value) || 320)),
      errorCorrectionLevel: level.value,
      color: { dark: dark.value, light: light.value },
      margin: 2,
    });
    error.value = null;
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
}

async function decodeFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  decoded.value = null;
  decodeError.value = null;
  try {
    const bitmap = await createImageBitmap(file);
    const canvas = document.createElement("canvas");
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("canvasUnavailable");
    context.drawImage(bitmap, 0, 0);
    bitmap.close();
    const data = context.getImageData(0, 0, canvas.width, canvas.height);
    const jsQR = (await import("jsqr")).default;
    const result = jsQR(data.data, data.width, data.height, { inversionAttempts: "attemptBoth" });
    if (!result) throw new Error("qrNotFound");
    decoded.value = result.data;
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    decodeError.value = message === "qrNotFound" ? t("devTools.qr.notFound") : message;
  }
}

void generate();
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.qr.title')"
    :description="t('devTools.tools.qr.description')"
    :icon="QrCode"
  >
    <div class="mx-auto max-w-[1080px] space-y-4">
      <div class="grid gap-4 lg:grid-cols-2">
        <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("devTools.qr.text") }}</Label>
            <textarea
              v-model="text"
              class="scrollbar-thin numeric h-24 w-full resize-y rounded-lg border border-hairline-strong bg-well px-2.5 py-2 text-[12px] outline-none focus:border-signal/50"
            />
          </div>
          <div class="flex flex-wrap items-end gap-2">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{ t("devTools.qr.size") }}</Label>
              <Input
                v-model="size"
                class="numeric h-8 w-20 rounded-md border-hairline-strong bg-well text-[11px]"
              />
            </div>
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{ t("devTools.qr.level") }}</Label>
              <div class="w-28">
                <FormSelect v-model="level" :options="levelOptions" />
              </div>
            </div>
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{ t("devTools.qr.colors") }}</Label>
              <div class="flex gap-1">
                <input
                  v-model="dark"
                  type="color"
                  class="h-8 w-8 rounded border border-hairline bg-well"
                />
                <input
                  v-model="light"
                  type="color"
                  class="h-8 w-8 rounded border border-hairline bg-well"
                />
              </div>
            </div>
            <Button size="sm" class="h-8 gap-1 text-[11px]" @click="generate">
              <QrCode class="size-3.5" />
              {{ t("devTools.qr.generate") }}
            </Button>
          </div>
          <p v-if="error" class="text-[11px] text-error">{{ error }}</p>
          <div
            v-if="imageUrl"
            class="flex flex-col items-center gap-2 rounded-xl border border-hairline bg-well p-3"
          >
            <img :src="imageUrl" :alt="text" class="size-[240px] object-contain" />
            <a
              :href="imageUrl"
              download="qrcode.png"
              class="inline-flex h-7 items-center gap-1 rounded-md border border-hairline-strong px-2 text-[10.5px] text-muted-foreground hover:text-foreground"
            >
              <Download class="size-3" />
              PNG
            </a>
          </div>
        </div>

        <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
          <div class="flex items-center justify-between">
            <p class="text-[11px] font-medium">{{ t("devTools.qr.decode") }}</p>
            <label
              class="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg border border-hairline-strong bg-well px-2.5 text-[11px] hover:border-signal/40"
            >
              <Upload class="size-3.5" />
              {{ t("devTools.qr.selectImage") }}
              <input type="file" accept="image/*" class="hidden" @change="decodeFile" />
            </label>
          </div>
          <p
            v-if="decodeError"
            class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
          >
            {{ decodeError }}
          </p>
          <div v-if="decoded" class="space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-[10px] uppercase tracking-wider text-muted-foreground">
                {{ t("devTools.qr.decoded") }}
              </span>
              <Button
                variant="ghost"
                size="sm"
                class="h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
                @click="copy('decoded', decoded)"
              >
                <Check v-if="isCopied('decoded')" class="size-3 text-signal" />
                <Copy v-else class="size-3" />
                {{ isCopied("decoded") ? t("common.copiedShort") : t("tools.copy") }}
              </Button>
            </div>
            <pre
              class="numeric max-h-[280px] overflow-auto whitespace-pre-wrap break-all rounded-lg border border-hairline bg-well px-3 py-2 text-[12px]"
              >{{ decoded }}</pre>
            <a
              v-if="/^https?:\/\//.test(decoded)"
              :href="decoded"
              target="_blank"
              rel="noreferrer"
              class="text-[11px] text-signal underline-offset-2 hover:underline"
            >
              {{ t("devTools.qr.openLink") }}
            </a>
          </div>
          <div
            v-else
            class="flex h-[240px] items-center justify-center rounded-xl border border-dashed border-hairline text-[11px] text-muted-foreground"
          >
            {{ t("devTools.qr.decodeHint") }}
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
