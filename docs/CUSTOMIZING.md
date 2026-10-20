# Customising Corollary

Corollary is meant to be changed. This guide goes from the gentlest changes to
the deepest; stop at the first one that does what you need.

## 1. Colours, in `site.toml`

```toml
[theme]
light = { accent = "#9b2226", accent-hover = "#b23a3e", highlight = "#f4c7c3" }
dark  = { accent = "#f0a3a0", accent-hover = "#f6c0be" }
```

Any custom property from `static/assets/css/theme.css` may be named. The
light values go on `:root`; the dark values are applied both when a reader
picks dark with the toggle and when their system asks for it. The generated
favicon follows `accent`, `paper` and `highlight`.

To keep the page's character, change hues rather than structure: one accent
for links, labels and separators, and a second (`accent-2`) for highlighted
BibTeX. The default pairing, slate blue and teal on a cool grey paper and a
slate-tinted near-black, is described in [DESIGN.md](DESIGN.md).

## 2. Small CSS changes, in `custom.css`

`static/assets/css/custom.css` is loaded after everything else, so a rule
there wins without editing the built-in stylesheets — which keeps later
updates painless. A few things to try:

```css
/* A round portrait instead of the squircle */
.portrait { -webkit-mask: none; mask: none; border-radius: 50%; }

/* A wider text block */
:root { --measure: 44rem; }

/* Underlined prose links */
.prose a { text-decoration: underline; text-underline-offset: 0.2em; }

/* One section alone: each is <section id="…" class="section section-<kind>"> */
#awards .entry-title { font-style: italic; }
```

Delete the file if you do not use it; the page then stops requesting it.

## 3. Order, headings, and generic sections

`[[sections]]` in `site.toml` lists what appears and in what order. Rename a
heading with `title`, hide it with `title = ""`, drop a section by leaving it
out. Add awards, students, grants or software with `kind = "list"`, and
anything free-form with `kind = "markdown"`. See
[CONFIGURATION.md](CONFIGURATION.md#sections).

## 4. A new kind of section

No Rust needed. Say you want a "Selected software" section with stars:

`content/site.toml`:

```toml
[[sections]]
kind = "software"
title = "Software"
projects = [
  { name = "fastlattice", url = "https://github.com/…", blurb = "Lattice reduction, quickly." },
]
```

`templates/sections/software.html`:

```html
{#- Rendered with `section` (this entry) and `data` (the whole site). -#}
{%- if section.projects %}
<ul class="entry-list">
  {%- for p in section.projects %}
  <li class="entry">
    <p class="entry-title"><a href="{{ p.url | url }}">{{ p.name }}</a></p>
    <p class="entry-body">{{ p.blurb | inline_markdown | safe }}</p>
  </li>
  {%- endfor %}
</ul>
{%- endif %}
```

That is all. The rules:

- Every key in the `[[sections]]` entry is available as `section.<key>`, along
  with `section.kind`, `section.id` and `section.title`.
- `data` holds everything else — `data.person`, `data.publications`,
  `data.talks`, … — so a section can present existing data differently, e.g.
  a "Selected publications" section that filters `data.publications`.
- The template renders only the body; the `<section>` wrapper and heading are
  added by `templates/index.html`.
- If the template renders nothing (an empty list, say), the section and its
  heading are left off the page.
- A `kind` with no template is a build error that lists the kinds available.

Filters available in every template:

| Filter | Does |
| --- | --- |
| `markdown` | Markdown + LaTeX to HTML, as paragraphs. |
| `inline_markdown` | The same, without the enclosing paragraph. |
| `url` | Prefixes a root-relative link with the site's subpath, if any. Use it on every `href` and `src`. |
| `asset` | `url`, plus a fingerprint of the file's contents, for stylesheets and scripts. |

Plus everything [Tera](https://keats.github.io/tera/docs/) offers.

## 5. Markup

| Template | Holds |
| --- | --- |
| `templates/base.html` | `<head>`, the theme switch, footer, scripts. |
| `templates/index.html` | Title block, then the section loop. |
| `templates/partials/masthead.html` | Title block: name, email, profile icons, and the portrait beside them. |
| `templates/sections/about.html` | The introduction (and a captioned portrait, which stays with it). |
| `templates/partials/paper.html` | One publication entry; the link-run macro. |
| `templates/sections/*.html` | One file per kind of section. |
| `templates/partials/head-extra.html` | Empty hook at the end of `<head>`: analytics, verification tags, extra styles. |
| `templates/partials/body-end.html` | Empty hook before `</body>`: extra scripts. |
| `templates/404.html` | The not-found page. |

For example, a privacy-friendly analytics snippet goes in `head-extra.html`;
nothing else needs to change. A stylesheet of your own added there should be
linked through the `asset` filter, which fingerprints the URL so a visitor
never pairs a new page with a cached old copy:

```html
<link rel="stylesheet" href="{{ "/assets/css/talks.css" | asset }}">
```

## 6. Typefaces

Three faces are served from `static/assets/fonts/`: Be Vietnam Pro for the
page (regular, medium, semibold), DM Mono for the email address, and Azeret
Mono for BibTeX. To use another face, replace its `.woff2` files and
`@font-face` blocks at the top of `theme.css`, and the first entry of the
matching stack (`--sans`, `--mono-address` or `--mono`). The page's regular
face is also preloaded in `templates/base.html`; change that URL with it. To
download no font at all, delete the blocks and the named entries; the
platform's own faces take over. Licences are in `static/assets/fonts/OFL.txt`.

## 7. The generator

`sitegen/src/` is a small Rust program, one module per concern:

| Module | Responsibility |
| --- | --- |
| `config.rs` | The schema of `site.toml`. Add a field here first. |
| `publications.rs` | Status, sorting, author links, the publication view. |
| `bibtex.rs` | Citation keys and entries. |
| `plugins/` | Build plugins behind one trait: `bibtex_highlight`, `minify_css`. |
| `markdown.rs` | Markdown, LaTeX → MathML, URL prefixing. |
| `seo.rs` | JSON-LD, sitemap, robots.txt, favicon. |
| `render.rs` | Tera setup and filters, the section pipeline, file output. |
| `main.rs` | Wiring. |

`cargo test -p sitegen` covers ordering, the Research list and the
publication/manuscript split, BibTeX and its highlighting, CSS minification,
ePrint and arXiv handling, the maths pipeline, base paths and theme overrides.
Add a test with any change.

### Plugins

A build plugin is a module in `sitegen/src/plugins/` with a type implementing
`BuildPlugin`. Its two hooks both default to doing nothing: `asset` may rewrite
any file copied from `static/` (before its URL is fingerprinted), and
`publication` may decorate each entry before it is rendered. Give it a switch
in `config::Plugins` and register it in `Plugins::new`.

A page plugin is a module in `wasm/src/plugins/` exporting a `Plugin` (a name
and an `install` function), listed in `plugins::ALL`. The generator names the
enabled ones in `<html data-plugins="…">`; the module installs only those.
The WebAssembly crate handles only what cannot be decided at build time (the
reader's theme choice, their clipboard, motion), so the page remains complete
without JavaScript.

## Keeping up with Corollary

Keep your changes in `content/`, `static/`, `custom.css` and new templates,
and later versions merge cleanly:

```sh
git remote add corollary https://github.com/DeviousCilantro/corollary.git
git fetch corollary
git merge corollary/main --allow-unrelated-histories   # the first time only needs the flag
git checkout --ours content/site.toml                  # keep your content if it conflicts
```
