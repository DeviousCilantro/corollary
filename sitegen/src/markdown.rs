//! Markdown and mathematics.
//!
//! LaTeX is converted to MathML here, at build time, so the published page
//! needs no maths engine, no CDN request and no JavaScript to show an equation.

use pulldown_cmark::{html, Options, Parser as MdParser};
use pulldown_latex::config::DisplayMode;
use pulldown_latex::{push_mathml, Parser as TexParser, RenderConfig, Storage};

/// Markdown to HTML, with root-relative links prefixed by `base_path` so that
/// `[CV](/assets/files/cv.pdf)` still works on a site served from a subpath.
pub fn to_html(input: &str, base_path: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_STRIKETHROUGH;

    // Maths is lifted out before the Markdown parser sees it: pulldown-cmark
    // would otherwise treat `\(` as an escaped parenthesis and `_` inside an
    // expression as emphasis.
    let (stripped, spans) = extract_math(input);
    let parser = MdParser::new_ext(&stripped, options);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    let html_out = keep_names_together(&reinsert_math(&html_out, &spans));
    prefix_root_relative(&html_out, base_path)
}

/// Joins the words of a linked proper name with non-breaking spaces, so
/// "Bob Baird" or "Example Lab" never splits across two lines. A link text
/// counts as a name when it is two to four words, each starting with a
/// capital ("Carol D. Chen"); "Cryptography and Security" can still wrap.
pub fn keep_names_together(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find("<a ") {
        let Some(tag_end) = rest[open..].find('>').map(|i| open + i + 1) else { break };
        let Some(close) = rest[tag_end..].find("</a>").map(|i| tag_end + i) else { break };
        out.push_str(&rest[..tag_end]);
        let text = &rest[tag_end..close];
        let words: Vec<&str> = text.split(' ').collect();
        let is_name = !text.contains('<')
            && (2..=4).contains(&words.len())
            && words.iter().all(|w| w.chars().next().is_some_and(char::is_uppercase));
        if is_name {
            out.push_str(&words.join("&nbsp;"));
        } else {
            out.push_str(text);
        }
        rest = &rest[close..];
    }
    out.push_str(rest);
    out
}

/// As [`to_html`], without the enclosing paragraph: for titles and one-liners.
pub fn to_inline_html(input: &str, base_path: &str) -> String {
    let rendered = to_html(input, base_path);
    let trimmed = rendered.trim();
    trimmed
        .strip_prefix("<p>")
        .and_then(|s| s.strip_suffix("</p>"))
        .unwrap_or(trimmed)
        .to_string()
}

/// `/assets/x.pdf` -> `/homepage/assets/x.pdf` when the site lives under
/// `/homepage`. Absolute URLs, protocol-relative URLs and anchors pass through.
pub fn site_url(url: &str, base_path: &str) -> String {
    if url.starts_with('/') && !url.starts_with("//") {
        format!("{base_path}{url}")
    } else {
        url.to_string()
    }
}

/// Applies [`site_url`] to every `href="/…"` and `src="/…"` in rendered HTML.
pub fn prefix_root_relative(html: &str, base_path: &str) -> String {
    if base_path.is_empty() {
        return html.to_string();
    }
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    loop {
        let next = ["href=\"/", "src=\"/"]
            .iter()
            .filter_map(|pattern| rest.find(pattern).map(|at| at + pattern.len() - 1))
            .min();
        let Some(slash) = next else {
            out.push_str(rest);
            return out;
        };
        let (head, tail) = rest.split_at(slash);
        out.push_str(head);
        if !tail.starts_with("//") {
            out.push_str(base_path);
        }
        out.push('/');
        rest = &tail[1..];
    }
}

struct MathSpan {
    display: bool,
    tex: String,
}

/// Placeholder made of bare uppercase letters and digits, so no Markdown
/// construct can match inside it and smart punctuation leaves it alone.
fn placeholder(index: usize) -> String {
    format!("XMATHSPAN{index}ENDX")
}

fn extract_math(input: &str) -> (String, Vec<MathSpan>) {
    const DELIMITERS: [(&str, &str, bool); 4] =
        [("\\[", "\\]", true), ("\\(", "\\)", false), ("$$", "$$", true), ("$", "$", false)];

    let mut out = String::with_capacity(input.len());
    let mut spans: Vec<MathSpan> = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let rest = &input[i..];

        // A backslash-escaped dollar is literal text, not a delimiter.
        if rest.starts_with("\\$") {
            out.push_str("\\$");
            i += 2;
            continue;
        }

        let opened = DELIMITERS
            .iter()
            .find(|(open, _, _)| rest.starts_with(open))
            .and_then(|&(open, close, display)| {
                let body_start = i + open.len();
                let end = if open == "$" {
                    // A lone `$` straight after another is the tail of an
                    // unclosed `$$`, not the start of inline maths.
                    if input[..i].ends_with('$') {
                        return None;
                    }
                    closing_dollar(input, body_start)?
                } else {
                    body_start + input[body_start..].find(close)?
                };
                Some((body_start, end, close.len(), display))
            });

        if let Some((start, end, close_len, display)) = opened {
            out.push_str(&placeholder(spans.len()));
            spans.push(MathSpan { display, tex: input[start..end].to_string() });
            i = end + close_len;
            continue;
        }

        let ch_len = rest.chars().next().map_or(1, char::len_utf8);
        out.push_str(&input[i..i + ch_len]);
        i += ch_len;
    }

    (out, spans)
}

