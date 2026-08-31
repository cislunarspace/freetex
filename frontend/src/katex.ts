// KaTeX / Temml 渲染助手：预览用 KaTeX，MathML 复制用 Temml。
// KaTeX / Temml render helpers: preview via KaTeX, MathML copies via Temml.

import katex from "katex";
import temml from "temml";
import "katex/dist/katex.min.css";

/** 渲染 LaTeX 为 HTML（出错时内联显示原文，不抛异常）。 */
export function renderLatex(latex: string): string {
  return katex.renderToString(latex, {
    displayMode: true,
    throwOnError: false,
    output: "html",
  });
}

/** 渲染 LaTeX 为 MathML 字符串（供 Word 粘贴）。 */
export function renderMathml(latex: string): string {
  return temml.renderToString(latex, {
    displayMode: false,
    throwOnError: false,
  });
}
