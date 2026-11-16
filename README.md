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

The renderer is vendored (`vendor/mermaid-rs-renderer`), so this table is what
this crate actually does rather than what Mermaid documents. Mermaid's own
syntax reference lists 31 diagram types; all 31 draw here.

**Anchor labels** is the feature above: an id resolved to the heading it names.
It needs a declaration form the diagram's own grammar accepts, and most
grammars have none — a mindmap or treemap node *is* its text at an indent, so
there is nothing to attach a label to, and an ER entity is drawn by its id with
no alias anywhere in the syntax. Where the column is empty the diagram still
renders; its labels are just the author's own.

**Links** is `click`, which only the flowchart grammar reads. **SVG ids** is
`data-node-id` on every node and `data-from`/`data-to` beside every
`data-edge-id`, for a page that wants to make the diagram interactive; the
types without it are the ones whose renderers draw their own shapes without
going through a node's link, where the marks cannot be placed safely and the
diagram is emitted exactly as the renderer drew it.

| diagram | draws | anchor labels | links | SVG ids |
|---|---|---|---|---|
| `flowchart` / `graph` | yes | yes | yes | yes |
| `sequenceDiagram` | yes | yes |  | yes |
| `classDiagram` | yes | yes |  | yes |
| `stateDiagram-v2` | yes | yes |  | yes |
| `block-beta` | yes | yes |  | yes |
| `swimlane-beta` | yes | yes | yes | yes |
| `usecase-beta` | yes | yes | yes | yes |
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

Every diagram type mermaid's syntax reference documents is drawn. Four of them
— `swimlane-beta`, `usecase-beta`, `treeView-beta` and `eventmodeling` — are
graphs of nodes, edges and groups, and reuse the layouts that already draw
those. Four more — `cynefin-beta`, `ishikawa-beta`, `venn-beta` and
`wardley-beta` — needed geometry of their own, because position *is* what they
say: a domain field, a fishbone's spine and bones, overlapping circles, a
value chain against an evolution axis. Each has its own layout module.

A header this renderer has no parser for — a typo, or a diagram type newer
than this crate — fails with `unsupported Mermaid diagram type: <header>`.
Before markdoc-mermaid 0.3.0 a word ending in `-beta` instead drew a two-box
flowchart named after the halves of itself, because the lone `-` in it read as
a link, which is the worse answer of the two: see change 3 in
`vendor/mermaid-rs-renderer/VENDORED.md`.

Four places where the drawing is honestly simpler than mermaid's own, all of
them noted here rather than in the table: a `treeView` is a tree of boxes
where mermaid draws an indented list with elbow connectors; a swimlane's lanes
are `subgraph` boxes rather than full-width bands; a `cynefin` field has a
straight boundary between its halves where mermaid draws a wavy one; and a
`venn` of four or more sets places its circles on a ring, which draws every
set but cannot realise every intersection — no arrangement of circles can,
which is why four-set Venn diagrams are normally drawn with ellipses.

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

## The renderer

Vendored rather than depended on, as of 0.3.0:
[mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer) 0.3.1,
MIT, published from here as `markdoc-mermaid-renderer`.
`vendor/mermaid-rs-renderer/VENDORED.md` records the upstream revision and
every change, each marked in the source.

It was chosen for two reasons that are not about features. It is MIT, where
`flowmaid` — the better engine by most measures — is GPL-3.0-or-later and would
pass that on to anything linking it. And it emits real `<text>` rather than
`<foreignObject>`, which decides whether a diagram printed into a PDF is
searchable text or a picture of text; for a manual read offline that is the
difference between a working document and a screenshot. The CLI and PNG stacks
are not vendored at all: this crate wants SVG and nothing else.

## License

MIT
