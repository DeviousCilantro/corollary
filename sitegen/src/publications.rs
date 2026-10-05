//! Publications: status, ordering, author links, and the view the templates see.

use crate::bibtex;
use crate::config::{BibtexConfig, Labels, Link, PublicationInput};
use crate::markdown;
use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Accepted,
    Preprint,
    Submission,
    InPreparation,
}

impl Status {
    /// Forgiving about spelling: `in preparation`, `in-preparation` and
    /// `draft` all mean the same thing.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().replace([' ', '-', '_'], "").as_str() {
            "accepted" | "published" => Some(Self::Accepted),
            "preprint" | "eprint" => Some(Self::Preprint),
            "submission" | "insubmission" | "submitted" => Some(Self::Submission),
            "inpreparation" | "preparation" | "draft" => Some(Self::InPreparation),
            _ => None,
        }
    }

    /// Accepted or publicly posted, as opposed to a manuscript still in flight.
    pub fn is_published(self) -> bool {
        matches!(self, Self::Accepted | Self::Preprint)
    }
}

#[derive(Debug, Serialize)]
pub struct PublicationView {
    /// Stable slug, used for the anchor and to tie the BibTeX block to its button.
    pub id: String,
    pub title_html: String,
    pub authors: Vec<AuthorView>,
    /// "CRYPTO 2026", "Preprint, 2026", "In submission, 2025", …
    pub venue_primary: String,
    pub venue_url: Option<String>,
    /// Parenthetical qualifier such as "to appear".
    pub venue_note: Option<String>,
    pub status: &'static str,
    pub year: i64,
    pub links: Vec<Link>,
    pub bibtex: Option<String>,
    pub abstract_html: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthorView {
    pub name: String,
    /// `None` for the site owner: a self-link on one's own homepage is noise.
    pub url: Option<String>,
    pub is_self: bool,
}

pub struct Sorted {
    pub published: Vec<PublicationView>,
    pub manuscripts: Vec<PublicationView>,
    /// True when every multi-author paper lists its authors alphabetically, so
    /// the page can state the convention once instead of annotating each entry.
    pub authors_alphabetical: bool,
}

pub struct Settings<'a> {
    pub people: &'a BTreeMap<String, String>,
    pub owner: &'a str,
    pub bibtex: &'a BibtexConfig,
    pub labels: &'a Labels,
    pub base_path: &'a str,
}

/// Newest first; within a year, the most firmly published entry leads. Doing
/// this here means `content/site.toml` never has to be kept in order by hand.
pub fn sort(inputs: &mut [PublicationInput]) -> Result<()> {
    for input in inputs.iter() {
        if Status::parse(&input.status).is_none() {
            bail!(
                "publication {:?} has unrecognised status {:?}; use accepted, preprint, submission or inpreparation",
                input.title,
                input.status
            );
        }
    }
    inputs.sort_by(|a, b| {
        b.year
            .cmp(&a.year)
            .then_with(|| Status::parse(&a.status).cmp(&Status::parse(&b.status)))
            .then_with(|| a.title.cmp(&b.title))
    });
    Ok(())
}

/// Splits sorted input into the Publications and Manuscripts lists, so a
/// manuscript under submission is never mistaken for a paper of record.
pub fn build(ordered: &[PublicationInput], settings: &Settings) -> Result<Sorted> {
    let authors_alphabetical = ordered
        .iter()
        .filter(|p| p.authors.len() > 1)
        .all(|p| is_alphabetical(&p.authors));

    let mut used_keys = HashSet::new();
    let mut used_ids = HashSet::new();
    let mut published = Vec::new();
    let mut manuscripts = Vec::new();
    for input in ordered {
        let mut view = build_one(input, settings, &mut used_keys)?;
        // A conference and a journal version often share a title. Their anchors
        // must not, or the second "copy" button would copy the first entry.
        let base = view.id.clone();
        let mut n = 2;
        while !used_ids.insert(view.id.clone()) {
            view.id = format!("{base}-{n}");
            n += 1;
        }
        if Status::parse(&input.status).is_some_and(Status::is_published) {
            published.push(view);
        } else {
            manuscripts.push(view);
        }
    }
    Ok(Sorted { published, manuscripts, authors_alphabetical })
}

fn build_one(
    input: &PublicationInput,
    settings: &Settings,
    used_keys: &mut HashSet<String>,
) -> Result<PublicationView> {
    let Some(status) = Status::parse(&input.status) else {
        bail!("publication {:?} has unrecognised status {:?}", input.title, input.status);
    };
    if status == Status::Accepted && input.venue.is_none() {
        bail!("publication {:?} is marked accepted but has no venue", input.title);
    }

    let authors = input
        .authors
        .iter()
        .map(|name| {
            let is_self = name == settings.owner;
            AuthorView {
                // A name never breaks across two lines of the author list.
                name: name.replace(' ', "\u{a0}"),
                url: if is_self { None } else { settings.people.get(name).cloned() },
                is_self,
            }
        })
        .collect();

    let labels = settings.labels;
    let year = input.year;
    // A non-breaking space keeps a venue and its year on one line.
    let venue_primary = match status {
        Status::Accepted => format!("{}\u{a0}{year}", input.venue.as_deref().unwrap_or_default()),
        Status::Preprint => format!("{},\u{a0}{year}", labels.preprint),
        Status::Submission => format!("{},\u{a0}{year}", labels.submission),
        Status::InPreparation => labels.in_preparation.clone(),
    };

    let mut links = Vec::new();
    if let Some(id) = normalised_eprint(input.eprint.as_deref()) {
        links.push(Link { label: "ePrint".into(), url: format!("https://eprint.iacr.org/{id}") });
    }
    if let Some(id) = input.arxiv.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        links.push(Link { label: "arXiv".into(), url: format!("https://arxiv.org/abs/{id}") });
    }
    if let Some(doi) = input.doi.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        links.push(Link { label: "DOI".into(), url: format!("https://doi.org/{doi}") });
    }
    links.extend(input.links.iter().cloned());

    let bibtex = if settings.bibtex.enabled {
        let key = bibtex::unique_key(input, settings.bibtex.key_style, used_keys);
        bibtex::entry(input, status, &key)
    } else {
        None
    };

    Ok(PublicationView {
        id: slug(&input.title),
        title_html: markdown::to_inline_html(&input.title, settings.base_path),
        authors,
        venue_primary,
        venue_url: input.venue_url.clone(),
        venue_note: input.note.clone(),
        status: match status {
            Status::Accepted => "accepted",
            Status::Preprint => "preprint",
            Status::Submission => "submission",
            Status::InPreparation => "inpreparation",
        },
        year,
        links,
        bibtex,
        abstract_html: input.abstract_text.as_deref().map(|a| markdown::to_html(a, settings.base_path)),
    })
}

