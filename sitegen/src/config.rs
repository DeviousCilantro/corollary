//! The shape of `content/site.toml`.
//!
//! Every struct here mirrors one table of that file. Unknown keys are rejected
//! (`deny_unknown_fields`), so a misspelt field is an error at build time
//! rather than a line silently missing from the page. The one deliberate
//! exception is `[[sections]]`, which accepts any extra keys and hands them to
//! its template untouched — that is what lets a new kind of section be added
//! without touching Rust.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteFile {
    pub site: SiteMeta,
    pub person: Person,
    #[serde(default)]
    pub about: About,
    #[serde(default)]
    pub conventions: Conventions,
    #[serde(default)]
    pub bibtex: BibtexConfig,
    #[serde(default)]
    pub labels: Labels,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub features: Features,
    #[serde(default)]
    pub people: BTreeMap<String, String>,
    /// Which sections appear, in what order. Empty means [`DEFAULT_SECTIONS`].
    #[serde(default)]
    pub sections: Vec<SectionConfig>,
    #[serde(default)]
    pub publications: Vec<PublicationInput>,
    #[serde(default)]
    pub talks: Vec<Talk>,
    #[serde(default)]
    pub teaching: Vec<Teaching>,
    #[serde(default)]
    pub service: Vec<Service>,
}

