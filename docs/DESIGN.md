# Design notes

The target is a theorist's homepage: clear hierarchy, restrained typography,
sparse content, low visual noise. Not a portfolio, not a blog. These notes
explain the defaults, so that changing one is a choice rather than an
accident.

## One column, one edge

A homepage has one job (let a visitor establish who you are and what you have
written) and one column, read top to bottom, does it with no navigation at
all. Everything hangs from a single left edge: the name, the introduction,
every entry, the footer.

On a wide screen each section's label hangs in a column of its own to the
left of that edge, small capitals in the accent, level with the section's
first entry. The labels mark the structure without a rule or a box: the edge,
the accent and whitespace carry it. On a narrower screen the labels sit above
their sections.

The page opens with the name and, beside it, the portrait: a squircle (a
superellipse, the shape between a circle and a square), its top level with the
name's capitals and its foot with the profile icons. Then the introduction,
whose closing paragraph (the research interests, usually) is set apart with a
fine accent line and a wash of the accent that fades out to the right, so it
has no hard edge to overrun the ragged lines above it. A one-line footer closes
the page.

## Typography

One sans text face, Be Vietnam Pro, in three weights: regular for text, medium
for titles, semibold for the name and the section labels. Two monospaces with
one job each: DM Mono for the email address, Azeret Mono for BibTeX, whose
columns want to line up. All three are served from the site itself.

- **Capitals**, letter-spaced, for the name and the section labels.
- **Hierarchy by weight and colour**, not size: a title in ink, its authors a
  step quieter, its venue, date and links quieter still.
- No italics, no underlines. A link is marked by the accent alone.

## Colour: one accent, and a second for code

Slate blue (#6a5acd) is the page's only colour. Links, section labels, the
separators in each meta line and the callout's line all take it; everything
else is ink and greys on a cool, neutral ground, so the accent is never
competing. Teal is a second accent with a single use: the field names of a
highlighted BibTeX entry.

The light theme is near-black ink on a cool grey paper. The dark theme is slate
blue darkened to a soft near-black, deliberately not the pure black of an OLED
theme, with links in a lighter slate blue.

The headings carry a gradient, the name from ink into the accent, the labels
from the accent into the teal, and it flows: the colours drift slowly back and
forth across the letters (see Plugins, below). It holds still for a reader who
asks for reduced motion, and in any browser without the module.

Contrast is deliberately short of the maximum. Headings sit near 15:1 and body
text near 10:1; every text colour clears WCAG AA against its own ground.
Metadata links (coauthors, venues, institutions) inherit their line's colour
and take the accent only under the pointer; actions (ePrint, arXiv, slides,
BibTeX) keep it.

## Details that do work

- Separators in a meta line are drawn by the stylesheet and clipped at the
  start of a line, so a wrapped line never opens on a stray dot; a venue and
  its year never part.
- Service venues are grouped by year: each year is written once, its venues
  after it, rather than a year after every venue.
- Profiles the page recognises (Google Scholar, ORCID, GitHub, DBLP, arXiv)
  are shown as small glyphs, their names kept for screen readers.
- Opening a BibTeX entry never moves its label; copying it copies exactly the
  entry, whatever colours it is drawn in.
- Copy buttons say "copied" only when the clipboard has the text; otherwise
  they select it and name the shortcut.
- Stylesheet and script URLs carry a fingerprint of their contents, so a
  browser never pairs a new page with a stylesheet or script it cached from an
  older one.

## Plugins

Each self-contained feature is a plugin, switched in `[plugins]` of
content/site.toml and on by default.

- **Build plugins** run in the Rust generator (`sitegen/src/plugins/`):
  `bibtex_highlight` colours BibTeX entries into spans, and `minify_css`
  strips the shipped stylesheets of comments and spare whitespace while the
  sources keep their documentation.
- **Runtime plugins** run in the reader's browser, in the Rust WebAssembly
  module (`wasm/src/plugins/`): `theme` (the switch), `clipboard` (the copy
  buttons) and `flow` (the drifting gradient). The generator names the enabled
  ones in the page, and the module installs only those.

## Rendering

Everything is rendered by the Rust generator at build time, including LaTeX,
which becomes MathML, and BibTeX highlighting. The page ships no maths engine
and makes no third-party request of any kind.

The WebAssembly module handles only what depends on the reader being present:
their colour theme, their clipboard, and the motion of the gradient. One small
inline script applies a stored theme before first paint, because WebAssembly
is instantiated asynchronously and a reader who chose dark would otherwise see
a flash of light. If the module fails to load, nothing is lost but the toggle,
the copy buttons and the motion.

## Content

Content earns its place or goes. A running head repeating the name, a subtitle
restating the introduction, a key-words line, section numbers, a footer
repeating the contact details, a closing tombstone: each was tried and
removed. A news feed restating the talks and teaching sections
is filler; a public CV invites detail that competes with the strongest signal.

## Honesty in the details

A few defaults exist so that the page never claims more than it can support:

- The alphabetical-authorship note is printed only while it is true.
- A paper accepted but not yet in the proceedings says "to appear", in the
  page and in its BibTeX.
- A manuscript under submission gets no BibTeX entry, since there is nothing
  yet to cite, and is left out of the structured data search engines read.
- The email address is a link on the page for the reader who wants it, but is
  kept out of the structured data, where it would be easiest to harvest.
