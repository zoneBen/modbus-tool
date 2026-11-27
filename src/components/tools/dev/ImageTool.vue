<script setup lang="ts">
import { Download, ImageDown, Images, Trash2 } from "@lucide/vue";
import { computed, onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface ConvertedImage {
  id: string;
  name: string;
  originalSize: number;
  size: number;
  width: number;
  height: number;
  url: string;
  preview: string;
}

const { t } = useI18n();
const format = ref<"image/png" | "image/jpeg" | "image/webp">("image/webp");
const quality = ref("82");
const maxWidth = ref("1920");
const items = ref<ConvertedImage[]>([]);
const busy = ref(false);
const error = ref<string | null>(null);
const urls = new Set<string>();

const formatOptions = [
  { value: "image/webp" as const, label: "WebP" },
  { value: "image/jpeg" as const, label: "JPEG" },
  { value: "image/png" as const, label: "PNG" },
];

const extension = computed(() =>
  format.value === "image/png" ? "png" : format.value === "image/jpeg" ? "jpg" : "webp",
);

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

async function onFiles(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  input.value = "";
  if (!files.length) return;
  busy.value = true;
  error.value = null;
  try {
    for (const file of files) {
      if (!file.type.startsWith("image/")) continue;
      const bitmap = await createImageBitmap(file);
      const limit = Math.max(32, Number(maxWidth.value) || 1920);
      const scale = Math.min(1, limit / bitmap.width);
      const width = Math.round(bitmap.width * scale);
      const height = Math.round(bitmap.height * scale);
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const context = canvas.getContext("2d");
      if (!context) throw new Error("canvasUnavailable");
      context.imageSmoothingQuality = "high";
      context.drawImage(bitmap, 0, 0, width, height);
      bitmap.close();
      const blob = await new Promise<Blob | null>((resolve) =>
        canvas.toBlob(
          resolve,
          format.value,
          Math.min(1, Math.max(0.1, Number(quality.value) / 100)),
        ),
      );
      if (!blob) throw new Error("encodeFailed");
      const url = URL.createObjectURL(blob);
      urls.add(url);
      const preview = canvas.toDataURL("image/png");
      items.value = [
        ...items.value,
        {
          id: `${file.name}-${Date.now()}-${items.value.length}`,
          name: file.name.replace(/\.[^.]+$/, "") + `.${extension.value}`,
          originalSize: file.size,
          size: blob.size,
          width,
          height,
          url,
          preview,
        },
      ];
    }
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy.value = false;
  }
}

function remove(id: string): void {
  items.value = items.value.filter((item) => {
    if (item.id === id) {
      URL.revokeObjectURL(item.url);
      urls.delete(item.url);
    }
    return item.id !== id;
  });
}

function clear(): void {
  for (const item of items.value) URL.revokeObjectURL(item.url);
  urls.clear();
  items.value = [];
}

onBeforeUnmount(() => {
  for (const url of urls) URL.revokeObjectURL(url);
});
</script>

<template>
  <ToolPanelFrame
    :title="t('devTools.tools.image.title')"
    :description="t('devTools.tools.image.description')"
    :icon="Images"
  >
    <div class="mx-auto max-w-[1080px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-3 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.image.format") }}</Label>
          <div class="w-32">
            <FormSelect v-model="format" :options="formatOptions" />
          </div>
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("devTools.image.quality") }}</Label>
          <Input
            v-model="quality"
            class="numeric h-9 w-20 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("devTools.image.maxWidth")
          }}</Label>
          <Input
            v-model="maxWidth"
            class="numeric h-9 w-24 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <label
          class="inline-flex h-9 cursor-pointer items-center gap-1.5 rounded-lg border border-hairline-strong bg-well px-3 text-[11.5px] hover:border-signal/40"
        >
          <ImageDown class="size-3.5" />
          {{ busy ? t("tools.running") : t("devTools.image.select") }}
          <input
            type="file"
            accept="image/*"
            multiple
            class="hidden"
            :disabled="busy"
            @change="onFiles"
          />
        </label>
        <Button
          v-if="items.length"
          variant="ghost"
          size="sm"
          class="ml-auto h-9 gap-1 text-[11px] text-muted-foreground"
          @click="clear"
        >
          <Trash2 class="size-3.5" />
          {{ t("devTools.image.clear") }}
        </Button>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ error }}
      </p>

      <div v-if="items.length" class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        <div
          v-for="item in items"
          :key="item.id"
          class="overflow-hidden rounded-xl border border-hairline bg-well-soft"
        >
          <div
            class="flex h-36 items-center justify-center bg-[repeating-conic-gradient(#0000_0%_25%,#ffffff08_0%_50%)] bg-[length:16px_16px]"
          >
            <img :src="item.preview" :alt="item.name" class="max-h-36 max-w-full object-contain" />
          </div>
          <div class="space-y-1 p-2.5">
            <p class="truncate text-[11px] font-medium">{{ item.name }}</p>
            <p class="numeric text-[10px] text-muted-foreground">
              {{ item.width }}×{{ item.height }} · {{ formatBytes(item.originalSize) }} →
              <span class="text-signal">{{ formatBytes(item.size) }}</span>
            </p>
            <div class="flex gap-1 pt-1">
              <a
                :href="item.url"
                :download="item.name"
                class="inline-flex h-7 flex-1 items-center justify-center gap-1 rounded-md border border-hairline-strong text-[10.5px] text-muted-foreground transition-colors hover:text-foreground"
              >
                <Download class="size-3" />
                {{ t("devTools.image.download") }}
              </a>
              <button
                type="button"
                class="inline-flex size-7 items-center justify-center rounded-md border border-hairline-strong text-muted-foreground hover:text-error"
                @click="remove(item.id)"
              >
                <Trash2 class="size-3" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else-if="!busy"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("devTools.image.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
