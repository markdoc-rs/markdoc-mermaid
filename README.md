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

## What is supported, per diagram type

This table is what this crate actually does rather than what Mermaid
documents. The renderer is [merman](https://github.com/Latias94/merman), which
draws every type Mermaid documents.

**Anchor labels** is the feature above: an id resolved to the heading it names.
It needs a declaration form the diagram's own grammar accepts, and most
grammars have none — a mindmap or treemap node *is* its text at an indent, so
there is nothing to attach a label to, and an ER entity is drawn by its id with
no alias anywhere in the syntax. Where the column is empty the diagram still
renders; its labels are just the author's own.

**Links** is `click`, read by the flowchart and swimlane grammars. **SVG
ids** is merman's `data-id`, for a page that wants to make the diagram
interactive, with `data-et` saying what each element is — `node` and `edge`
for a flowchart, `participant`/`message`/`life-line` for a sequence, and
edges only for class, state, ER and mindmap. A block diagram carries none.
`data-from`/`data-to` are not emitted: take an edge's ends from
`Edge::from`/`Edge::to`.

| diagram | draws | anchor labels | links | SVG ids |
|---|---|---|---|---|
| `flowchart` / `graph` | yes | yes | yes | yes |
| `sequenceDiagram` | yes | yes |  | yes |
| `classDiagram` | yes | yes |  | yes |
| `stateDiagram-v2` | yes | yes |  | yes |
| `block-beta` | yes | yes |  |  |
| `swimlane-beta` | yes | yes | yes | yes |
| `usecase-beta` | yes |  |  | yes |
| `erDiagram` | yes |  |  | yes |
| `mindmap` | yes |  |  | yes |
| `kanban` | yes |  |  | yes |
| `treemap-beta` | yes |  |  | yes |
| `treeView-beta` | yes |  |  | yes |
| `eventmodeling` | yes |  |  | yes |
| `zenuml` | yes |  |  | yes |
| `packet-beta` | yes |  |  | yes |
| `architecture-beta` | yes |  |  |  |
| `requirementDiagram` | yes |  |  |  |
| `gitGraph` | yes |  |  |  |
| `C4Context` | yes |  |  |  |
| `timeline` | yes |  |  |  |
| `journey` | yes |  |  |  |
| `quadrantChart` | yes |  |  |  |
| `radar-beta` | yes |  |  |  |
| `sankey-beta` | yes |  |  |  |
| `xychart-beta` | yes |  |  |  |
| `gantt` | yes |  |  |  |
| `pie` | yes |  |  |  |
| `cynefin-beta` | yes |  |  |  |
| `ishikawa-beta` | yes |  |  |  |
| `venn-beta` | yes |  |  |  |
| `wardley-beta` | yes |  |  |  |

Every diagram type mermaid's syntax reference documents is drawn, because
merman draws them all.

`usecase-beta` draws but does not take anchor labels: in merman a use case id
carries a *kind*, so declaring `Browse("heading")` after `actor Browse` is
rejected as a conflicting kind. Labelling one without re-declaring its kind
would need the declaration form to know the kind.

A header this renderer has no parser for — a typo, or a diagram type newer
than this crate — fails with `unsupported Mermaid diagram type: <header>`.
merman's own message names no diagram, so the header is quoted back into the
error.

Where merman's drawing differs from Mermaid-in-a-browser, merman's own
documentation is the place to look: this crate does not lay anything out.

Everything *within* a supported type is untouched — edge labels, subgraphs,
directions, shapes, styling — because none of it passes through this crate.

`markdoc_mermaid::resolves` answers the third column for a diagram kind, so a
caller can tell an empty `Diagram::resolved` that means "no ids matched" from
one that means "this kind cannot carry a label".

## Links, and why the href matters in print

`Anchor::href` is the caller's to choose, and the choice is not cosmetic:

| href | on the web | in a PDF |
|------|-----------|----------|
| `#step_two` | jumps to the heading | **internal destination** — the reader moves within the document |
| `/docs/page#step_two` | jumps to the heading | external URI action — opens a browser, or the viewer refuses |

## Migrating from 0.3

0.4.0 draws with [merman](https://github.com/Latias94/merman). `render`,
`Anchor`, `Anchors`, `flowchart` and the graph types keep their shape; the
rest is additions, two removals and the renderer's own SVG.

- `Options` gained `id` and `measure`, so a struct literal naming only the
  old fields needs `..Options::default()`.
- `Layout` lost `label_clearance` and `tidy_edges`, and gained
  `wrapping_width` (180), `classic_look` (on), `palette`, `font_family`,
  `label_slack` (1.0), `corner_radius` and `line_width`. It is
  `#[non_exhaustive]`, so build one from `Layout::default()`.
- `Diagram` gained `size`, `font_size` and `min_font_size`, and is
  `#[non_exhaustive]` too. The renderer writes no `font-size` attribute for
  a caller to scrape, which is why the last two exist.
- The SVG hooks are merman's: `data-id` with `data-et` on a node and on an
  edge, and an edge id of `L_<from>_<to>_<n>`. Take an edge's ends from
  `Edge::from` and `Edge::to` rather than splitting the id.
- Every rule the SVG carries is scoped under the root's id, so it outranks
  a page's own rule for the same box. In themeable output, set a
  `--diagram-…` token on the node and the diagram's own rule paints from it;
  otherwise use `!important`, or name the diagram in the selector, which
  means setting `Options::id`.
- A very large flowchart no longer renders; see the renderer's ceiling
  below.
- `usecase-beta` no longer takes anchor labels, for the reason above.

## What a themed page can and cannot recolour

A kind is only reliably readable in **both** a light and a dark page when
its text *and* the surface under it are the page's. Where a kind paints its
own surfaces — a slice, a bar, a tile, a quadrant — the page's text colour
lands on Mermaid's fill, and one of the two is then wrong.

Flowchart, sequence, class, state, block, swimlane, use case, ER, tree view,
packet and mindmap are the kinds whose surfaces this crate can reach, and
they are the ones a themed page can rely on. Where a surface is Mermaid's
own — a gantt bar, a timeline section, a treemap tile, a mindmap section, a
note — the text keeps Mermaid's matching colour rather than taking the
page's, because tokenising one half of a pair is what makes contrast worse.

**An author's own colours stay the author's.** A `style`, a `classDef` or a
sequence diagram's `rect rgb(200, 220, 255)` keeps the fill it names, while
the text inside it takes the page's colour. A diagram that sets its own
colours wants `themeable: false`, or colours chosen to work with the page.

A diagram also carries its stylesheet in a `<style>` element, which is text
content: a search index or reading-time estimate that walks the page will
count it unless it skips `<style>`.

## The renderer

[merman](https://github.com/Latias94/merman), which is Mermaid.js headless
in Rust, drawn through `SvgPipeline::resvg_safe()`. That pipeline turns
Mermaid's HTML labels into real `<text>` rather than leaving them in a
`<foreignObject>`, which decides whether a diagram printed into a PDF is
searchable text or a picture of text. It is kept behind this crate's own
API: nothing a caller touches names it.

`default-features = false` drops `layout-elk`, which pulls in
`merman-elk-layered` (EPL-2.0); a diagram asking for `layout: elk` falls
back to dagre.

## What the renderer gets wrong

merman has defects this crate cannot work around, written up for upstream.
These reach a diagram you draw:

- **An emphasis marker in a `stateDiagram-v2` label is eaten.** A `**` draws
  the words bold and a `_` draws them italic, markers gone, because a state
  label is read as markdown *before* any escape. Every other kind that reads
  markdown draws the characters as typed.
- **A literal entity reference cannot be drawn in a label.** `&amp;` as
  *text* is decoded twice, once with the source's placeholders and once by
  the label reader, and nothing a caller writes survives both.
- **An escape in a `sankey-beta` field fails the diagram**, quoted or not,
  where every other kind takes Mermaid's escapes in a label.
- **A large flowchart does not draw.** The renderer's label pass has a fixed
  ceiling of selector-matching work with no setting to raise it, which a
  flowchart of a few hundred boxes reaches. `render` returns an error saying
  so, and the answer is to split the diagram.
- **A long `radar-beta` legend entry runs outside the declared canvas**, so
  a consumer that honours that size clips it.

Two more are avoided by how this crate writes what it appends, which is
worth knowing if you read the SVG yourself. A quoted label is escaped with
entities (`&lt;`) rather than Mermaid's own `#lt;`, because a `#lt;`
reaching the label reader draws *every* label of a state diagram empty; and
the placeholder characters the renderer leaves in a `click` tooltip are
decoded after the render.

Output is byte-identical run to run, process to process and across font
sets. It is not byte-identical between a glibc build and a musl one, which
matters only to a consumer keeping golden SVGs across platforms.

## License

MIT
