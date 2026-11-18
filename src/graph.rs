//! Which nodes each edge joins: as data.
//!
//! Read from merman's own semantic model, parsed from the same expanded
//! source [`render`](crate::render) draws, so the graph and the picture
//! cannot disagree. Flowcharts only; the SVG itself names nodes and edges
//! for every kind merman draws, with `data-id` and `data-et`.

use merman::model::SemanticDiagramModel;
use merman::model::families::flowchart::{FlowNodeProvenance, FlowchartModel};

use crate::{Anchors, expand};

/// A flowchart as data: what [`render`](crate::render) draws, without the
/// drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Flowchart {
    /// Every node the source names, in its order. A node merman synthesised
    /// to anchor a subgraph edge is not one of them
    /// (`FlowNodeProvenance::SubgraphAnchor`).
    pub nodes: Vec<Node>,
    /// Every edge, in source order: a chain `a --> b --> c` is two, labelled
    /// or not (`a -- yes --> b -- no --> c`), and so is `a --> b & c`.
    pub edges: Vec<Edge>,
    /// Every subgraph, in source order.
    pub subgraphs: Vec<Subgraph>,
}

/// One box of a flowchart.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Node {
    /// The id the source names it by, and its `data-id` in the SVG.
    pub id: String,
    /// Its text as drawn: the heading, for an id the anchors resolved;
    /// otherwise the label the author wrote, decoded. A line break is `\n`.
    pub label: String,
}

/// One edge of a flowchart.
///
/// `from` and `to` follow the source left to right, not the arrowhead:
/// `b <-- a` is an edge from `b` to `a`, as the renderer reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Edge {
    /// Its `data-id` in the SVG. Read `from` and `to` from the fields below
    /// rather than by splitting this: an id may contain `_`.
    pub id: String,
    /// The id of the node the edge leaves.
    pub from: String,
    /// The id of the node the edge reaches.
    pub to: String,
    /// The edge's own text, where it has one.
    pub label: Option<String>,
}

/// One subgraph of a flowchart.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Subgraph {
    /// The id the source names it by, where it named one.
    pub id: Option<String>,
    /// Its title as drawn.
    pub label: String,
    /// The ids of the nodes inside it.
    pub nodes: Vec<String>,
}

/// The graph of a flowchart, with the same labels [`render`](crate::render)
/// would draw: ids resolved through `anchors`, exactly as the picture has
/// them.
///
/// `None` when the source does not parse, or is not a flowchart.
pub fn flowchart(source: &str, anchors: &dyn Anchors) -> Option<Flowchart> {
    // Parse only: no kind, and the seed decides nothing that is not drawn.
    let renderer = crate::engine(&crate::Layout::default(), false, "", 0);
    let probe = crate::parse(&renderer, source).ok()?;
    let nodes: Vec<(String, String)> = crate::labelled_nodes(probe.model());
    let expanded = expand::expand(source, anchors, false, probe.diagram_type(), &nodes);
    let parsed = if expanded.resolved.is_empty() {
        probe
    } else {
        crate::parse(&renderer, &expanded.source).ok()?
    };
    match parsed.model() {
        SemanticDiagramModel::Flowchart(model) => {
            Some(of_model(model, anchors, &expanded.resolved))
        }
        _ => None,
    }
}

/// The label the picture draws: the anchor's own text for an id `expand`
/// resolved, and merman's model decoded for anything else.
///
/// Resolved, not merely known: `expand` leaves an id the author labelled
/// alone, so the anchors alone would report a label the diagram does not
/// draw.
fn drawn_label(id: &str, model_label: &str, anchors: &dyn Anchors, resolved: &[String]) -> String {
    if resolved.iter().any(|it| it == id)
        && let Some(anchor) = anchors.get(id)
    {
        return anchor.label;
    }
    decode_html(model_label)
}

/// The named and numeric references Mermaid's own label text can carry,
/// turned back into the characters they stand for.
fn decode_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        // `find`, not a slice: `rest[..12]` panics where byte 12 falls
        // inside a character.
        let Some(end) = rest.find(';').filter(|&end| end < 12) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..end];
        let decoded = match name {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            numeric if numeric.starts_with('#') => {
                numeric[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            // Not a reference this knows: left as written, stepping past the
            // `&` so the scan makes progress.
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn of_model(model: &FlowchartModel, anchors: &dyn Anchors, resolved: &[String]) -> Flowchart {
    Flowchart {
        nodes: model
            .nodes
            .iter()
            .filter(|node| matches!(node.provenance, FlowNodeProvenance::Authored))
            .map(|node| Node {
                id: node.id.clone(),
                label: drawn_label(
                    &node.id,
                    node.label.as_deref().unwrap_or(&node.id),
                    anchors,
                    resolved,
                ),
            })
            .collect(),
        edges: model
            .edges
            .iter()
            .map(|edge| Edge {
                id: edge.id.clone(),
                from: edge.from.clone(),
                to: edge.to.clone(),
                label: edge
                    .label
                    .as_deref()
                    .filter(|label| !label.is_empty())
                    .map(decode_html),
            })
            .collect(),
        subgraphs: model
            .subgraphs
            .iter()
            .map(|group| Subgraph {
                id: (!group.id.is_empty()).then(|| group.id.clone()),
                label: group.title.clone(),
                nodes: group.nodes.clone(),
            })
            .collect(),
    }
}
