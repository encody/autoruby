//! Annotation formatting.

use wana_kana::ConvertJapanese;

/// Format annotations.
pub trait Format {
    /// Formats the given base text with annotation text.
    fn format(&self, base: &str, text: &str) -> String;
}

impl<T> Format for T
where
    T: Fn(&str, &str) -> String,
{
    fn format(&self, base: &str, text: &str) -> String {
        self(base, text)
    }
}

/// Markdown annotation formatting.
#[must_use]
pub fn markdown(base: &str, text: &str) -> String {
    format!("[{base}]{{{text}}}")
}

/// HTML annotation formatting.
#[must_use]
pub fn html(base: &str, text: &str) -> String {
    format!("<ruby>{base}<rp>(</rp><rt>{text}</rt><rp>)</rp></ruby>")
}

/// LaTeX annotation formatting.
#[must_use]
pub fn latex(base: &str, text: &str) -> String {
    format!("\\ruby{{{base}}}{{{text}}}")
}

/// Converts the annotation text to katakana.
pub fn use_katakana(f: impl Format) -> impl Format {
    move |base: &str, text: &str| f.format(base, &text.to_katakana())
}
