//! BibTeX entries and citation keys.

use crate::config::{KeyStyle, PublicationInput};
use crate::publications::{normalised_eprint, surname, Status};
use std::collections::HashSet;

/// A citation key, made unique against every key already issued on the page.
pub fn unique_key(
    input: &PublicationInput,
    style: KeyStyle,
    used: &mut HashSet<String>,
) -> String {
    if let Some(key) = input.bibtex_key.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        used.insert(key.to_string());
        return key.to_string();
    }

    let base = match style {
        // Initials of the author surnames plus a two-digit year — the
        // convention used throughout the cryptography literature (`ABCD26`).
        KeyStyle::Initials => {
            let initials: String = input
                .authors
                .iter()
                .filter_map(|a| surname(a).chars().next())
                .flat_map(char::to_uppercase)
                .collect();
            format!("{initials}{:02}", input.year.rem_euclid(100))
        }
        // The first author's surname and the full year (`doe2026`), as most
        // reference managers produce.
        KeyStyle::SurnameYear => {
            let first: String = input
                .authors
                .first()
                .map(|a| surname(a))
                .unwrap_or_default()
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .flat_map(|c| c.to_lowercase())
                .collect();
            let first = if first.is_empty() { "anon".to_string() } else { first };
            format!("{first}{}", input.year)
        }
    };

    let mut candidate = base.clone();
    let mut suffix = b'a';
    while !used.insert(candidate.clone()) {
        candidate = format!("{base}{}", suffix as char);
        suffix += 1;
    }
    candidate
}

pub fn entry(input: &PublicationInput, status: Status, key: &str) -> Option<String> {
    if let Some(verbatim) = input.bibtex.as_deref().map(str::trim).filter(|b| !b.is_empty()) {
        return Some(verbatim.to_string());
    }

    let eprint = normalised_eprint(input.eprint.as_deref());
    let arxiv = input.arxiv.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let doi = input.doi.as_deref().map(str::trim).filter(|s| !s.is_empty());

    // Only emit an entry for something a reader could actually cite. A paper
    // that is merely under submission, with nothing public to point at, gets no
    // BibTeX button rather than an unusable stub.
    if !status.is_published() && eprint.is_none() && arxiv.is_none() && doi.is_none() {
        return None;
    }

    let authors = input.authors.iter().map(|a| tex_escape(a)).collect::<Vec<_>>().join(" and ");
    // Double braces protect the capitalisation of acronyms in the title.
    let title = format!("{{{}}}", tex_escape(&input.title));
    let year = input.year;

    let mut fields: Vec<(&str, String)> = vec![("author", authors), ("title", title)];
    let entry_type;

    if let Some(id) = &eprint {
        entry_type = "misc";
        fields.push(("howpublished", format!("Cryptology {{ePrint}} Archive, Paper {id}")));
        fields.push(("year", year.to_string()));
        fields.push(("url", format!("https://eprint.iacr.org/{id}")));
    } else if let Some(id) = arxiv {
        entry_type = "misc";
        fields.push(("year", year.to_string()));
        fields.push(("eprint", id.to_string()));
        fields.push(("archivePrefix", "arXiv".to_string()));
        fields.push(("url", format!("https://arxiv.org/abs/{id}")));
    } else if status == Status::Accepted {
        entry_type = "inproceedings";
        let venue = input.venue.as_deref().unwrap_or_default();
        fields.push(("booktitle", format!("{} {year}", tex_escape(venue))));
        fields.push(("year", year.to_string()));
        // A paper accepted but not yet in the proceedings has no pages,
        // publisher or DOI to give. Saying so is the convention, and stops the
        // entry from claiming more than it can support.
        if let Some(note) = input.note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            fields.push(("note", sentence_case(&tex_escape(note))));
        }
    } else {
        entry_type = "unpublished";
        fields.push(("year", year.to_string()));
        fields.push(("note", "Manuscript".to_string()));
    }

    if let Some(doi) = doi {
        fields.push(("doi", doi.to_string()));
    }

    let width = fields.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let mut out = format!("@{entry_type}{{{key},\n");
    for (name, value) in fields {
        out.push_str(&format!("  {name:<width$} = {{{value}}},\n"));
    }
    out.push('}');
    Some(out)
}

