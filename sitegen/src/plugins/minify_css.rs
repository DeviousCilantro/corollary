//! Strips comments and spare whitespace from the stylesheets the site ships.
//!
//! The sources in `static/assets/css/` keep their comments, which document the
//! design; readers download only the rules. Deliberately conservative: it
//! removes comments, collapses runs of whitespace, and drops the space beside
//! `{ } ; ,`, never touching a quoted string (a `data:` URL, say) or the space
//! a selector or `calc()` depends on.

use super::BuildPlugin;
use std::path::Path;

pub struct MinifyCss;

impl BuildPlugin for MinifyCss {
    fn asset(&self, path: &Path, contents: &[u8]) -> Option<Vec<u8>> {
        if path.extension()? != "css" {
            return None;
        }
        let css = std::str::from_utf8(contents).ok()?;
        Some(minify(css).into_bytes())
    }
}

pub fn minify(css: &str) -> String {
    let chars: Vec<char> = css.chars().collect();
    let mut out = String::with_capacity(css.len());
    let mut pending_space = false;
    let mut i = 0;

    let tight = |c: char| matches!(c, '{' | '}' | ';' | ',');

    while i < chars.len() {
        let c = chars[i];

        if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i += 2;
            pending_space = true;
            continue;
        }

        if c.is_whitespace() {
            pending_space = true;
            i += 1;
            continue;
        }

        if pending_space {
            let after_tight = out.chars().last().map_or(true, tight);
            if !after_tight && !tight(c) {
                out.push(' ');
            }
            pending_space = false;
        }

        if c == '"' || c == '\'' {
            // Copied verbatim, escapes and all, to the matching quote.
            out.push(c);
            i += 1;
            while i < chars.len() {
                let d = chars[i];
                out.push(d);
                i += 1;
                if d == '\\' {
                    if let Some(&e) = chars.get(i) {
                        out.push(e);
                        i += 1;
                    }
                } else if d == c {
                    break;
                }
            }
            continue;
        }

        out.push(c);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::minify;

    #[test]
    fn removes_comments_and_spare_space_but_keeps_meaning() {
        let css = "/* note */\n.a  > .b ,\n.c:hover {\n  color : red ; /* x */\n  width: calc(100% - 2rem);\n}\n";
        assert_eq!(minify(css), ".a > .b,.c:hover{color : red;width: calc(100% - 2rem);}");
    }

    #[test]
    fn leaves_quoted_strings_alone() {
        let css = r#"a { --i: url("data:image/svg+xml,%3Csvg  /* not a comment */ %3E"); content: ' ; { '; }"#;
        let out = minify(css);
        assert!(out.contains(r#"url("data:image/svg+xml,%3Csvg  /* not a comment */ %3E")"#), "{out}");
        assert!(out.contains("' ; { '"), "{out}");
    }

    #[test]
    fn keeps_media_queries_and_descendant_selectors() {
        assert_eq!(minify("@media (max-width: 34rem) {\n  .x .y { a: b }\n}"), "@media (max-width: 34rem){.x .y{a: b}}");
    }
}
