<script setup lang="ts">
import { Copy, Route, Square } from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import type { TracerouteHop } from "@/types/tools";

const { t } = useI18n();
const target = ref("8.8.8.8");
const maxHops = ref("30");
const probes = ref("3");
const timeoutMs = ref("1000");
const resolveNames = ref(true);
const running = ref(false);
const taskId = ref<string | null>(null);
const hops = ref<TracerouteHop[]>([]);
const reached = ref(false);
const error = ref<string | null>(null);
let unlisten: (() => void) | null = null;

const progressPercent = computed(() => {
  const max = Math.max(1, Number(maxHops.value) || 30);
  return Math.min(100, Math.round((hops.value.length / max) * 100));
});

onMounted(async () => {
  unlisten = await ipc.on("tool:event", (event) => {
    if (!taskId.value || event.taskId !== taskId.value) return;
    if (event.event === "tracerouteHop") {
      hops.value = [...hops.value, event.hop];
    } else if (event.event === "tracerouteDone") {
      running.value = false;
      taskId.value = null;
      reached.value = event.reached;
      if (event.error) error.value = toolErrorText(t, event.error);
    }
  });
});

onBeforeUnmount(() => {
  unlisten?.();
  unlisten = null;
});

async function start(): Promise<void> {
  if (running.value) return;
  if (!target.value.trim()) {
    error.value = t("tools.errors.invalidHost");
    return;
  }
  error.value = null;
  hops.value = [];
  reached.value = false;
  running.value = true;
  try {
    taskId.value = await ipc.toolTracerouteStart({
      target: target.value.trim(),
      maxHops: Math.min(64, Math.max(1, Number(maxHops.value) || 30)),
      probes: Math.min(3, Math.max(1, Number(probes.value) || 3)),
      timeoutMs: Math.min(5000, Math.max(100, Number(timeoutMs.value) || 1000)),
      resolveNames: resolveNames.value,
    });
  } catch (cause: unknown) {
    running.value = false;
    error.value = toolErrorText(t, cause);
  }
}

async function cancel(): Promise<void> {
  const current = taskId.value;
  if (current) await ipc.toolTaskCancel(current);
}

function hopStatus(hop: TracerouteHop): string {
  if (hop.reached) return t("networkTools.traceroute.reached");
  return t(`networkTools.traceroute.statuses.${hop.status}`);
}

function rtts(hop: TracerouteHop): string {
  return hop.rttsMs
    .map((value) => (value === null || value === undefined ? "*" : `${value.toFixed(1)} ms`))
    .join("  ");
}

async function copyHops(): Promise<void> {
  if (!hops.value.length) return;
  const text = hops.value
    .map((hop) => `${hop.hop}\t${hop.address ?? "*"}\t${hop.hostname ?? ""}\t${rtts(hop)}`)
    .join("\n");
  try {
    await navigator.clipboard.writeText(text);
    toast.success(t("common.copied"));
  } catch {
    toast.error(t("tools.copyFailed"));
  }
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.traceroute.title')"
    :description="t('networkTools.tools.traceroute.description')"
    :icon="Route"
  >
    <div class="mx-auto max-w-[1000px] space-y-4">
      <div
        class="grid grid-cols-2 gap-2 rounded-xl border border-hairline bg-well-soft p-3 lg:grid-cols-6"
      >
        <div class="col-span-2 space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.traceroute.target")
          }}</Label>
          <Input
            v-model="target"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            @keydown.enter="start"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.traceroute.maxHops")
          }}</Label>
          <Input
            v-model="maxHops"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.traceroute.probes")
          }}</Label>
          <Input
            v-model="probes"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.traceroute.timeout")
          }}</Label>
          <Input
            v-model="timeoutMs"
            class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="flex items-end">
          <Button v-if="!running" size="sm" class="h-9 w-full gap-1.5 text-[12px]" @click="start">
            <Route class="size-3.5" />
            {{ t("networkTools.traceroute.start") }}
          </Button>
          <Button
            v-else
            variant="outline"
            size="sm"
            class="h-9 w-full gap-1.5 border-warn/25 bg-warn/10 text-[12px] text-warn"
            @click="cancel"
          >
            <Square class="size-3" />
            {{ t("networkTools.traceroute.cancel") }}
          </Button>
        </div>
        <label class="col-span-2 flex items-center gap-2 text-[11px] text-muted-foreground">
          <Switch v-model="resolveNames" />
          {{ t("networkTools.traceroute.resolveNames") }}
        </label>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="hops.length || running" class="rounded-xl border border-hairline">
        <div class="border-b border-hairline px-3 py-2">
          <div class="flex items-center gap-2 text-[10.5px] text-muted-foreground">
            <span>{{ t("networkTools.traceroute.progress") }}</span>
            <span class="numeric">{{ hops.length }}/{{ maxHops }}</span>
            <span class="ml-auto">
              {{
                reached
                  ? t("networkTools.traceroute.reached")
                  : running
                    ? t("networkTools.traceroute.running")
                    : t("networkTools.traceroute.incomplete")
              }}
            </span>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 gap-1 px-1.5 text-[10.5px]"
              :disabled="!hops.length"
              @click="copyHops"
            >
              <Copy class="size-3" />
              {{ t("tools.copy") }}
            </Button>
          </div>
          <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-overlay-2">
            <div
              class="h-full rounded-full bg-gradient-to-r from-signal to-rx transition-[width] duration-200"
              :style="{ width: `${progressPercent}%` }"
            />
          </div>
        </div>
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">#</th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.traceroute.address") }}
                </th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.traceroute.hostname") }}
                </th>
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.traceroute.rtts") }}
                </th>
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.traceroute.statusLabel") }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="hop in hops" :key="hop.hop" class="border-b border-hairline last:border-0">
                <td class="numeric px-3 py-1.5 text-muted-foreground">{{ hop.hop }}</td>
                <td class="numeric px-2 py-1.5 text-foreground">{{ hop.address ?? "*" }}</td>
                <td class="px-2 py-1.5 text-muted-foreground">{{ hop.hostname ?? "—" }}</td>
                <td class="numeric px-3 py-1.5 text-muted-foreground">{{ rtts(hop) }}</td>
                <td class="px-3 py-1.5">
                  <span
                    class="rounded border px-1 py-0.5 text-[10px]"
                    :class="
                      hop.reached
                        ? 'border-signal/25 bg-signal/10 text-signal'
                        : hop.status === 'timeout'
                          ? 'border-warn/25 bg-warn/10 text-warn'
                          : 'border-hairline bg-overlay-1 text-muted-foreground'
                    "
                  >
                    {{ hopStatus(hop) }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div
        v-else-if="!running"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.traceroute.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
