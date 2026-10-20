# Configuration reference

Everything lives in `content/site.toml`. Only `[site]` and `[person]` are
required; every other table may be left out, and every list may be empty — a
section with nothing in it is not drawn.

Unknown keys are rejected, so a typo fails the build with the offending name
rather than quietly dropping a line from the page.

**Markdown and LaTeX** work in every long-text field (`intro`, `abstract`,
titles, `text`, `body`, `footer`). Inline maths is `\( … \)` or `$ … $`,
display maths is `\[ … \]` or `$$ … $$`. In TOML *basic* strings (`"…"` and
`"""…"""`) a backslash must be doubled — `\\( x^2 \\)` — or use a *literal*
string, `'''\( x^2 \)'''`, where it need not be. A single `$` followed by a
space, or a closing `$` followed by a digit, is not maths, so prices are safe.

**Links** may be absolute (`https://…`) or root-relative (`/assets/files/cv.pdf`).
Root-relative links are rewritten for you when the site lives under a subpath
such as `https://you.github.io/homepage/`, and checked: a link to a file that
is not in `static/` — a mistyped slides path, say — is reported when you build.

---

## `[site]`

| Key | Required | Default | Meaning |
| --- | --- | --- | --- |
| `title` | yes | | The browser-tab title. Usually your name. |
| `description` | yes | | One sentence for search results and link previews. |
| `base_url` | no | see below | Where the site is served, e.g. `https://jane.example.org`. |
| `lang` | no | `"en"` | The page's language tag. |
| `footer` | no | `""` | Markdown appended to the footer, e.g. `"Last updated May 2026."` |
| `credit` | no | `true` | The *Built with Corollary* line in the footer. |

**How `base_url` is chosen**, first match wins:

1. `--base-url` on the command line (what `scripts/serve.sh` uses for preview);
2. `base_url` in `[site]`;
3. the `SITE_BASE_URL` environment variable (what the GitHub Pages workflow sets);
4. `http://localhost:8000`.

So on GitHub Pages you can leave it unset and the workflow supplies the right
address, subpath included. Set it when you use a custom domain.

## `[person]`

| Key | Required | Meaning |
| --- | --- | --- |
| `name` | yes | The title of the page. Must match your spelling in `authors`. |
| `role` | no | e.g. `"PhD student"`. Not shown — say it in the introduction — but given to search engines. |
| `institution`, `institution_url` | no | Likewise: structured data only. |
| `group`, `group_url`, `location` | no | Not shown by the default templates; available to your own. |
| `email` | no | Shown with a copy button. Omit it to hide the contact line. |
| `photo_path` | no | Root-relative path under `static/`, e.g. `"/assets/img/me.jpg"`. Omit for no portrait. |
| `photo_alt` | no | Alt text. Defaults to "Portrait of *name*". |
| `photo_caption` | no | An optional caption under the portrait, e.g. a photo credit. |
| `links` | no | `[{ label = "DBLP", url = "…" }, …]` — listed under the email, and declared to search engines as the same person. |

The portrait's width and height are read from the file (JPEG or PNG) and
written into the markup, so nothing jumps while it loads. A `photo_path` that
does not exist is a build error.

## `[about]`

| Key | Meaning |
| --- | --- |
| `intro` | The opening paragraphs, beside the portrait. Markdown and LaTeX. |
| `keywords` | Research areas for structured data. Never shown. |

## `[people]`

```toml
[people]
"Bob Baird" = "https://example.org/bob"
```

Every paper that lists an author with exactly this spelling links their name.
Your own name is never linked.

## `[[publications]]`

One block per paper, in any order. They sort themselves: newest year first,
then accepted, preprint, submission, in preparation.

