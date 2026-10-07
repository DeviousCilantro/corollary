//! Templates: Tera setup, the section pipeline, and writing pages to disk.
//!
//! The page is assembled from sections. Each `[[sections]]` entry in
//! `content/site.toml` names a `kind`, and is rendered by
//! `templates/sections/<kind>.html` with two variables in scope:
//!
//! - `data`    — everything on the site (see [`RenderData`])
//! - `section` — that entry's own keys: `kind`, `id`, `title`, and any others
//!
//! `templates/index.html` then wraps each one in a `<section>` with its
//! heading. A section whose template renders nothing is left off the page, so
//! an empty list never leaves an orphaned heading behind.

use crate::config::{Features, Person, SectionConfig, Service, Talk, Teaching, Theme};
use crate::markdown;
use crate::publications::{slug, PublicationView};
use anyhow::{bail, Context as AnyhowContext, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use tera::{Context, Tera, Value};
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub struct SiteView {
    /// Absolute, without a trailing slash: `https://jane.github.io/homepage`.
    pub base_url: String,
    /// The path part of `base_url`, or empty: `/homepage`.
    pub base_path: String,
    pub title: String,
    pub description: String,
    pub lang: String,
    pub footer_html: String,
    pub credit: bool,
}

/// What the templates see as `data`.
#[derive(Debug, Serialize)]
pub struct RenderData {
    pub site: SiteView,
    pub person: Person,
    pub intro_html: String,
    /// Accepted and posted work.
    pub publications: Vec<PublicationView>,
    /// Work under submission or in preparation.
    pub manuscripts: Vec<PublicationView>,
    pub authors_alphabetical: bool,
    pub author_order_note: String,
    pub highlight_self: bool,
    pub talks: Vec<Talk>,
    pub teaching: Vec<Teaching>,
    pub service: Vec<Service>,
    pub features: Features,
    /// Inline `<style>` body built from `[theme]`; empty when nothing is set.
    pub theme_css: String,
    /// Whether `static/assets/css/custom.css` exists and should be linked.
    pub has_custom_css: bool,
    pub json_ld: String,
}

#[derive(Debug, Serialize)]
pub struct BuildInfo {
    pub year: i32,
}

#[derive(Debug, Serialize)]
struct SectionView<'a> {
    kind: &'a str,
    id: String,
    title: String,
    #[serde(flatten)]
    extra: &'a BTreeMap<String, toml::Value>,
}

#[derive(Debug, Serialize)]
pub struct RenderedSection {
    id: String,
    kind: String,
    title: String,
    html: String,
}

/// `public_dir` must already hold the copied `static/` files, which the `asset`
/// filter fingerprints.
pub fn tera(root: &Path, base_path: &str, public_dir: &Path) -> Result<Tera> {
    let glob = root.join("templates/**/*.html");
    let mut tera = Tera::new(glob.to_str().context("template path is not UTF-8")?)?;

    let bp = base_path.to_string();
    tera.register_filter("markdown", move |value: &Value, _: &HashMap<String, Value>| {
        Ok(Value::String(markdown::to_html(value.as_str().unwrap_or_default(), &bp)))
    });
    let bp = base_path.to_string();
    tera.register_filter("inline_markdown", move |value: &Value, _: &HashMap<String, Value>| {
        Ok(Value::String(markdown::to_inline_html(value.as_str().unwrap_or_default(), &bp)))
    });
    // `{{ "/assets/x.pdf" | url }}` — every root-relative link in a template goes
    // through this, so the site works from a subpath such as a GitHub project page.
    let bp = base_path.to_string();
    tera.register_filter("url", move |value: &Value, _: &HashMap<String, Value>| {
        Ok(Value::String(markdown::site_url(value.as_str().unwrap_or_default(), &bp)))
    });
    // `{{ "/assets/css/site.css" | asset }}` — `url`, plus a fingerprint of the
    // file's contents. A browser caches a stylesheet for some minutes, and a new
    // page arriving with the old stylesheet would be laid out by rules written
    // for different markup; a fingerprint changes the URL whenever the file
    // changes, so a page only ever loads the files it was built with.
    let bp = base_path.to_string();
    let public = public_dir.to_path_buf();
    tera.register_filter("asset", move |value: &Value, _: &HashMap<String, Value>| {
        let path = value.as_str().unwrap_or_default();
        let url = markdown::site_url(path, &bp);
        Ok(Value::String(match fs::read(public.join(path.trim_start_matches('/'))) {
            Ok(bytes) => format!("{url}?v={}", fingerprint(&bytes)),
            // A missing file is reported by the link check after rendering.
            Err(_) => url,
        }))
    });

    // Tera's stock escaper also encodes '/', which turns every href into
    // "https:&#x2F;&#x2F;…". Browsers decode it, but it makes the served HTML
    // larger and its source unreadable.
    tera.set_escape_fn(escape_html);
    Ok(tera)
}

/// The heading a built-in section gets when `title` is not set. The
/// introduction has none; a custom section has none unless given one.
fn default_title(kind: &str) -> &'static str {
    match kind {
        "publications" => "Publications",
        "manuscripts" => "Manuscripts",
        "talks" => "Talks",
        "teaching" => "Teaching",
        "service" => "Service",
        _ => "",
    }
}

