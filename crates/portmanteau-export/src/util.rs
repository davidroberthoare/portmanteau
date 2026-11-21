use super::*;

pub(crate) fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.replace('\n', " "))
        .collect()
}

pub(crate) fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

pub(crate) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Page width and height in twentieths of a point, the unit DOCX and RTF use.
pub(crate) fn paper_twips(settings: &ExportSettings) -> (i64, i64) {
    let (w_mm, h_mm) = settings.paper.mm();
    (
        (w_mm as f64 * 1440.0 / 25.4).round() as i64,
        (h_mm as f64 * 1440.0 / 25.4).round() as i64,
    )
}
