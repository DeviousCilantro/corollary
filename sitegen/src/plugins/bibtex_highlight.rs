//! Syntax-highlights BibTeX entries at build time: the page carries coloured
//! spans and no script.

use super::BuildPlugin;
use crate::publications::PublicationView;

pub struct BibtexHighlight;

impl BuildPlugin for BibtexHighlight {
    fn publication(&self, item: &mut PublicationView) {
        item.bibtex_html = item.bibtex.as_deref().map(highlight);
    }
}

/// A BibTeX entry as HTML, its parts wrapped in spans for the stylesheet to
/// colour: the entry type and key, the field names, the values, and the
/// punctuation between them. The text itself is unchanged, so what is copied
/// is exactly the entry. Anything the tokenizer does not recognise is passed
/// through, escaped, uncoloured, so a hand-written entry never breaks.
pub fn highlight(src: &str) -> String {
    fn esc(s: &str) -> String {
        s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }
    fn span(out: &mut String, class: &str, text: &str) {
        if !text.is_empty() {
            out.push_str(&format!("<span class=\"bib-{class}\">{}</span>", esc(text)));
        }
    }
    let ident = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | ':' | '.' | '+' | '/');

    let chars: Vec<char> = src.chars().collect();
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    let mut out = String::with_capacity(src.len() * 2);
    let mut i = 0;
    // Before the entry's opening brace; then expecting its key; then its body.
    #[derive(PartialEq)]
    enum At { Start, Key, Body }
    let mut at = At::Start;

    while i < chars.len() {
        let c = chars[i];
        match at {
            At::Start if c == '@' => {
                let start = i;
                i += 1;
                while i < chars.len() && ident(chars[i]) { i += 1; }
                span(&mut out, "type", &text(start, i));
                continue;
            }
            At::Start if c == '{' || c == '(' => {
                span(&mut out, "punct", &c.to_string());
                at = At::Key;
            }
            At::Key if !c.is_whitespace() && c != ',' => {
                let start = i;
                while i < chars.len() && chars[i] != ',' && chars[i] != '\n' { i += 1; }
                span(&mut out, "key", &text(start, i));
                at = At::Body;
                continue;
            }
            At::Body if ident(c) && !c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && ident(chars[i]) { i += 1; }
                span(&mut out, "field", &text(start, i));
                continue;
            }
            At::Body if c == '=' => {
                span(&mut out, "punct", "=");
                i += 1;
                while i < chars.len() && chars[i].is_whitespace() { out.push(chars[i]); i += 1; }
                // The value: braced, quoted, or a bare word or number.
                if i < chars.len() && (chars[i] == '{' || chars[i] == '"') {
                    let close = if chars[i] == '{' { '}' } else { '"' };
                    span(&mut out, "punct", &chars[i].to_string());
                    let start = i + 1;
                    let mut depth = 0i32;
                    let mut j = start;
                    while j < chars.len() {
                        match chars[j] {
                            '{' => depth += 1,
                            '}' if depth > 0 => depth -= 1,
                            ch if ch == close && depth == 0 => break,
                            _ => {}
                        }
                        j += 1;
                    }
                    span(&mut out, "value", &text(start, j.min(chars.len())));
                    if j < chars.len() {
                        span(&mut out, "punct", &chars[j].to_string());
                        j += 1;
                    }
                    i = j;
                } else {
                    let start = i;
                    while i < chars.len() && !matches!(chars[i], ',' | '}' | '\n') { i += 1; }
                    span(&mut out, "value", &text(start, i));
                }
                continue;
            }
            At::Body if c == ',' || c == '}' || c == ')' => span(&mut out, "punct", &c.to_string()),
            At::Key if c == ',' => span(&mut out, "punct", ","),
            _ => out.push_str(&esc(&c.to_string())),
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::highlight;

    #[test]
    fn highlighting_keeps_the_text_and_marks_its_parts() {
        let bib = "@misc{ABCD26,\n  title = {{A & B}},\n  year  = 2026,\n}";
        let html = highlight(bib);
        assert!(html.contains(r#"<span class="bib-type">@misc</span>"#), "got {html}");
        assert!(html.contains(r#"<span class="bib-key">ABCD26</span>"#), "got {html}");
        assert!(html.contains(r#"<span class="bib-field">title</span>"#), "got {html}");
        assert!(html.contains(r#"<span class="bib-value">{A &amp; B}</span>"#), "got {html}");
        assert!(html.contains(r#"<span class="bib-value">2026</span>"#), "got {html}");
        // Stripped of its tags and unescaped, the HTML is the entry itself.
        let mut plain = String::new();
        let mut in_tag = false;
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => plain.push(ch),
                _ => {}
            }
        }
        assert_eq!(plain.replace("&amp;", "&"), bib);
    }
}
