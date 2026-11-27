<script setup lang="ts">
import { Check, Copy, FileUp, FolderOpen, Globe2, Plus, Send, X } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormSelect from "@/components/common/FormSelect.vue";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { useCopyFeedback } from "@/composables/useCopyFeedback";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { useToolboxStore } from "@/stores/toolbox";
import { toolErrorText } from "@/lib/tool-error";
import type { HttpBodyMode, HttpMethod, HttpResponse } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const method = toolbox.toolPref<HttpMethod>("network.http.method", "POST");
const url = toolbox.toolPref("network.http.url", "https://example.com/");
const headers = ref<Array<{ name: string; value: string }>>([]);
const bodyMode = toolbox.toolPref<HttpBodyMode>("network.http.bodyMode", "json");
const body = ref('{\n  "hello": "world"\n}');
const formFields = ref<Array<{ name: string; value: string }>>([{ name: "", value: "" }]);
const uploadPath = ref("");
const uploadField = toolbox.toolPref("network.http.uploadField", "file");
const timeoutMs = toolbox.toolPref("network.http.timeout", "15000");
const followRedirects = toolbox.toolPref("network.http.followRedirects", true);
const caPath = ref("");
const busy = ref(false);
const result = ref<HttpResponse | null>(null);
const error = ref<string | null>(null);

const methodOptions: Array<{ label: HttpMethod; value: HttpMethod }> = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
].map((value) => ({ label: value as HttpMethod, value: value as HttpMethod }));

const bodyModeOptions = computed<Array<{ label: string; value: HttpBodyMode }>>(() =>
  (["none", "json", "form", "multipart", "raw"] as HttpBodyMode[]).map((value) => ({
    label: t(`networkTools.http.bodyModes.${value}`),
    value,
  })),
);

const bodyDisabled = computed(() => method.value === "GET" || method.value === "HEAD");
const displayBody = computed(() => {
  const response = result.value;
  if (!response) return "";
  if (response.contentType?.includes("json") && !response.binary) {
    try {
      return JSON.stringify(JSON.parse(response.body), null, 2);
    } catch {
      return response.body;
    }
  }
  return response.body;
});

function addHeader(): void {
  headers.value.push({ name: "", value: "" });
}

function removeHeader(index: number): void {
  headers.value.splice(index, 1);
}

function addFormField(): void {
  formFields.value.push({ name: "", value: "" });
}

function removeFormField(index: number): void {
  formFields.value.splice(index, 1);
}

async function browseUpload(): Promise<void> {
  const path = await ipc.pickFile();
  if (path) uploadPath.value = path;
}

async function browseCa(): Promise<void> {
  const path = await ipc.pickFile([
    { name: t("networkTools.http.certificateFiles"), extensions: ["pem", "crt", "cer", "der"] },
  ]);
  if (path) caPath.value = path;
}

