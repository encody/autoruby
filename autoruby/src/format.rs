//! Annotation formatting.

use wana_kana::ConvertJapanese;

/// Format annotations.
pub trait Format {
    /// Formats the given base text with annotation text.
    fn format(&self, base: &str, text: &str) -> String;
}

/// Markdown annotation formatting.
pub struct Markdown;

impl Format for Markdown {
    fn format(&self, base: &str, text: &str) -> String {
        format!("[{base}]{{{text}}}")
    }
}

/// HTML annotation formatting.
pub struct Html;

impl Format for Html {
    fn format(&self, base: &str, text: &str) -> String {
        format!("<ruby>{base}<rp>(</rp><rt>{text}</rt><rp>)</rp></ruby>")
    }
}

/// LaTeX annotation formatting.
pub struct Latex;

impl Format for Latex {
    fn format(&self, base: &str, text: &str) -> String {
        format!("\\ruby{{{base}}}{{{text}}}")
    }
}

/// Converts the annotation text to katakana.
pub struct WithKatakana<'a>(pub &'a dyn Format);

impl<'a> Format for WithKatakana<'a> {
    fn format(&self, base: &str, text: &str) -> String {
        self.0.format(base, &text.to_katakana())
    }
}