/// "to appear" -> "To appear". BibTeX note fields read as sentences.
fn sentence_case(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn tex_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '&' | '%' | '#' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paper(authors: &[&str], status: &str) -> PublicationInput {
        PublicationInput {
            title: "Succinct NIZKs From LWE".into(),
            authors: authors.iter().map(|a| a.to_string()).collect(),
            year: 2026,
            status: status.into(),
            venue: Some("EXAMPLECRYPT".into()),
            ..Default::default()
        }
    }

    const FOUR: [&str; 4] = ["Alice Archer", "Bob Baird", "Carol Chen", "Dave Dunmore"];

    #[test]
    fn builds_initials_style_keys() {
        let mut used = HashSet::new();
        let input = paper(&FOUR, "accepted");
        assert_eq!(unique_key(&input, KeyStyle::Initials, &mut used), "ABCD26");
        // A second paper by the same authors in the same year is disambiguated.
        assert_eq!(unique_key(&input, KeyStyle::Initials, &mut used), "ABCD26a");
    }

    #[test]
    fn builds_surname_year_keys() {
        let mut used = HashSet::new();
        let input = paper(&["Alice O'Archer", "Bob Baird"], "accepted");
        assert_eq!(unique_key(&input, KeyStyle::SurnameYear, &mut used), "oarcher2026");
    }

    #[test]
    fn an_explicit_key_wins() {
        let mut used = HashSet::new();
        let input = PublicationInput { bibtex_key: Some("Archer:Crypto26".into()), ..paper(&FOUR, "accepted") };
        assert_eq!(unique_key(&input, KeyStyle::Initials, &mut used), "Archer:Crypto26");
    }

    #[test]
    fn omits_entries_for_uncitable_manuscripts() {
        let input = PublicationInput { venue: None, ..paper(&["Alice Archer"], "submission") };
        assert!(entry(&input, Status::Submission, "A25").is_none());
    }

    #[test]
    fn protects_acronym_capitalisation() {
        let input = paper(&["Alice Archer", "Dave Dunmore"], "accepted");
        let bibtex = entry(&input, Status::Accepted, "AD26").expect("citable");
        assert!(bibtex.contains("title     = {{Succinct NIZKs From LWE}}"), "got {bibtex}");
        assert!(bibtex.contains("booktitle = {EXAMPLECRYPT 2026}"), "got {bibtex}");
        assert!(bibtex.starts_with("@inproceedings{AD26,"));
    }

    #[test]
    fn prefers_the_eprint_entry_once_posted() {
        let input = PublicationInput { eprint: Some("2026/0123".into()), ..paper(&["Alice Archer"], "accepted") };
        let bibtex = entry(&input, Status::Accepted, "A26").expect("citable");
        assert!(bibtex.contains("Cryptology {ePrint} Archive, Paper 2026/0123"), "got {bibtex}");
        assert!(bibtex.contains("https://eprint.iacr.org/2026/0123"));
    }

    #[test]
    fn builds_arxiv_entries() {
        let input = PublicationInput { arxiv: Some("2601.01234".into()), ..paper(&["Alice Archer"], "preprint") };
        let bibtex = entry(&input, Status::Preprint, "A26").expect("citable");
        assert!(bibtex.contains("archivePrefix = {arXiv}"), "got {bibtex}");
    }

    #[test]
    fn marks_accepted_but_unpublished_papers_as_to_appear() {
        let input = PublicationInput { note: Some("to appear".into()), ..paper(&["Alice Archer"], "accepted") };
        let bibtex = entry(&input, Status::Accepted, "A26").expect("citable");
        assert!(bibtex.contains("note      = {To appear}"), "got {bibtex}");
    }

    #[test]
    fn uses_a_verbatim_entry_when_given() {
        let input = PublicationInput { bibtex: Some("  @article{x, title={Y}}  ".into()), ..paper(&["Alice Archer"], "accepted") };
        assert_eq!(entry(&input, Status::Accepted, "A26").unwrap(), "@article{x, title={Y}}");
    }
}
