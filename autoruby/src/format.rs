use wana_kana::ConvertJapanese;

pub trait Format {
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

pub fn markdown(base: &str, text: &str) -> String {
    format!("[{base}]{{{text}}}")
}

pub fn html(base: &str, text: &str) -> String {
    format!("<ruby>{base}<rp>(</rp><rt>{text}</rt><rp>)</rp></ruby>")
}

pub fn latex(base: &str, text: &str) -> String {
    format!("\\ruby{{{base}}}{{{text}}}")
}

pub fn use_katakana(f: impl Format) -> impl Format {
    move |base: &str, text: &str| f.format(base, &text.to_katakana())
}
