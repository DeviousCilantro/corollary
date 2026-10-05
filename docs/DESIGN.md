# Design notes

The target is a theorist's homepage: clear hierarchy, restrained typography,
sparse content, low visual noise. Not a portfolio, not a blog. These notes
explain the defaults, so that changing one is a choice rather than an
accident.

## One column

A homepage has one job — let a visitor establish who you are and what you have
written — and one column, read top to bottom, does it with no navigation at
all. The column is the width of a printed text block, about 70 characters,
centred on the screen, and nothing hangs outside it: margins stay even on both
sides.

Inside it, everything hangs from a single left edge. Ragged-right text gives a
column only one firm edge, and a centred heading over it floats with nothing
to anchor it; left-aligned heads, each followed by a hairline running out to
the right margin, keep the axis and carry the eye across, so the right side of
the column never reads as empty.

On large screens the type grows with the window, up to 20px, so the column
keeps the proportion of a printed page instead of shrinking into a strip
between two empty margins.

The page opens with the name as the title — the theme switch alone in the
corner above it — then the contact line, and the introduction. The portrait is round, and the text follows
its curve rather than the square box around it: the one soft shape on a page
of straight lines. A one-line footer closes the page.

## Typography

One text face, Literata, chosen because it carries the two features the page
relies on and most platform serifs lack: true small caps and old-style
figures. A monospace is kept for BibTeX and the copy buttons only.

- **Small caps** for the section heads, letter-spaced. Nothing else.
- **Old-style figures** throughout the prose: a year in a sentence is a word,
  not a measurement. Code is exempt; a BibTeX year wants to line up.
- **Italic** for the venue of a paper and the authorship note.

Structure comes from type and whitespace alone. There are no cards, no
shadows, no rounded panels, no icons other than the theme switch.

## Colour: paper and blackboard

The two surfaces theory is actually done on.

In the light theme the page is ivory paper written in blue-black fountain-pen
ink, and links are marked the way one marks up a paper while reading it — a
stroke of yellow highlighter behind the lower half of the word. On hover a
second, stronger stroke sweeps across the word, left to right, as a pen would.
Selected text takes the same yellow.

In the dark theme the page is a slate blackboard. There is no highlighter on a
blackboard: links are simply written in yellow chalk, with no mark at rest. A
translucent yellow under them on slate only muddies to olive, and a yellow line
under them looks like an error underline; chalk on its own is cleaner.

So the yellow is the constant — the highlighter by day, the chalk by night —
and the page keeps its character across the toggle. Photographs are held back
a little in both themes, more on the blackboard, so they sit in the page rather
than on it.

Contrast is deliberately short of the maximum. Near-white on near-black glares;
headings sit near 15:1 and body text near 10:1, roughly where print sits. Every
text colour clears WCAG AA against its own ground.

Metadata links — coauthors, venues, institutions — inherit their line's colour
and stay quiet, so the eye is not pulled through the lists. Actions — ePrint,
slides, BibTeX — keep the accent.

## Details that do work

- Separators in a meta line are drawn by the stylesheet and clipped at the
  start of a line, so a wrapped line never opens on a stray dot; a venue and
  its year never part.
- Opening a BibTeX entry never moves its label.
- Copy buttons say "copied" only when the clipboard has the text; otherwise
  they select it and name the shortcut.

## Rendering

Everything is rendered by the Rust generator at build time, including LaTeX,
which becomes MathML. The page ships no maths engine and makes no third-party
request of any kind.

The WebAssembly module handles only what genuinely depends on the reader: their
colour theme and their clipboard. One small inline script applies a stored
theme before first paint, because WebAssembly is instantiated asynchronously
and a reader who chose dark would otherwise see a flash of light. If the module
fails to load, nothing is lost but the toggle and the copy buttons.

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
