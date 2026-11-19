import type { Plugin } from "prettier";

export type FormatLanguage = "json" | "javascript" | "typescript" | "html" | "css" | "xml";

export async function formatText(
  text: string,
  language: FormatLanguage,
  indent: 2 | 4,
): Promise<string> {
  if (language === "xml") {
    const module = await import("xml-formatter");
    const formatXml = module.default;
    return formatXml(text, {
      indentation: " ".repeat(indent),
      collapseContent: true,
      lineSeparator: "\n",
    });
  }

  const prettier = await import("prettier/standalone");
  const plugins: Plugin[] = [
    (await import("prettier/plugins/babel")) as unknown as Plugin,
    (await import("prettier/plugins/estree")) as unknown as Plugin,
  ];
  if (language === "typescript") {
    plugins.push((await import("prettier/plugins/typescript")) as unknown as Plugin);
  }
  if (language === "html") {
    plugins.push((await import("prettier/plugins/html")) as unknown as Plugin);
    plugins.push((await import("prettier/plugins/postcss")) as unknown as Plugin);
  }
  const parser =
    language === "json"
      ? "json"
      : language === "javascript"
        ? "babel"
        : language === "typescript"
          ? "typescript"
          : language === "html"
            ? "html"
            : "css";

  return prettier.format(text, {
    parser,
    plugins,
    tabWidth: indent,
    printWidth: 100,
  });
}