| Key | Required | Meaning |
| --- | --- | --- |
| `title` | yes | Markdown and LaTeX. |
| `authors` | yes | Full names, in the paper's order. |
| `year` | yes | Integer. |
| `status` | no | `accepted`, `preprint`, `submission` (default), `inpreparation`. Spelling is forgiving: `in preparation`, `draft`, `published` and `submitted` also work. |
| `venue` | if accepted | Short name, e.g. `"CRYPTO"`. |
| `venue_url` | no | Links the venue. |
| `note` | no | Parenthetical after the venue, e.g. `"to appear"`. Also goes into BibTeX. |
| `eprint` | no | IACR ePrint id `"2026/0123"` or full URL. Adds a link; BibTeX becomes an ePrint `@misc`. |
| `arxiv` | no | arXiv id `"2601.01234"`. Adds a link; BibTeX becomes an arXiv `@misc`. |
| `doi` | no | Adds a link and a `doi` field. |
| `links` | no | Anything else: `[{ label = "slides", url = "…" }]`. |
| `abstract` | no | Shown behind a fold. Markdown and LaTeX. |
| `bibtex_key` | no | Override the generated key. |
| `bibtex` | no | A complete BibTeX entry, used verbatim instead of the generated one. |

`status` decides the section:

| `status` | Section | Rendered as |
| --- | --- | --- |
| `accepted` | Publications | `CRYPTO 2026 (to appear)` |
| `preprint` | Publications | `Preprint, 2026` |
| `submission` | Manuscripts | `In submission, 2026` |
| `inpreparation` | Manuscripts | `In preparation` |

BibTeX is generated only for something a reader could cite: accepted or
posted work, or anything with an ePrint, arXiv id or DOI. Titles are
double-braced so `DCR` stays `DCR`.

## `[[talks]]`

| Key | Required | Meaning |
| --- | --- | --- |
| `title` | yes | Markdown allowed. |
| `venue` | yes | e.g. `"Theory Seminar, University of Example"`. |
| `venue_url` | no | Links the venue. |
| `date` | yes | Free text: `"Jun 2026"`. |
| `text` | no | One line beneath. |
| `links` | no | Slides, video, … |

Listed in the order written — newest first, by convention.

## `[[teaching]]`

| Key | Required | Meaning |
| --- | --- | --- |
| `role` | yes | `"Teaching assistant"`, `"Lecturer"`, … |
| `institution` | yes | |
| `institution_url` | no | |
| `term` | yes | `"Autumn 2026"`. |
| `text` | no | The course, in a line. Markdown allowed. |
| `links` | no | Course page, notes, … |

Listed in the order written.

## `[[service]]`

```toml
[[service]]
role = "Program committee"
venues = [
  { name = "TCC", year = 2026, url = "https://…" },
  { name = "ITC", year = "2025" },
]
```

One row per role. Venues are sorted newest year first; within a year they keep
the order written. `year` may be a number or a string.

## `[[sections]]`

Which sections appear and in what order. Leave it out for the default:
`about`, `research`, `talks`, `teaching`, `service`. `research` lists every
`[[publications]]` entry together, newest year first; to split published work
from work in flight, use `publications` and `manuscripts` in its place.

| Key | Meaning |
| --- | --- |
| `kind` | Picks `templates/sections/<kind>.html`. |
| `title` | The heading. Defaults to the kind's usual name; `""` hides it. `about` has none by default. |
| `id` | The anchor (`#awards`). Defaults to `kind`; repeats get `-2`, `-3`. |
| anything else | Passed to the template as `section.<key>`. |

Two generic kinds ship with Corollary:

```toml
[[sections]]
kind = "list"
id = "students"
title = "Students"

[[sections.items]]
title = "Sam Lee"              # required; Markdown allowed
meta = "PhD, co-advised"       # optional
meta_url = "https://…"         # optional; links `meta`
date = "2025–"                 # optional
text = "Working on lattices."  # optional; Markdown allowed
links = [{ label = "web", url = "https://…" }]

[[sections]]
kind = "markdown"
title = "Contact"
body = """
Office 3.14, Example Building. Office hours by appointment.
"""
```

