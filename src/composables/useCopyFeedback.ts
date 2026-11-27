import { onScopeDispose, ref } from "vue";
import { toast } from "vue-sonner";
import { i18n } from "@/i18n";
import { copyText } from "@/lib/clipboard";

const DEFAULT_FEEDBACK_MS = 5000;

/**
 * Copy text and keep a transient "copied" state for five seconds so every
 * copy button in the app gives the same visual confirmation.
 */
export function useCopyFeedback(feedbackMs = DEFAULT_FEEDBACK_MS) {
  const copiedKey = ref<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function clear(): void {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    copiedKey.value = null;
  }

  async function copy(key: string, text: string): Promise<boolean> {
    const ok = await copyText(text);
    if (!ok) {
      toast.error(i18n.global.t("tools.copyFailed"));
      return false;
    }
    copiedKey.value = key;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      copiedKey.value = null;
      timer = null;
    }, feedbackMs);
    return true;
  }

  function isCopied(key: string): boolean {
    return copiedKey.value === key;
  }

  onScopeDispose(clear);

  return { copiedKey, copy, isCopied, clear };
}
