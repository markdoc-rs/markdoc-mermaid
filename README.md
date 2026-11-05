# markdoc-mermaid

Mermaid diagrams for Markdoc, with node labels resolved from the document's
own anchors.

## Why not just write Mermaid

Because a label written in a diagram is a *second copy* of a heading that
already exists, and a second copy is free to drift. Rename the heading and the
picture keeps the old wording, with nothing anywhere to tell you. That is
precisely how the hand-drawn diagrams this replaces went stale.

So an author writes **ids, not labels**:

```markdoc
{% mermaid %}
flowchart TD
  troubleshooting_cannot_connect      --> troubleshooting_cannot_connect_can2
  troubleshooting_cannot_connect_can2 -- Ping succeeded --> troubleshooting_cannot_connect_can
  troubleshooting_cannot_connect_can2 -- Ping failed    --> troubleshooting_cannot_connect_is
{% /mermaid %}
```

and the box text is the heading each id sits on, read out of the document at
build time. The diagram and the steps are one string, and renaming a step
redraws the diagram.

Everything else about Mermaid still works. Edge labels, subgraphs, the other
twenty-odd diagram types — none of it is touched, which is the point of
choosing Mermaid over a bespoke tag.

## What it does to the source

Nothing, in place. The author's own lines are never edited; labels and links
are **appended** as separate declarations:

```text
  troubleshooting_cannot_connect_can2["Can you ping the switch?"]
  click troubleshooting_cannot_connect_can2 "#troubleshooting_cannot_connect_can2"
```

Rewriting edge lines would mean understanding every arrow form Mermaid has
(`-->`, `---`, `-.->`, `==>`, `--x`, `--o`, and the labelled variants of each).
Appending needs none of that, so a form this crate has never heard of passes
through intact.

There is **no Mermaid parser here**, and there does not need to be: the
resolver is the filter. An identifier becomes a label only if the caller's
index knows it, and anchor ids are long and distinctive. A bare word in an
edge label resolves to nothing and is left alone. A label the author wrote by
hand wins; a `click` they wrote themselves is not duplicated.

## Links, and why the href matters in print

`Anchor::href` is the caller's to choose, and the choice is not cosmetic:

| href | on the web | in a PDF |
|------|-----------|----------|
| `#step_two` | jumps to the heading | **internal destination** — the reader moves within the document |
| `/docs/page#step_two` | jumps to the heading | external URI action — opens a browser, or the viewer refuses |

Use the fragment whenever the target is on the page being rendered. Only the
caller knows which page that is, which is why this crate will not guess.

## Output a page can own

A renderer emits a finished picture: a white canvas, fixed pixel dimensions,
and every colour written into the markup. A document needs the opposite. So
the SVG is reworked:

- the full-bleed background rect is removed — identified by its geometry, not
  its colour, because the same white is used behind edge labels to stop a line
  striking through the text;
- `width` and `height` come off the root, leaving `viewBox`, so CSS can size
  it for a phone, a text column or a sheet of paper;
- each baked colour becomes `var(--diagram-*, original)`. With no stylesheet
  it looks exactly as the renderer drew it; with one it follows the page into
  dark mode and back out for print.

| token | what it paints |
|-------|----------------|
| `--diagram-text` | node and edge-label text |
| `--diagram-line` | edges and arrowheads |
| `--diagram-border` | node and label outlines |
| `--diagram-node` | node body |
| `--diagram-surface` | the backplate behind an edge label |

## Room for the labels

A renderer's defaults assume a diagram someone drew by hand, where a node is
called `A` and an edge is labelled `yes`. Labels taken from headings are
sentences, and the defaults do not expect them.

Left alone, `mermaid-rs-renderer` 0.3.1 draws **four of the twenty-nine edge
labels** across this project's three troubleshooting diagrams *underneath* a
node box. Not crowded, not clipped — invisible, while still sitting in the
PDF's text layer where a search will find them. On a decision tree the edge
labels are the branch conditions, so one of those is a reader who cannot tell
which way to go:

| | lost | height |
|---|---|---|
| renderer default (`0.55`) | 4 | — |
| `Layout::default()` (`0.80`) | **0** | +0.9% |

`0.80` is the centre of the `0.75..=0.85` plateau where all three come out
clean. Above `0.90` the count climbs again and moves about unpredictably,
because nothing in the renderer *constrains* a label to stay clear — a wider
gap only makes a collision less likely. That is why this is a knob with a
tuned default and not a fixed constant, and why
`no_branch_condition_is_drawn_underneath_a_box` asserts against the rendered
geometry rather than against the number.

```rust
Options {
    layout: Layout { label_clearance: 1.2, ..Layout::default() },
    ..Options::default()
}
```

`rank_spacing` and `node_spacing` are there too, and both default to `None`,
meaning *let the renderer measure*. Setting either switches the diagram to
explicit spacing, because the renderer's adaptive pass treats a configured
spacing as a **ceiling** — it halves the mean node extent and keeps whichever
is smaller, so asking for 90 and asking for 110 otherwise draw the same
picture. Neither fixes a buried label; they only make the diagram taller.

## Using it

```rust
use markdoc_mermaid::{Anchor, Options, render};

let diagram = render(
    source,
    &|id: &str| index.get(id).map(|a| Anchor {
        label: a.title.clone(),
        href: if a.url == here { format!("#{id}") } else { format!("{}#{id}", a.url) },
    }),
    &Options::default(),
)?;
```

`Diagram::resolved` lists the ids that got a label. **Empty is worth a word
from the caller**: the diagram rendered, but nothing resolved, so either it is
hand-labelled — fine — or the ids are misspelt and a reader is about to meet a
diagram of identifiers.

## Choice of renderer

`mermaid-rs-renderer`, for two reasons that are not about features.

**Licence.** `flowmaid` is the better engine by most measures and is
GPL-3.0-or-later, which it passes to anything linking it. `merman` is
`0.10.0-alpha.1` and wants Rust 1.95. Neither is a safe foundation for a
proprietary documentation pipeline today. `tree-sitter-mermaid` is a grammar,
not a renderer; `mermaid-fixer` is a lint helper.

**`<text>`, not `<foreignObject>`.** That one decides whether a diagram
printed into a PDF is searchable text or a picture of text — for a manual read
offline, the difference between a document and a screenshot.

The renderer sits behind this crate's own API on the assumption that it will
be replaced. Nothing a caller touches names it.

## Status

- [x] Id resolution — labels and `click` links from a document's anchors
- [x] Author-written labels and `click`s are respected, never overruled
- [x] SVG reworked so the page owns size and colour
- [x] Deterministic output — the same source renders the same bytes
- [x] Spacing control, with a default that keeps branch conditions readable
- [ ] Diagram types beyond flowchart exercised against real content. The
      renderer claims 23; only flowchart has been looked at
- [ ] Rhombus nodes (`A{...}`). 0.3.1 lays them out poorly — a large diamond
      with the edge label drawn across it. Rectangles are fine; this is worth
      re-testing on each renderer bump
- [ ] A diagram too tall for a printed page. Nothing crops or scales it yet,
      and height is the live risk, not width: these trees render near an
      aspect ratio of 0.55, and a longer one would run off the sheet.
      Widening `max_label_width_chars` past the renderer's 22 does flatten
      them — 0.76 on the safety tree — but it is not free: at 40 chars the
      *cannot-start* tree loses a label that is readable at 22. Worth
      exposing, but it needs its own clearance tuning rather than a default
- [ ] `backedge_cross_weight` is read by nothing in 0.3.1, so a graph with
      edges pointing back up it has no handle at all. Worth raising upstream