/// Where a single-dollar span opened at `body_start` closes, if it does.
///
/// Pandoc's rule, so that prices are not typeset: the opening `$` must be
/// followed by a non-space, and the closing `$` preceded by a non-space and not
/// followed by a digit. "costs $5 and $10" is text; "$x$" is maths. Inline
/// maths never crosses a blank line, and a `$` that belongs to a `$$` never
/// closes it.
fn closing_dollar(input: &str, body_start: usize) -> Option<usize> {
    let body = &input[body_start..];
    if body.chars().next().is_none_or(|c| c.is_whitespace() || c == '$') {
        return None;
    }
    let paragraph_end = body.find("\n\n").map_or(input.len(), |rel| body_start + rel);
    body.match_indices('$').map(|(rel, _)| body_start + rel).take_while(|&end| end < paragraph_end).find(|&end| {
        let before = input[..end].chars().next_back();
        let after = input[end + 1..].chars().next();
        end > body_start
            && before.is_some_and(|c| !c.is_whitespace() && c != '\\' && c != '$')
            && !after.is_some_and(|c| c.is_ascii_digit() || c == '$')
    })
}

fn reinsert_math(html_input: &str, spans: &[MathSpan]) -> String {
    let mut out = html_input.to_string();
    for (index, span) in spans.iter().enumerate() {
        out = out.replace(&placeholder(index), &render_math(span));
    }
    out
}

fn render_math(span: &MathSpan) -> String {
    let storage = Storage::new();
    let parser = TexParser::new(&span.tex, &storage);
    let config = RenderConfig {
        display_mode: if span.display { DisplayMode::Block } else { DisplayMode::Inline },
        ..Default::default()
    };

    let mut out = String::new();
    // The renderer reports most mistakes not as an error but as an inline
    // <merror> box carrying its message, which would print a parser complaint
    // on the page. Either way the reader is shown the TeX as written, and the
    // author a warning at build time.
    let failure = match push_mathml(&mut out, parser, config) {
        Ok(()) if out.contains("<merror") => Some("the expression did not parse".to_string()),
        Ok(()) => None,
        Err(error) => Some(error.to_string()),
    };
    match failure {
        None => out,
        Some(reason) => {
            eprintln!("warning: could not typeset {:?}: {reason}", span.tex);
            format!("<code class=\"math-error\">{}</code>", html_escape(&span.tex))
        }
    }
}

pub fn html_escape(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_maths_to_mathml_without_a_browser() {
        let html = to_html(r"The modulus is \(N^2\).", "");
        assert!(html.contains("<math"), "expected MathML, got {html}");
        assert!(html.contains("msup"), "expected a superscript, got {html}");
    }

    #[test]
    fn does_not_mangle_underscores_inside_maths() {
        let html = to_html(r"\(a_1 + a_2\) and _emphasis_", "");
        assert!(html.contains("<em>emphasis</em>"));
        assert!(!html.contains("<em>1 + a</em>"));
    }

    #[test]
    fn leaves_unpaired_delimiters_alone() {
        let html = to_html("costs $5 to run", "");
        assert!(html.contains("$5 to run"), "got {html}");
    }

    #[test]
    fn does_not_typeset_prices() {
        let html = to_html("costs $5 and $10 to run", "");
        assert!(!html.contains("<math"), "got {html}");
        assert!(html.contains("$5 and $10"), "got {html}");
        let html = to_html("where $x$ and $y_1$ are fixed", "");
        assert_eq!(html.matches("<math").count(), 2, "got {html}");
    }

    #[test]
    fn a_price_never_pairs_with_display_maths() {
        let html = to_html("a price of $5 and $10.\n\n$$ x^2 $$", "");
        assert!(html.contains("$5 and $10"), "got {html}");
        assert_eq!(html.matches("<math").count(), 1, "got {html}");
        assert!(html.contains(r#"display="block""#), "got {html}");
    }

    #[test]
    fn inline_maths_does_not_cross_paragraphs() {
        let html = to_html("costs $5\n\nand x$ later", "");
        assert!(!html.contains("<math"), "got {html}");
    }

    #[test]
    fn malformed_maths_shows_the_tex_not_an_error_box() {
        let html = to_html(r"\( \frac{1}{ \)", "");
        assert!(!html.contains("merror"), "got {html}");
        assert!(html.contains("math-error"), "got {html}");
    }

    #[test]
    fn keeps_linked_names_on_one_line() {
        let html = to_html(
            "[Bob Baird](https://a), [Carol D. Chen](https://b), [Example Lab](https://c), \
             [Cryptography and Security](https://d), [slides](https://e)",
            "",
        );
        assert!(html.contains(">Bob&nbsp;Baird</a>"), "got {html}");
        assert!(html.contains(">Carol&nbsp;D.&nbsp;Chen</a>"), "got {html}");
        assert!(html.contains(">Example&nbsp;Lab</a>"), "got {html}");
        assert!(html.contains(">Cryptography and Security</a>"), "got {html}");
        assert!(html.contains(">slides</a>"), "got {html}");
    }

    #[test]
    fn prefixes_root_relative_links_only() {
        let html = to_html(
            "[cv](/assets/cv.pdf), [web](https://example.org/x), [cdn](//cdn.example/y), [top](#top)",
            "/homepage",
        );
        assert!(html.contains(r#"href="/homepage/assets/cv.pdf""#), "got {html}");
        assert!(html.contains(r#"href="https://example.org/x""#));
        assert!(html.contains(r#"href="//cdn.example/y""#));
        assert!(html.contains(r##"href="#top""##));
    }

    #[test]
    fn site_url_respects_the_base_path() {
        assert_eq!(site_url("/", "/homepage"), "/homepage/");
        assert_eq!(site_url("/a.css", ""), "/a.css");
        assert_eq!(site_url("https://x.org/a", "/homepage"), "https://x.org/a");
    }
}