/// Surname-alphabetical check, used only to decide whether to print the
/// standing note about author ordering.
pub fn is_alphabetical(authors: &[String]) -> bool {
    let keys: Vec<String> = authors.iter().map(|a| surname(a).to_lowercase()).collect();
    keys.windows(2).all(|w| w[0] <= w[1])
}

/// The last word of a name. Wrong for "van der Berg"; set `bibtex_key` on the
/// paper if the generated key comes out wrong.
pub fn surname(full_name: &str) -> &str {
    full_name.trim().rsplit(' ').next().unwrap_or(full_name)
}

/// Accepts "2026/0123" or the full ePrint URL, and yields the bare identifier.
pub fn normalised_eprint(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().trim_end_matches('/');
    if value.is_empty() {
        return None;
    }
    Some(value.rsplit("eprint.iacr.org/").next().unwrap_or(value).trim_matches('/').to_string())
}

pub fn slug(title: &str) -> String {
    const LIMIT: usize = 48;

    let mut out = String::new();
    let mut prev_dash = true;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    let trimmed = out.trim_matches('-');
    if trimmed.len() <= LIMIT {
        return trimmed.to_string();
    }
    // Cut at the last word boundary inside the limit, so anchors do not end in
    // half a word.
    let head = &trimmed[..LIMIT];
    head.rfind('-').map_or(head, |cut| &head[..cut]).trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_newest_and_most_published_first() {
        let mut inputs = vec![
            PublicationInput { title: "Old".into(), year: 2024, status: "accepted".into(), venue: Some("V".into()), ..Default::default() },
            PublicationInput { title: "Draft".into(), year: 2026, status: "in preparation".into(), ..Default::default() },
            PublicationInput { title: "New".into(), year: 2026, status: "accepted".into(), venue: Some("V".into()), ..Default::default() },
            PublicationInput { title: "Posted".into(), year: 2026, status: "preprint".into(), ..Default::default() },
        ];
        sort(&mut inputs).unwrap();
        let titles: Vec<_> = inputs.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, ["New", "Posted", "Draft", "Old"]);
    }

    #[test]
    fn papers_sharing_a_title_get_distinct_anchors() {
        let settings_people = BTreeMap::new();
        let bibtex = BibtexConfig::default();
        let labels = Labels::default();
        let settings = Settings { people: &settings_people, owner: "", bibtex: &bibtex, labels: &labels, base_path: "" };
        let paper = |year| PublicationInput { title: "Same Title".into(), authors: vec!["A B".into()], year, status: "preprint".into(), ..Default::default() };
        let sorted = build(&[paper(2026), paper(2024)], &settings).unwrap();
        let ids: Vec<_> = sorted.published.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["same-title", "same-title-2"]);
    }

    #[test]
    fn rejects_an_unknown_status() {
        let mut inputs = vec![PublicationInput { title: "X".into(), status: "maybe".into(), ..Default::default() }];
        assert!(sort(&mut inputs).is_err());
    }

    #[test]
    fn separates_published_work_from_manuscripts() {
        assert!(Status::parse("accepted").unwrap().is_published());
        assert!(Status::parse("preprint").unwrap().is_published());
        assert!(!Status::parse("submission").unwrap().is_published());
        assert!(!Status::parse("in preparation").unwrap().is_published());
    }

    #[test]
    fn detects_alphabetical_author_order() {
        let ordered = ["Alice Archer", "Bob Baird", "Carol Chen", "Dave Dunmore"].map(String::from);
        assert!(is_alphabetical(&ordered));
        let unordered = ["Dave Dunmore", "Alice Archer"].map(String::from);
        assert!(!is_alphabetical(&unordered));
    }

    #[test]
    fn accepts_bare_and_full_eprint_identifiers() {
        assert_eq!(normalised_eprint(Some("2026/0123")).unwrap(), "2026/0123");
        assert_eq!(normalised_eprint(Some("https://eprint.iacr.org/2026/0123")).unwrap(), "2026/0123");
        assert!(normalised_eprint(Some("  ")).is_none());
        assert!(normalised_eprint(None).is_none());
    }

    #[test]
    fn slugs_are_anchor_safe() {
        let generated = slug("Short Signatures from Imaginary Assumptions in the Random Oracle Model");
        assert!(generated.starts_with("short-signatures-from-imaginary-assumptions"));
        assert!(generated.len() <= 48);
        assert!(generated.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        assert!(!generated.starts_with('-') && !generated.ends_with('-'));
    }

    #[test]
    fn slugs_collapse_runs_of_punctuation() {
        assert_eq!(slug("  A -- B: C!  "), "a-b-c");
    }
}
