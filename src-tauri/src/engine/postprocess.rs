//! LaTeX 文本后处理：清理解码输出里多余的空白。
//!
//! LaTeX post-processing: removes redundant whitespace from decoder output.
//!
//! 与 RapidLaTeXOCR 的 `post_process` 一致（原实现来自 LaTeX-OCR）：
//! 1. `\mathrm {x}` / `\text {a b}` 等命令内的空格去掉（`\mathrm{x}`）；
//! 2. 循环折叠「非字母-空白-非字母」等相邻组合里的空白；
//!    `(?!\\ )` 防止吞掉 LaTeX 的转义空格 `\ `。
//!
//! Matches RapidLaTeXOCR's `post_process` (originally from LaTeX-OCR):
//! 1. collapse spaces inside `\mathrm {x}`-style commands;
//! 2. iteratively fold whitespace between letter/non-letter neighbours, with
//!    `(?!\\ )` guarding the LaTeX escaped space `\ `.

use fancy_regex::Regex;
use std::sync::LazyLock;

/// `\operatorname {x}`、`\mathrm {x}` 等：可选空白、可选 `*`、空格、非贪婪花括号体。
/// `\operatorname {x}`, `\mathrm {x}` etc.: optional space, optional `*`, a space,
/// then a non-greedy brace body.
static TEXT_REG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\\(operatorname|mathrm|text|mathbf)\s?\*? \{.*?\})").expect("TEXT_REG 编译失败")
});

static NOLETTER_NOLETTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?!\\ )([\W_^\d])\s+?([\W_^\d])"#).expect("regex 编译失败"));
static NOLETTER_LETTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?!\\ )([\W_^\d])\s+?([a-zA-Z])"#).expect("regex 编译失败"));
static LETTER_NOLETTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"([a-zA-Z])\s+?([\W_^\d])"#).expect("regex 编译失败"));

/// 去除多余空白，返回清理后的 LaTeX。
/// Strips redundant whitespace; returns cleaned LaTeX.
pub fn post_process(input: &str) -> String {
    // 第一步：\mathrm {a} → \mathrm{a}
    // Step 1: \mathrm {a} → \mathrm{a}
    let mut names: Vec<String> = Vec::new();
    for capture in TEXT_REG.captures_iter(input).flatten() {
        if let Some(m) = capture.get(0) {
            names.push(m.as_str().replace(' ', ""));
        }
    }
    let mut replaced = String::with_capacity(input.len());
    let mut last = 0usize;
    let mut next = 0usize;
    for capture in TEXT_REG.captures_iter(input).flatten() {
        let Some(m) = capture.get(0) else {
            continue;
        };
        replaced.push_str(&input[last..m.start()]);
        // 与 Python names.pop(0) 一致：按出现顺序消费
        // consume in order of appearance, like Python's names.pop(0)
        if let Some(name) = names.get(next) {
            replaced.push_str(name);
            next += 1;
        }
        last = m.end();
    }
    replaced.push_str(&input[last..]);
    let mut s = replaced;

    // 第二步：循环折叠空白，直到稳定
    // Step 2: fold whitespace until stable
    loop {
        let news = NOLETTER_NOLETTER.replace_all(&s, "${1}${2}").to_string();
        let news = NOLETTER_LETTER.replace_all(&news, "${1}${2}").to_string();
        let news = LETTER_NOLETTER.replace_all(&news, "${1}${2}").to_string();
        if news == s {
            break;
        }
        s = news;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_space_in_mathrm() {
        assert_eq!(post_process(r"\mathrm {d}x"), r"\mathrm{d}x");
        assert_eq!(post_process(r"\operatorname {sn} x"), r"\operatorname{sn}x");
    }

    #[test]
    fn folds_whitespace_between_symbols() {
        // "5 + 3" → "5+3"（数字属 noletter）
        // "5 + 3" → "5+3" (digits count as noletter)
        assert_eq!(post_process("5 + 3"), "5+3");
        assert_eq!(post_process("a + b"), "a+b");
        // 字母-字母之间的空格按 Python 语义保留（t 与 b 都是 letter，无规则折叠）
        // letter-letter spaces are preserved by design (both t and b are letters)
        assert_eq!(post_process("a \\cdot b"), "a\\cdot b");
    }

    #[test]
    fn keeps_escaped_space() {
        // `\ `（LaTeX 空格）不应被吞掉
        // the LaTeX escaped space `\ ` must survive
        assert_eq!(post_process("a\\ b"), "a\\ b");
    }

    #[test]
    fn idempotent_on_clean_input() {
        let cleaned = post_process(r"\frac{a}{b}+\sqrt{x}");
        assert_eq!(cleaned, r"\frac{a}{b}+\sqrt{x}");
        assert_eq!(post_process(&cleaned), cleaned);
    }

    #[test]
    fn handles_multi_space_runs() {
        assert_eq!(post_process("x   =   y"), "x=y");
    }

    #[test]
    fn mathrm_body_spaces_preserved() {
        // \text 体内空格已在第一步压缩（与 Python 行为一致）
        // \text body spaces are collapsed in step one (matching Python)
        let out = post_process(r"\text {a b}");
        assert_eq!(out, r"\text{ab}");
    }
}