pub fn render_sections(
    tera: &Tera,
    sections: &[SectionConfig],
    data: &RenderData,
    build: &BuildInfo,
) -> Result<Vec<RenderedSection>> {
    let available: Vec<String> = tera
        .get_template_names()
        .filter_map(|name| name.strip_prefix("sections/")?.strip_suffix(".html"))
        .map(str::to_string)
        .collect();

    let mut used_ids = HashSet::new();
    let mut out = Vec::new();
    for config in sections {
        let kind = config.kind.trim();
        if !available.iter().any(|k| k == kind) {
            let mut known = available.clone();
            known.sort();
            bail!(
                "section kind {kind:?} has no template: create templates/sections/{kind}.html, \
                 or use one of: {}",
                known.join(", ")
            );
        }

        // Anchors must be unique on the page; a second "list" becomes "list-2".
        let base_id = slug(config.id.as_deref().unwrap_or(kind));
        let mut id = base_id.clone();
        let mut n = 2;
        while !used_ids.insert(id.clone()) {
            id = format!("{base_id}-{n}");
            n += 1;
        }

        let view = SectionView {
            kind,
            id: id.clone(),
            title: config.title.clone().unwrap_or_else(|| default_title(kind).to_string()),
            extra: &config.extra,
        };

        let mut ctx = Context::new();
        ctx.insert("data", data);
        ctx.insert("build", build);
        ctx.insert("section", &view);
        let template = format!("sections/{kind}.html");
        let html = tera.render(&template, &ctx).with_context(|| format!("rendering {template}"))?;

        if html.trim().is_empty() {
            continue;
        }
        out.push(RenderedSection { id, kind: kind.to_string(), title: view.title, html });
    }
    Ok(out)
}

pub fn render_page(
    tera: &Tera,
    template: &str,
    data: &RenderData,
    build: &BuildInfo,
    sections: &[RenderedSection],
    out_path: &Path,
) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut ctx = Context::new();
    ctx.insert("data", data);
    ctx.insert("build", build);
    ctx.insert("sections", sections);
    let rendered =
        tera.render(template, &ctx).with_context(|| format!("rendering {template}"))?;
    fs::write(out_path, rendered).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// `[theme]` as CSS. Light values go on `:root`; dark values are written twice,
/// mirroring `theme.css`: once for an explicit choice made with the toggle, and
/// once for readers whose system asks for dark and who have not chosen.
pub fn theme_css(theme: &Theme) -> Result<String> {
    fn block(selector: &str, vars: &BTreeMap<String, String>) -> Result<String> {
        let mut out = format!("{selector} {{");
        for (name, value) in vars {
            let name = name.trim_start_matches("--");
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                bail!("[theme] property {name:?} must be letters, digits and dashes");
            }
            if value.contains(['<', '>', '{', '}', ';']) {
                bail!("[theme] value for {name:?} may not contain < > {{ }} or ;");
            }
            out.push_str(&format!(" --{name}: {value};"));
        }
        out.push_str(" }");
        Ok(out)
    }

    let mut css = Vec::new();
    if !theme.light.is_empty() {
        css.push(block(":root", &theme.light)?);
    }
    if !theme.dark.is_empty() {
        css.push(block("html[data-theme=\"dark\"]", &theme.dark)?);
        css.push(format!(
            "@media (prefers-color-scheme: dark) {{ {} }}",
            block("html:not([data-theme=\"light\"])", &theme.dark)?
        ));
    }
    Ok(css.join("\n"))
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(src)?;
        // Notes left for the site's author, not for its readers.
        if rel.file_name().is_some_and(|n| n == "README.md" || n == ".DS_Store") {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)
                .with_context(|| format!("copying {} to {}", path.display(), target.display()))?;
        }
    }
    Ok(())
}

/// Escapes everything that can break out of element text or a quoted attribute
/// value, and nothing else.
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Ten hex digits of the 64-bit FNV-1a hash of `bytes`: stable across builds and
/// platforms, which `std`'s randomly seeded hasher is not, and plenty to tell
/// one version of a stylesheet from the next.
fn fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")[..10].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fingerprint_is_stable_and_follows_the_contents() {
        // The published FNV-1a 64 test vector for "a".
        assert_eq!(fingerprint(b"a"), "af63dc4c86");
        assert_eq!(fingerprint(b"body { }"), fingerprint(b"body { }"));
        assert_ne!(fingerprint(b"body { }"), fingerprint(b"body {}"));
    }

    #[test]
    fn theme_overrides_cover_both_dark_paths() {
        let mut theme = Theme::default();
        theme.light.insert("accent".into(), "#8a1c1c".into());
        theme.dark.insert("--accent".into(), "#e09a9a".into());
        let css = theme_css(&theme).unwrap();
        assert!(css.contains(":root { --accent: #8a1c1c; }"), "got {css}");
        assert!(css.contains("html[data-theme=\"dark\"] { --accent: #e09a9a; }"));
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
    }

    #[test]
    fn theme_values_cannot_escape_the_style_element() {
        let mut theme = Theme::default();
        theme.light.insert("accent".into(), "red</style><script>".into());
        assert!(theme_css(&theme).is_err());
    }

    #[test]
    fn an_empty_theme_writes_nothing() {
        assert_eq!(theme_css(&Theme::default()).unwrap(), "");
    }
}
