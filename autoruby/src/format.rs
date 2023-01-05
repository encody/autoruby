pub type Format = fn(&str, &str) -> String;

pub fn markdown(base: &str, text: &str) -> String {
    format!("[{base}]{{{text}}}")
}

pub fn html(base: &str, text: &str) -> String {
    format!("<ruby>{base}<rp>(</rp><rt>{text}</rt><rp>)</rp></ruby>")
}

pub fn latex(base: &str, text: &str) -> String {
    format!("\\ruby{{{base}}}{{{text}}}")
}
