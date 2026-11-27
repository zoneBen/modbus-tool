<script setup lang="ts">
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import { copyText } from "@/lib/clipboard";
import { ipc } from "@/lib/ipc";

const props = defineProps<{
  sessionId: string;
  active: boolean;
  connected: boolean;
}>();

const { t } = useI18n();
const container = ref<HTMLDivElement | null>(null);
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let unlistenData: (() => void) | null = null;
let observer: ResizeObserver | null = null;

function decodeBase64(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function fit(): void {
  if (!terminal || !container.value) return;
  if (!container.value.offsetWidth || !container.value.offsetHeight) return;
  try {
    fitAddon?.fit();
  } catch {
    // A hidden terminal can report a zero-size viewport; retry when shown.
  }
}

function focus(): void {
  terminal?.focus();
}

function clear(): void {
  terminal?.clear();
}

async function copySelection(): Promise<void> {
  const selection = terminal?.getSelection() ?? "";
  if (!selection) return;
  if (await copyText(selection)) {
    toast.success(t("common.copiedShort"));
  } else {
    toast.error(t("tools.copyFailed"));
  }
}

function scheduleFit(): void {
  window.requestAnimationFrame(() => {
    fit();
    if (props.active) focus();
  });
}

onMounted(async () => {
  if (!container.value) return;
  terminal = new Terminal({
    allowTransparency: true,
    convertEol: false,
    cursorBlink: true,
    cursorStyle: "bar",
    fontFamily: '"JetBrains Mono Variable", "Cascadia Code", monospace',
    fontSize: 12.5,
    lineHeight: 1.2,
    scrollback: 6000,
    theme: {
      background: "#0a111b",
      foreground: "#d7e2ee",
      cursor: "#5eead4",
      cursorAccent: "#0a111b",
      selectionBackground: "#214e6b",
      black: "#0a111b",
      red: "#fb7185",
      green: "#5eead4",
      yellow: "#fbbf24",
      blue: "#60a5fa",
      magenta: "#c084fc",
      cyan: "#22d3ee",
      white: "#d7e2ee",
      brightBlack: "#64748b",
      brightRed: "#fda4af",
      brightGreen: "#99f6e4",
      brightYellow: "#fde68a",
      brightBlue: "#93c5fd",
      brightMagenta: "#d8b4fe",
      brightCyan: "#67e8f9",
      brightWhite: "#f8fafc",
    },
  });
  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(container.value);
  terminal.options.disableStdin = !props.connected;

  terminal.attachCustomKeyEventHandler((event) => {
    if (event.type !== "keydown") return true;
    if (event.ctrlKey && event.shiftKey && event.code === "KeyC") {
      void copySelection();
      return false;
    }
    if (event.ctrlKey && event.shiftKey && event.code === "KeyV") {
      void navigator.clipboard
        .readText()
        .then((text) => (text ? ipc.toolSshWrite(props.sessionId, text) : undefined))
        .catch(() => undefined);
      return false;
    }
    return true;
  });

  terminal.onData((data) => {
    void ipc.toolSshWrite(props.sessionId, data).catch(() => undefined);
  });
  terminal.onResize(({ cols, rows }) => {
    void ipc.toolSshResize(props.sessionId, cols, rows).catch(() => undefined);
  });

  unlistenData = await ipc.on("ssh:data", (event) => {
    if (event.sessionId !== props.sessionId) return;
    terminal?.write(decodeBase64(event.data));
  });

  observer = new ResizeObserver(() => fit());
  observer.observe(container.value);
  await nextTick();
  scheduleFit();

  // Replay retained output so a remount or a fast-connecting session still
  // shows the banner, prompt and earlier scrollback.
  try {
    const history = await ipc.toolSshReadOutput(props.sessionId, 0);
    if (history.data) terminal.write(history.data);
  } catch {
    // Session may have ended before the terminal mounted.
  }
});

watch(
  () => props.active,
  (active) => {
    if (active) scheduleFit();
  },
);

watch(
  () => props.connected,
  (connected) => {
    if (terminal) terminal.options.disableStdin = !connected;
    if (connected) scheduleFit();
  },
);

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  unlistenData?.();
  unlistenData = null;
  terminal?.dispose();
  terminal = null;
  fitAddon = null;
});

defineExpose({ clear, fit, focus, copySelection });
</script>

<template>
  <div ref="container" class="h-full w-full overflow-hidden rounded-xl bg-[#0a111b] p-2" />
</template>
