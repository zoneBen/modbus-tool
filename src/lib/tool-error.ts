import { toToolError } from "@/types/tools";

type Translate = (key: string) => string;

export function toolErrorText(t: Translate, error: unknown): string {
  const toolError = toToolError(error);
  const key = `tools.errors.${toolError.code}`;
  const translated = t(key);
  if (translated !== key) {
    return toolError.detail ? `${translated} · ${toolError.detail}` : translated;
  }
  return toolError.detail || toolError.code;
}
