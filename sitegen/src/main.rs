//! Corollary: a static generator for a theorist's homepage.
//!
//! Reads `content/site.toml`, renders a single page of HTML into `public/`, and
//! copies `static/` alongside it. Everything the browser receives is produced
//! here at build time: Markdown, LaTeX (as MathML), BibTeX entries, author
//! links, publication ordering, structured data and the sitemap. The browser
//! is never asked to render content.
//!
//! Modules, in the order the data flows through them:
//!
//! - [`config`]       the schema of `content/site.toml`
//! - [`publications`] status, ordering and author links
//! - [`bibtex`]       citation keys and entries
//! - [`markdown`]     Markdown, LaTeX -> MathML, and base-path-aware URLs
//! - [`images`]       portrait dimensions
//! - [`seo`]          JSON-LD, sitemap, robots.txt, favicon
//! - [`render`]       Tera, the section pipeline, and file output
//!
//! Usage: `sitegen [--base-url URL]`. See README.md.

mod bibtex;
mod config;
mod images;
mod markdown;
mod publications;
mod render;
mod seo;

use anyhow::{bail, Context as AnyhowContext, Result};
use chrono::Datelike;
use config::{SectionConfig, SiteFile, DEFAULT_SECTIONS};
use render::{BuildInfo, RenderData, SiteView};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let cli_base_url = parse_args()?;
    let root = workspace_root()?;
    let raw = config::load(&root)?;
    let base_url = config::resolve_base_url(cli_base_url, raw.site.base_url.as_deref())?;
    let base_path = config::base_path(&base_url);

    let sections = if raw.sections.is_empty() {
        DEFAULT_SECTIONS.iter().map(|kind| SectionConfig::of_kind(kind)).collect()
    } else {
        raw.sections.clone()
    };
    // The favicon follows any [theme] overrides of the light palette.
    let colour = |name: &str, default: &str| {
        raw.theme.light.get(name).cloned().unwrap_or_else(|| default.to_string())
    };
    let favicon = seo::favicon(
        &raw.person.name,
        &colour("accent", "#2b3c8c"),
        &colour("paper", "#f8f7f2"),
        &colour("highlight", "#ffd43b"),
    );

    let data = build_render_data(raw, &root, base_url, base_path)?;
    let public_dir = root.join("public");

    if public_dir.exists() {
        fs::remove_dir_all(&public_dir)
            .with_context(|| format!("removing {}", public_dir.display()))?;
    }
    fs::create_dir_all(&public_dir)
        .with_context(|| format!("creating {}", public_dir.display()))?;

    render::copy_dir(&root.join("static"), &public_dir)?;
    // Tells GitHub Pages not to run Jekyll over the output.
    fs::write(public_dir.join(".nojekyll"), "")?;
    if root.join("CNAME").exists() {
        fs::copy(root.join("CNAME"), public_dir.join("CNAME"))?;
    }
    // Generated unless you supply your own in static/.
    write_unless_present(&public_dir.join("robots.txt"), &seo::robots(&data.site.base_url))?;
    write_unless_present(&public_dir.join("favicon.svg"), &favicon)?;
    fs::write(public_dir.join("sitemap.xml"), seo::sitemap(&data.site.base_url))?;

    let tera = render::tera(&root, &data.site.base_path, &public_dir)?;
    let build = BuildInfo { year: chrono::Local::now().year() };
    let rendered = render::render_sections(&tera, &sections, &data, &build)?;
    render::render_page(&tera, "index.html", &data, &build, &rendered, &public_dir.join("index.html"))?;
    render::render_page(&tera, "404.html", &data, &build, &[], &public_dir.join("404.html"))?;

    for missing in missing_local_files(&public_dir, &data.site.base_path)? {
        eprintln!("warning: the page links to {missing}, which is not in static/");
    }

    println!(
        "generated {} for {}: {} sections, {} publications, {} manuscripts, {} talks",
        public_dir.display(),
        data.site.base_url,
        rendered.len(),
        data.publications.len(),
        data.manuscripts.len(),
        data.talks.len(),
    );
    Ok(())
}

fn parse_args() -> Result<Option<String>> {
    let mut base_url = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base-url" => base_url = Some(args.next().context("--base-url needs a value")?),
            "-h" | "--help" => {
                println!(
                    "usage: sitegen [--base-url URL]\n\n\
                     Builds content/site.toml into public/.\n\
                     --base-url  overrides [site] base_url and $SITE_BASE_URL (used for local preview)"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument {other:?}; try --help"),
        }
    }
    Ok(base_url)
}

/// The site's root: the working directory when it holds `content/site.toml`
/// (scripts/build.sh always runs from the root), otherwise the workspace the
/// binary was compiled in. Looking at the working directory first keeps a
/// moved or copied checkout from building the site it was compiled for.
fn workspace_root() -> Result<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("content/site.toml").is_file() {
            return Ok(cwd);
        }
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .context("run sitegen from the site's root, the directory holding content/site.toml")
}