/// The page used when `[[sections]]` is absent: the introduction, then every
/// built-in list. A section with nothing in it is dropped from the page.
pub const DEFAULT_SECTIONS: [&str; 6] =
    ["about", "publications", "manuscripts", "talks", "teaching", "service"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteMeta {
    /// Where the site is served from, e.g. `https://jane.example.org` or
    /// `https://jane.github.io/homepage`. Optional: see [`resolve_base_url`].
    pub base_url: Option<String>,
    pub title: String,
    pub description: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Markdown appended to the footer, e.g. a "last updated" line.
    #[serde(default)]
    pub footer: String,
    /// The "Built with Corollary" line in the footer. Optional, and appreciated.
    #[serde(default = "yes")]
    pub credit: bool,
}

fn default_lang() -> String {
    "en".to_string()
}

fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    pub name: String,
    pub role: Option<String>,
    pub group: Option<String>,
    pub group_url: Option<String>,
    pub institution: Option<String>,
    pub institution_url: Option<String>,
    /// Optional, and unused by the default templates: the masthead names the
    /// institution instead, which is what an academic reader is after.
    pub location: Option<String>,
    pub email: Option<String>,
    pub photo_path: Option<String>,
    pub photo_alt: Option<String>,
    /// An optional caption under the portrait, e.g. a photo credit.
    #[serde(default)]
    pub photo_caption: String,
    /// Scholar, DBLP, ORCID, GitHub, a CV — listed under the contact line, and
    /// declared to search engines as the same person.
    #[serde(default)]
    pub links: Vec<Link>,
    /// Read from the file itself at build time, never written in the TOML, so
    /// the markup cannot drift out of step with the image it describes.
    #[serde(skip_deserializing, default)]
    pub photo_width: Option<u32>,
    #[serde(skip_deserializing, default)]
    pub photo_height: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct About {
    #[serde(default)]
    pub intro: String,
    /// Structured data only; never shown on the page.
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conventions {
    /// Printed once above the paper lists, and only while the claim is true.
    /// Empty by default: the note is opt-in.
    #[serde(default)]
    pub author_order: String,
    /// Set your own name in the author lists in bold. Off by default.
    #[serde(default)]
    pub highlight_self: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BibtexConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub key_style: KeyStyle,
}

impl Default for BibtexConfig {
    fn default() -> Self {
        Self { enabled: true, key_style: KeyStyle::default() }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyStyle {
    /// Surname initials and a two-digit year, e.g. `ABCD26`.
    #[default]
    Initials,
    /// First author's surname and the full year, e.g. `doe2026`.
    SurnameYear,
}

/// Words the generator itself writes into the page. Everything else is in the
/// templates, where it can be edited directly.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Labels {
    pub preprint: String,
    pub submission: String,
    pub in_preparation: String,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            preprint: "Preprint".into(),
            submission: "In submission".into(),
            in_preparation: "In preparation".into(),
        }
    }
}

/// CSS custom-property overrides, e.g. `light = { accent = "#8a1c1c" }` sets
/// `--accent` in the light theme. Any property in `theme.css` can be named.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[serde(default)]
    pub light: BTreeMap<String, String>,
    #[serde(default)]
    pub dark: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Features {
    pub theme_toggle: bool,
}

impl Default for Features {
    fn default() -> Self {
        Self { theme_toggle: true }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub url: String,
}

/// One entry of `[[sections]]`. `kind` picks `templates/sections/<kind>.html`;
/// every other key is passed to that template as `section.<key>`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SectionConfig {
    pub kind: String,
    pub id: Option<String>,
    pub title: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

impl SectionConfig {
    pub fn of_kind(kind: &str) -> Self {
        Self { kind: kind.to_string(), id: None, title: None, extra: BTreeMap::new() }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationInput {
    pub title: String,
    pub authors: Vec<String>,
    pub year: i64,
    #[serde(default = "default_status")]
    pub status: String,
    pub venue: Option<String>,
    pub venue_url: Option<String>,
    pub note: Option<String>,
    pub eprint: Option<String>,
    pub arxiv: Option<String>,
    pub doi: Option<String>,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    /// Overrides the generated citation key.
    pub bibtex_key: Option<String>,
    /// A complete BibTeX entry, used verbatim instead of the generated one.
    pub bibtex: Option<String>,
}

fn default_status() -> String {
    "submission".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Talk {
    pub title: String,
    pub venue: String,
    pub venue_url: Option<String>,
    pub date: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Teaching {
    pub role: String,
    pub institution: String,
    pub institution_url: Option<String>,
    pub term: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub role: String,
    pub venues: Vec<ServiceVenue>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceVenue {
    pub name: String,
    /// `year = 2026` and `year = "2026"` are both accepted.
    #[serde(default, deserialize_with = "string_or_int")]
    pub year: Option<String>,
    pub url: Option<String>,
}

fn string_or_int<'de, D: Deserializer<'de>>(de: D) -> std::result::Result<Option<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Text(String),
        Number(i64),
    }
    Ok(Option::<Raw>::deserialize(de)?.map(|raw| match raw {
        Raw::Text(text) => text,
        Raw::Number(n) => n.to_string(),
    }))
}

pub fn load(root: &Path) -> Result<SiteFile> {
    let path = root.join("content/site.toml");
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

/// Where the site lives, in order of precedence: `--base-url` on the command
/// line (used by the preview script), `[site] base_url`, the `SITE_BASE_URL`
/// environment variable (set by the GitHub Pages workflow), and finally the
/// local preview address.
pub fn resolve_base_url(cli: Option<String>, configured: Option<&str>) -> Result<String> {
    let chosen = cli
        .or_else(|| configured.map(str::to_string))
        .or_else(|| std::env::var("SITE_BASE_URL").ok())
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| "http://localhost:8000".to_string());

    if !(chosen.starts_with("https://") || chosen.starts_with("http://")) {
        bail!("base_url must start with https:// or http://, got {chosen:?}");
    }
    Ok(chosen.trim_end_matches('/').to_string())
}

/// The path component of the base URL: empty for a site at the root of its
/// domain, `/homepage` for one served from `https://jane.github.io/homepage`.
/// Every root-relative link on the page is prefixed with it.
pub fn base_path(base_url: &str) -> String {
    let after_scheme = base_url.split_once("://").map_or(base_url, |(_, rest)| rest);
    after_scheme
        .find('/')
        .map(|i| after_scheme[i..].trim_end_matches('/').to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_path_is_empty_at_a_domain_root() {
        assert_eq!(base_path("https://example.org"), "");
        assert_eq!(base_path("https://example.org/"), "");
    }

    #[test]
    fn base_path_carries_a_project_page_prefix() {
        assert_eq!(base_path("https://jane.github.io/homepage"), "/homepage");
        assert_eq!(base_path("https://jane.github.io/a/b/"), "/a/b");
    }

    #[test]
    fn command_line_base_url_wins() {
        let url = resolve_base_url(Some("http://localhost:9000/".into()), Some("https://x.org"));
        assert_eq!(url.unwrap(), "http://localhost:9000");
    }

    #[test]
    fn rejects_a_base_url_without_a_scheme() {
        assert!(resolve_base_url(None, Some("example.org")).is_err());
    }

    #[test]
    fn service_years_may_be_numbers_or_strings() {
        let parsed: Service = toml::from_str(
            r#"
            role = "Reviewer"
            venues = [{ name = "A", year = 2026 }, { name = "B", year = "2025" }, { name = "C" }]
            "#,
        )
        .unwrap();
        let years: Vec<_> = parsed.venues.iter().map(|v| v.year.clone()).collect();
        assert_eq!(years, [Some("2026".into()), Some("2025".into()), None]);
    }

    #[test]
    fn rejects_misspelt_fields() {
        let parsed = toml::from_str::<Talk>(r#"title = "T"
venue = "V"
date = "D"
slides = "x""#);
        assert!(parsed.is_err());
    }

    #[test]
    fn sections_keep_unknown_keys_for_their_template() {
        let parsed: SectionConfig =
            toml::from_str("kind = \"list\"\ntitle = \"Awards\"\ncolumns = 2").unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Awards"));
        assert_eq!(parsed.extra.get("columns").and_then(toml::Value::as_integer), Some(2));
    }
}