## `[conventions]`

| Key | Default | Meaning |
| --- | --- | --- |
| `author_order` | `""` | A note printed above the paper lists, e.g. `"Authors are listed alphabetically."` Printed only while every multi-author paper really is in surname-alphabetical order. |
| `highlight_self` | `false` | Bold your own name in author lists. |

## `[bibtex]`

| Key | Default | Meaning |
| --- | --- | --- |
| `enabled` | `true` | `false` removes every BibTeX button. |
| `key_style` | `"initials"` | `"initials"` → `ABD26`; `"surname-year"` → `archer2026`. Clashes get `a`, `b`, … |

Surnames are taken as the last word of a name; for "van der Berg" set
`bibtex_key` on the paper.

## `[labels]`

The words the generator writes itself, for another phrasing or language:

```toml
[labels]
preprint = "Preprint"
submission = "In submission"
in_preparation = "In preparation"
```

Every other word on the page is in `templates/`, where it can be edited
directly.

## `[features]`

| Key | Default | Meaning |
| --- | --- | --- |
| `theme_toggle` | `true` | The light/dark switch. Without it the page follows the reader's system setting. |

## `[plugins]`

Each self-contained feature, on unless switched off. Build plugins run in the
generator (`sitegen/src/plugins/`); page plugins run in the reader's browser,
in the WebAssembly module (`wasm/src/plugins/`).

| Key | Default | Kind | Meaning |
| --- | --- | --- | --- |
| `bibtex_highlight` | `true` | build | Colours BibTeX entries (type and key, field names, values, punctuation). Copying an entry copies plain text either way. |
| `minify_css` | `true` | build | Ships the stylesheets without comments or spare whitespace. The sources keep both. |
| `heading_flow` | `true` | page | Lets the gradient on the name and section labels drift slowly. It holds still for readers who ask for reduced motion. |

The theme switch and copy buttons are page plugins too, always present (the
switch follows `[features] theme_toggle`).

## `[theme]`

Overrides for the CSS custom properties in
`static/assets/css/theme.css`, without the leading dashes:

```toml
[theme]
light = { accent = "#9b2226", accent-2 = "#2f6f4e" }
dark  = { accent = "#f0a3a0" }
```

The properties most worth changing:

| Property | Role |
| --- | --- |
| `paper`, `paper-raised` | Background, and the slightly raised ground behind controls. |
| `ink`, `body`, `muted` | Headings, running text, metadata. |
| `rule`, `rule-strong` | Faint lines. |
| `accent`, `accent-hover` | Links, section labels, separators, the callout's line, and the name's gradient. |
| `accent-2` | The second accent: BibTeX field names, and the far end of the labels' gradient. |
| `accent-quiet` | Text selection. |
| `highlight` | Colours the generated favicon. |
| `link-mark`, `link-sweep` | An optional stroke behind prose links and a sweep on hover, as CSS gradients; `none` by default. |
| `sans`, `mono`, `mono-address`, `serif` | Font stacks: the page, BibTeX, the email address, mathematics. |
| `gutter` | Side margin on small screens. |
| `portrait-mask`, `portrait-shape` | The portrait's shape (a squircle, as an SVG mask) and the outline text wraps to beside a captioned portrait. |
| `portrait-size`, `portrait-filter` | The portrait's width in the title block, and its CSS filter. |
| `measure` | The column's width. |

Check new colours against both grounds: every default clears WCAG AA.

## Files outside `site.toml`

| Path | Effect |
| --- | --- |
| `static/**` | Copied to the site as-is. `README.md` files there are not published. |
| `static/favicon.svg` | Replaces the generated monogram favicon. |
| `static/robots.txt` | Replaces the generated one. |
| `static/assets/css/custom.css` | Loaded after the built-in styles; delete it if unused. |
| `CNAME` | Copied into the output, for hosts that read it. |
