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
syntax reference lists 31 diagram types; 27 of them draw here.

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
| `cynefin-beta` | **no** |  |  |  |
| `ishikawa-beta` | **no** |  |  |  |
| `venn-beta` | **no** |  |  |  |
| `wardley-beta` | **no** |  |  |  |

The four at the bottom have no parser here and fail with
`unsupported Mermaid diagram type: <header>`. Each needs geometry of its own —
a 2x2 domain field, a fishbone spine, circle intersections, an evolution axis —
and drawn as boxes and arrows each would lose the only thing it says. That is
the line between them and `swimlane-beta`, `usecase-beta`, `treeView-beta` and
`eventmodeling`, which markdoc-mermaid 0.3.0 added: those four are nodes, edges
and groups, which the existing layouts already draw.
`vendor/mermaid-rs-renderer/VENDORED.md` says what each of the remaining four
would need.

Before 0.3.0 none of the eight worked, and seven of them failed silently: a
lone `-` read as a link, so each `*-beta` header drew a two-box flowchart named
after the halves of itself. Only `eventmodeling`, which has no hyphen, said so.
See change 3 in `VENDORED.md`.

Two caveats on the four that were added. A `treeView` is drawn as a tree of
boxes, where mermaid draws an indented list with elbow connectors — the same
hierarchy, a different picture. A swimlane's lanes are `subgraph` boxes rather
than full-width bands.

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