/// Every root-relative `href` or `src` on the generated pages that points at no
/// file in the output: a mistyped slides path, a CV not yet copied into
/// static/. Reported as warnings, so a broken link is caught at build time
/// rather than by a reader.
fn missing_local_files(public_dir: &Path, base_path: &str) -> Result<Vec<String>> {
    let mut missing = std::collections::BTreeSet::new();
    for page in ["index.html", "404.html"] {
        let html = fs::read_to_string(public_dir.join(page))?;
        for attr in ["href=\"", "src=\""] {
            for (at, _) in html.match_indices(attr) {
                let value = &html[at + attr.len()..];
                let Some(end) = value.find('"') else { continue };
                let url = &value[..end];
                if !url.starts_with('/') || url.starts_with("//") {
                    continue;
                }
                let path = url.split(['#', '?']).next().unwrap_or(url);
                let local = path.strip_prefix(base_path).unwrap_or(path).trim_start_matches('/');
                let file = public_dir.join(local);
                let found = if local.is_empty() || path.ends_with('/') {
                    file.join("index.html").is_file()
                } else {
                    file.is_file()
                };
                // The WebAssembly module is added after the generator runs.
                if !found && !local.starts_with("assets/wasm/") {
                    missing.insert(path.to_string());
                }
            }
        }
    }
    Ok(missing.into_iter().collect())
}

fn write_unless_present(path: &Path, contents: &str) -> Result<()> {
    if !path.exists() {
        fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// Newest year first, like the publication list. The sort is stable, so venues
/// within one year keep the order they are written in.
fn sort_venues_newest_first(venues: &mut [config::ServiceVenue]) {
    fn year(venue: &config::ServiceVenue) -> Option<i64> {
        let digits: String = venue.year.as_deref()?.chars().filter(char::is_ascii_digit).take(4).collect();
        digits.parse().ok()
    }
    // A venue without a year sorts last.
    venues.sort_by(|a, b| year(b).cmp(&year(a)));
}

fn build_render_data(
    raw: SiteFile,
    root: &Path,
    base_url: String,
    base_path: String,
) -> Result<RenderData> {
    let SiteFile {
        site,
        mut person,
        about,
        conventions,
        bibtex,
        labels,
        theme,
        features,
        people,
        sections: _,
        mut publications,
        talks,
        teaching,
        mut service,
    } = raw;

    for item in &mut service {
        sort_venues_newest_first(&mut item.venues);
    }

    // The portrait's width and height belong in the markup, so the space is
    // reserved before the image arrives and nothing below it jumps. They are
    // measured here rather than written by hand, so a replacement portrait of
    // another size cannot leave stale numbers behind.
    if let Some(path) = &person.photo_path {
        let file = root.join("static").join(path.trim_start_matches('/'));
        if !file.exists() && path.starts_with('/') {
            bail!("[person] photo_path {path:?} does not exist: expected {}", file.display());
        }
        if let Some((w, h)) = fs::read(&file).ok().as_deref().and_then(images::dimensions) {
            person.photo_width = Some(w);
            person.photo_height = Some(h);
        }
    }

    publications::sort(&mut publications)?;
    let sorted = publications::build(
        &publications,
        &publications::Settings {
            people: &people,
            owner: &person.name,
            bibtex: &bibtex,
            labels: &labels,
            base_path: &base_path,
        },
    )?;

    // Structured data describes work that exists publicly; a manuscript under
    // submission is not something to advertise to a search engine.
    let indexable: Vec<_> = publications
        .iter()
        .filter(|p| publications::Status::parse(&p.status).is_some_and(|s| s.is_published()))
        .collect();
    let json_ld = seo::json_ld(&base_url, &person, &about, &indexable);

    Ok(RenderData {
        intro_html: markdown::to_html(&about.intro, &base_path),
        author_order_note: markdown::to_inline_html(&conventions.author_order, &base_path),
        site: SiteView {
            footer_html: markdown::to_inline_html(&site.footer, &base_path),
            title: site.title,
            description: site.description,
            lang: site.lang,
            credit: site.credit,
            base_url,
            base_path,
        },
        person,
        publications: sorted.published,
        manuscripts: sorted.manuscripts,
        authors_alphabetical: sorted.authors_alphabetical,
        highlight_self: conventions.highlight_self,
        talks,
        teaching,
        service,
        features,
        theme_css: render::theme_css(&theme)?,
        has_custom_css: root.join("static/assets/css/custom.css").exists(),
        json_ld,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::ServiceVenue;

    fn venue(name: &str, year: Option<&str>) -> ServiceVenue {
        ServiceVenue { name: name.into(), year: year.map(String::from), url: None }
    }

    #[test]
    fn service_venues_run_newest_first_and_keep_order_within_a_year() {
        let mut venues = vec![
            venue("EUROCRYPT", Some("2026")),
            venue("Untimed", None),
            venue("CRYPTO", Some("2026")),
            venue("FC", Some("2027")),
            venue("TCC", Some("2025")),
        ];
        sort_venues_newest_first(&mut venues);
        let names: Vec<_> = venues.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["FC", "EUROCRYPT", "CRYPTO", "TCC", "Untimed"]);
    }
}
