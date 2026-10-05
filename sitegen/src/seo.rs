//! Everything a crawler reads: structured data, the sitemap, robots.txt, and
//! the generated favicon.

use crate::config::{About, Person, PublicationInput};
use crate::markdown::html_escape;

pub fn json_ld(
    base_url: &str,
    person: &Person,
    about: &About,
    publications: &[&PublicationInput],
) -> String {
    let mut root = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "Person",
        "name": person.name,
        "url": format!("{base_url}/"),
        // The address is deliberately absent. It is on the page as a mailto for
        // a reader who wants it, but a labelled field in a documented schema is
        // the easiest form there is to harvest, and no search feature consumes
        // Person.email for a personal homepage.
    });

    if let Some(role) = &person.role {
        root["jobTitle"] = serde_json::json!(role);
    }
    if let Some(institution) = &person.institution {
        let mut org = serde_json::json!({ "@type": "Organization", "name": institution });
        if let Some(url) = &person.institution_url {
            org["url"] = serde_json::json!(url);
        }
        root["affiliation"] = org;
    }
    if let Some(photo) = &person.photo_path {
        root["image"] = serde_json::json!(absolute(base_url, photo));
    }
    // Scholar, ORCID, DBLP and the like are how a search engine learns that
    // these profiles and this page describe the same person.
    let same_as: Vec<&str> = person
        .links
        .iter()
        .map(|link| link.url.as_str())
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        .collect();
    if !same_as.is_empty() {
        root["sameAs"] = serde_json::json!(same_as);
    }
    if !about.keywords.is_empty() {
        root["knowsAbout"] = serde_json::json!(about.keywords);
    }
    if !publications.is_empty() {
        let works: Vec<_> = publications
            .iter()
            .map(|p| {
                serde_json::json!({
                    "@type": "ScholarlyArticle",
                    "name": p.title,
                    "datePublished": p.year.to_string(),
                    "author": p.authors.iter()
                        .map(|a| serde_json::json!({ "@type": "Person", "name": a }))
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        root["subjectOf"] = serde_json::json!(works);
    }

    // The result is emitted inside a <script> element, so a literal "</" in any
    // field would end the element early. Escaping the slash keeps the JSON valid
    // and the document well formed.
    serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".to_string()).replace("</", "<\\/")
}

/// A root-relative path made absolute against the site's base URL.
pub fn absolute(base_url: &str, path: &str) -> String {
    if path.starts_with('/') && !path.starts_with("//") {
        format!("{base_url}{path}")
    } else {
        path.to_string()
    }
}

pub fn sitemap(base_url: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  <url><loc>{base_url}/</loc></url>\n</urlset>\n"
    )
}

pub fn robots(base_url: &str) -> String {
    format!("User-agent: *\nAllow: /\n\nSitemap: {base_url}/sitemap.xml\n")
}

/// Your initial in ink on paper, with a stroke of highlighter behind its lower
/// half — the same mark the page puts on its links. Used only when
/// `static/favicon.svg` does not exist; drop your own file there to replace it.
pub fn favicon(name: &str, ink: &str, paper: &str, highlight: &str) -> String {
    let initial: String = name.trim().chars().next().map(String::from).unwrap_or_default();
    let clean = |colour: &str| html_escape(colour).replace('"', "");
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" role="img" aria-label="{label}">
  <rect width="64" height="64" rx="12" fill="{paper}"/>
  <rect x="9" y="31" width="46" height="17" rx="2" fill="{highlight}" transform="rotate(-4 32 40)"/>
  <text x="32" y="47" text-anchor="middle"
        font-family="ui-serif, Georgia, 'Times New Roman', serif"
        font-size="40" font-weight="600" fill="{ink}">{initial}</text>
</svg>
"#,
        label = html_escape(name).replace('"', "&quot;"),
        initial = html_escape(&initial),
        ink = clean(ink),
        paper = clean(paper),
        highlight = clean(highlight),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_paths_absolute_against_a_subpath_site() {
        assert_eq!(
            absolute("https://jane.github.io/homepage", "/assets/img/me.jpg"),
            "https://jane.github.io/homepage/assets/img/me.jpg"
        );
        assert_eq!(absolute("https://x.org", "https://y.org/a.jpg"), "https://y.org/a.jpg");
    }

    #[test]
    fn favicon_uses_the_first_letter() {
        let svg = favicon("Jane Doe", "#2b3c8c", "#f8f7f2", "#ffd43b");
        assert!(svg.contains(">J</text>"));
        assert!(svg.contains("fill=\"#2b3c8c\""));
        assert!(svg.contains("fill=\"#ffd43b\""));
    }
}