async function send(): Promise<void> {
  if (busy.value) return;
  if (!url.value.trim()) {
    error.value = t("tools.errors.invalidUrl");
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    result.value = await ipc.toolHttpRequest({
      method: method.value,
      url: url.value.trim(),
      headers: headers.value
        .map((header) => ({ name: header.name.trim(), value: header.value }))
        .filter((header) => header.name),
      body: bodyDisabled.value ? undefined : body.value,
      bodyMode: bodyDisabled.value ? "none" : bodyMode.value,
      formFields: formFields.value
        .map((field) => ({ name: field.name.trim(), value: field.value }))
        .filter((field) => field.name),
      uploadPath:
        !bodyDisabled.value && bodyMode.value === "multipart" && uploadPath.value.trim()
          ? uploadPath.value.trim()
          : undefined,
      uploadField: uploadField.value.trim() || "file",
      timeoutMs: Math.min(120_000, Math.max(100, Number(timeoutMs.value) || 15000)),
      followRedirects: followRedirects.value,
      caPath: caPath.value.trim() || undefined,
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
    result.value = null;
  } finally {
    busy.value = false;
  }
}

const { copy: copyFeedback, isCopied } = useCopyFeedback();

async function copy(key: string, value: string): Promise<void> {
  if (!value) return;
  await copyFeedback(key, value);
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.http.title')"
    :description="t('networkTools.tools.http.description')"
    :icon="Globe2"
  >
    <div class="mx-auto max-w-[1100px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-[112px_minmax(0,1fr)_150px_120px]">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.http.method")
            }}</Label>
            <FormSelect v-model="method" :options="methodOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.http.url")
            }}</Label>
            <Input
              v-model="url"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              placeholder="https://api.example.com/endpoint"
              @keydown.enter="send"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.http.timeout")
            }}</Label>
            <Input
              v-model="timeoutMs"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="flex items-end">
            <Button size="sm" class="h-9 w-full gap-1.5 text-[12px]" :disabled="busy" @click="send">
              <Send class="size-3.5" />
              {{ busy ? t("tools.running") : t("networkTools.http.send") }}
            </Button>
          </div>
        </div>

        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.http.headers")
            }}</Label>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
              @click="addHeader"
            >
              <Plus class="size-3" />
              {{ t("networkTools.http.addHeader") }}
            </Button>
          </div>
          <div v-for="(header, index) in headers" :key="index" class="flex items-center gap-2">
            <Input
              v-model="header.name"
              class="numeric h-8 w-[220px] shrink-0 rounded-lg border-hairline-strong bg-well text-[11px]"
              placeholder="Header"
            />
            <Input
              v-model="header.value"
              class="h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[11px]"
              placeholder="Value"
            />
            <button
              type="button"
              class="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
              :aria-label="t('networkTools.http.removeHeader')"
              @click="removeHeader(index)"
            >
              <X class="size-3.5" />
            </button>
          </div>
          <p v-if="!headers.length" class="text-[10.5px] text-muted-foreground">
            {{ t("networkTools.http.noHeaders") }}
          </p>
        </div>

        <div class="space-y-2">
          <div class="flex flex-wrap items-center gap-2">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.http.body")
            }}</Label>
            <div class="w-[150px]">
              <FormSelect v-model="bodyMode" :options="bodyModeOptions" />
            </div>
            <span class="text-[10.5px] text-muted-foreground">
              {{ t("networkTools.http.bodyModeHint") }}
            </span>
          </div>

          <p
            v-if="bodyDisabled"
            class="rounded-lg border border-hairline bg-well px-3 py-2 text-[10.5px] text-muted-foreground"
          >
            {{ t("networkTools.http.bodyDisabled") }}
          </p>

          <template v-else-if="bodyMode === 'none'">
            <p class="text-[10.5px] text-muted-foreground">
              {{ t("networkTools.http.noBody") }}
            </p>
          </template>

          <template v-else-if="bodyMode === 'form' || bodyMode === 'multipart'">
            <div class="flex items-center justify-between">
              <span class="text-[10.5px] text-muted-foreground">
                {{ t("networkTools.http.formFields") }}
              </span>
              <Button
                variant="ghost"
                size="sm"
                class="h-6 gap-1 px-1.5 text-[10.5px] text-muted-foreground"
                @click="addFormField"
              >
                <Plus class="size-3" />
                {{ t("networkTools.http.addField") }}
              </Button>
            </div>
            <div class="space-y-1.5">
              <div
                v-for="(field, index) in formFields"
                :key="index"
                class="flex items-center gap-2"
              >
                <Input
                  v-model="field.name"
                  class="numeric h-8 w-[220px] shrink-0 rounded-lg border-hairline-strong bg-well text-[11px]"
                  :placeholder="t('networkTools.http.fieldName')"
                />
                <Input
                  v-model="field.value"
                  class="h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[11px]"
                  :placeholder="t('networkTools.http.fieldValue')"
                />
                <button
                  type="button"
                  class="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
                  :aria-label="t('networkTools.http.removeField')"
                  @click="removeFormField(index)"
                >
                  <X class="size-3.5" />
                </button>
              </div>
              <p v-if="!formFields.length" class="text-[10.5px] text-muted-foreground">
                {{ t("networkTools.http.noFields") }}
              </p>
            </div>
            <div v-if="bodyMode === 'multipart'" class="space-y-1.5">
              <div class="flex flex-wrap items-center gap-2">
                <Label class="shrink-0 text-[10.5px] text-muted-foreground">{{
                  t("networkTools.http.uploadField")
                }}</Label>
                <Input
                  v-model="uploadField"
                  class="numeric h-8 w-[150px] rounded-lg border-hairline-strong bg-well text-[11px]"
                  placeholder="file"
                />
                <Input
                  v-model="uploadPath"
                  class="numeric h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[11px]"
                  :placeholder="t('networkTools.http.uploadPlaceholder')"
                />
                <Button
                  variant="outline"
                  size="sm"
                  class="h-8 shrink-0 gap-1 border-hairline-strong bg-transparent text-[11px]"
                  @click="browseUpload"
                >
                  <FolderOpen class="size-3" />
                  {{ t("networkTools.http.browse") }}
                </Button>
                <Button
                  v-if="uploadPath"
                  variant="ghost"
                  size="sm"
                  class="h-8 shrink-0 text-[10.5px] text-muted-foreground"
                  @click="uploadPath = ''"
                >
                  {{ t("common.clear") }}
                </Button>
              </div>
              <p class="flex items-center gap-1 text-[10px] text-muted-foreground">
                <FileUp class="size-3" />
                {{ t("networkTools.http.uploadHint") }}
              </p>
            </div>
          </template>

          <textarea
            v-else
            v-model="body"
            rows="6"
            class="scrollbar-thin numeric w-full resize-y rounded-lg border border-hairline-strong bg-well px-3 py-2 text-[11.5px] text-foreground outline-none transition-colors focus:border-signal/50"
            :placeholder="
              bodyMode === 'json'
                ? t('networkTools.http.jsonPlaceholder')
                : t('networkTools.http.bodyPlaceholder')
            "
          />
        </div>

        <div class="flex flex-wrap items-center gap-4">
          <label class="flex items-center gap-2 text-[11px] text-muted-foreground">
            <Switch v-model="followRedirects" />
            {{ t("networkTools.http.followRedirects") }}
          </label>
          <div class="flex min-w-0 flex-1 items-center gap-2">
            <Label class="shrink-0 text-[11px] text-muted-foreground">{{
              t("networkTools.http.ca")
            }}</Label>
            <Input
              v-model="caPath"
              class="numeric h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[11px]"
              :placeholder="t('networkTools.http.caPlaceholder')"
            />
            <Button
              variant="outline"
              size="sm"
              class="h-8 shrink-0 gap-1 border-hairline-strong bg-transparent text-[11px]"
              @click="browseCa"
            >
              <FolderOpen class="size-3" />
              {{ t("networkTools.http.browse") }}
            </Button>
          </div>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="space-y-3">
        <div
          class="flex flex-wrap items-center gap-2 rounded-xl border border-hairline bg-well-soft px-3 py-2.5 text-[11px]"
        >
          <span
            class="rounded border px-1.5 py-0.5"
            :class="
              result.status < 400
                ? 'border-signal/25 bg-signal/10 text-signal'
                : 'border-error/25 bg-error/10 text-error'
            "
          >
            {{ result.status }} {{ result.statusText }}
          </span>
          <span class="numeric text-muted-foreground">
            {{ result.elapsedMs }} ms · {{ result.bodyBytes }} B
            <template v-if="result.truncated"> · {{ t("networkTools.http.truncated") }}</template>
          </span>
          <span class="min-w-0 truncate text-muted-foreground">{{ result.finalUrl }}</span>
        </div>

        <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_340px]">
          <div class="rounded-xl border border-hairline">
            <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
              <span class="text-[11px] font-medium">{{ t("networkTools.http.responseBody") }}</span>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 text-[11px] text-muted-foreground"
                :disabled="!displayBody"
                @click="copy('body', displayBody)"
              >
                <Check v-if="isCopied('body')" class="size-3 text-signal" />
                <Copy v-else class="size-3" />
                {{ isCopied("body") ? t("common.copiedShort") : t("tools.copy") }}
              </Button>
            </div>
            <pre
              class="scrollbar-thin max-h-[460px] min-h-[160px] overflow-auto whitespace-pre-wrap break-all px-3 py-2.5 text-[11px] text-muted-foreground"
              >{{ displayBody || "—" }}</pre>
          </div>

          <div class="rounded-xl border border-hairline">
            <div class="flex items-center justify-between border-b border-hairline px-3 py-2">
              <span class="text-[11px] font-medium">{{
                t("networkTools.http.responseHeaders")
              }}</span>
              <Button
                variant="ghost"
                size="sm"
                class="h-7 gap-1 text-[11px] text-muted-foreground"
                :disabled="!result.headers.length"
                @click="
                  copy('headers', result.headers.map((h) => `${h.name}: ${h.value}`).join('\n'))
                "
              >
                <Check v-if="isCopied('headers')" class="size-3 text-signal" />
                <Copy v-else class="size-3" />
                {{ isCopied("headers") ? t("common.copiedShort") : t("tools.copy") }}
              </Button>
            </div>
            <div class="scrollbar-thin max-h-[430px] overflow-auto px-3 py-2">
              <p
                v-for="(header, index) in result.headers"
                :key="`${header.name}-${index}`"
                class="numeric break-all py-0.5 text-[10.5px] leading-relaxed text-muted-foreground"
              >
                <span class="text-foreground">{{ header.name }}</span>: {{ header.value }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else-if="!busy"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.http.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
