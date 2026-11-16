//! Which nodes each edge joins: as data, and in the SVG.
//!
//! Read by the renderer's own parser, from the same expanded source
//! [`render`](crate::render) draws, so the graph and the picture cannot
//! disagree.
//!
//! [`flowchart`] -- the graph *as data* -- is flowcharts only, because
//! [`Flowchart`] describes a flowchart and nothing else. `draw`, which
//! names the nodes and edge ends in the SVG, is not: it works from the
//! layout's ids, which every kind has. Where a kind's renderer draws its
//! shapes without going through a node's link there is nowhere to put the
//! marks, and `draw` finds them missing and emits what the renderer drew,
//! so a kind it cannot name is never a kind it damages.

use mermaid_rs_renderer::{DiagramKind, Graph, Layout, LayoutConfig, NodeLink, Theme};

use crate::{Anchors, expand};

/// A flowchart as data: what [`render`](crate::render) draws, without the
/// drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Flowchart {
    /// Every node drawn, in the order the source first names it.
    pub nodes: Vec<Node>,
    /// Every edge, in source order: a chain `a --> b --> c` is two, and so
    /// is `a --> b & c`.
    pub edges: Vec<Edge>,
    /// Every subgraph, in source order.
    pub subgraphs: Vec<Subgraph>,
}

/// One box of a flowchart.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Node {
    /// The id the source names it by, and its `data-node-id` in the SVG.
    pub id: String,
    /// Its text: the heading, for an id the anchors resolved; otherwise the
    /// label the author wrote, or the id itself. A line break is `\n`.
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
    /// Its `data-edge-id` in the SVG: `edge-N`, N counting the edges in
    /// source order from 0.
    pub id: String,
    /// The node the edge is written from (on the left of the link). A
    /// subgraph's id, or its title, when the edge leaves a subgraph.
    pub from: String,
    /// The node the edge is written to (on the right of the link). A
    /// subgraph's id, or its title, when the edge enters a subgraph.
    pub to: String,
    /// The text on the edge, in any of Mermaid's forms (`-- text -->`,
    /// `-->|text|`, `-. text .->`, …); `None` when it has none.
    pub label: Option<String>,
}

/// A subgraph of a flowchart.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Subgraph {
    /// Its id, when the source gives one. An edge to or from the subgraph
    /// names it by this, or by its title, which the renderer accepts too.
    pub id: Option<String>,
    /// The title drawn on it.
    pub label: String,
    /// The ids of the nodes inside it, a nested subgraph's included.
    pub nodes: Vec<String>,
}

/// The graph of a flowchart, without drawing it.
///
/// Expanded with `anchors` as [`render`](crate::render) expands it, so the
/// nodes, edges and ids are the ones the SVG draws. `None` for an empty
/// source, one the renderer refuses, and any other diagram type.
///
/// ```
/// use markdoc_mermaid::{Anchor, flowchart};
///
/// let anchors = |id: &str| {
///     (id == "can_ping").then(|| Anchor {
///         label: "Can you ping the switch?".into(),
///         href: "#can_ping".into(),
///     })
/// };
/// let graph = flowchart(
///     "flowchart TD\n  can_ping -- Ping failed --> power & cable\n",
///     &anchors,
/// )
/// .expect("a flowchart");
///
/// assert_eq!(graph.nodes[0].label, "Can you ping the switch?");
/// let edge = &graph.edges[1];
/// assert_eq!(edge.id, "edge-1"); // its data-edge-id in the SVG
/// assert_eq!((edge.from.as_str(), edge.to.as_str()), ("can_ping", "cable"));
/// assert_eq!(edge.label.as_deref(), Some("Ping failed"));
/// ```
pub fn flowchart(source: &str, anchors: &dyn Anchors) -> Option<Flowchart> {
    if source.trim().is_empty() {
        return None;
    }
    let probe = mermaid_rs_renderer::parse_mermaid_strict(source).ok()?;
    if probe.graph.kind != DiagramKind::Flowchart {
        return None;
    }
    // Links change nothing about the graph, so none are asked for.
    let nodes: Vec<(String, String)> = probe
        .graph
        .nodes
        .iter()
        .map(|(id, node)| (id.clone(), node.label.clone()))
        .collect();
    let expanded = expand::expand(source, anchors, false, probe.graph.kind, &nodes);
    let parsed = if expanded.resolved.is_empty() {
        probe
    } else {
        mermaid_rs_renderer::parse_mermaid_strict(&expanded.source).ok()?
    };
    let graph = &parsed.graph;
    let heading = |id: &str| {
        expanded
            .resolved
            .iter()
            .any(|r| r == id)
            .then(|| anchors.get(id))
            .flatten()
            .map(|a| a.label)
    };

    let mut order: Vec<(&str, usize)> = graph
        .node_order
        .iter()
        .filter(|(id, _)| graph.nodes.contains_key(*id))
        .map(|(id, at)| (id.as_str(), *at))
        .collect();
    order.sort_by_key(|&(_, at)| at);
    let mut ids: Vec<&str> = order.into_iter().map(|(id, _)| id).collect();
    // A node only an edge names is still drawn: the renderer adds it before
    // layout, after every node it already had.
    for edge in &graph.edges {
        for end in [edge.from.as_str(), edge.to.as_str()] {
            if !ids.contains(&end) {
                ids.push(end);
            }
        }
    }
    let anchored = subgraph_anchors(graph, &ids);
    let nodes = ids
        .iter()
        .copied()
        .filter(|id| !anchored.contains(id))
        .map(|id| Node {
            id: id.to_string(),
            label: heading(id)
                .unwrap_or_else(|| plain(graph.nodes.get(id).map_or(id, |n| n.label.as_str()))),
        })
        .collect();
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .map(|(n, e)| Edge {
            id: edge_id(n),
            from: e.from.clone(),
            to: e.to.clone(),
            label: e.label.as_deref().map(plain),
        })
        .collect();
    let subgraphs = graph
        .subgraphs
        .iter()
        .map(|s| Subgraph {
            id: s.id.clone(),
            label: plain(&s.label),
            // The renderer counts a nested subgraph's stand-in as a member
            // when an edge inside names it; it is not a node.
            nodes: s
                .nodes
                .iter()
                .filter(|n| !anchored.contains(&n.as_str()))
                .cloned()
                .collect(),
        })
        .collect();
    Some(Flowchart {
        nodes,
        edges,
        subgraphs,
    })
}

/// The id the renderer gives the Nth edge (`edge_dom_id` in
/// `mermaid-rs-renderer`). The tests hold the two together.
fn edge_id(n: usize) -> String {
    format!("edge-{n}")
}

/// The nodes that stand for a subgraph, such as `s` in `a --> s` when `s` is
/// a subgraph's id. The renderer draws no box for them; an edge to one ends
/// at the subgraph's box. By its rules, a node named by the subgraph's id or
/// title, and not inside it, stands for it: the first of the two is hidden
/// (`subgraph_anchor_id`), and both are skipped once some subgraph with that
/// title has a box, which it has when a node is inside it
/// (`apply_subgraph_anchors`).
fn subgraph_anchors<'a>(graph: &'a Graph, ids: &[&str]) -> Vec<&'a str> {
    let mut anchors = Vec::new();
    for sub in &graph.subgraphs {
        let outside = |name: &&str| ids.contains(name) && !sub.nodes.iter().any(|n| n == name);
        let names = sub.id.as_deref().into_iter().chain([sub.label.as_str()]);
        let boxed = graph
            .subgraphs
            .iter()
            .any(|s| s.label == sub.label && !s.nodes.is_empty());
        anchors.extend(names.filter(outside).take(if boxed { 2 } else { 1 }));
    }
    anchors
}

/// A label as text: the renderer breaks a line at `<br>`, `<br/>` and a
/// written `\n`, and trims each line.
fn plain(label: &str) -> String {
    label
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("\\n", "\n")
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Draw a flowchart with its nodes and edge ends named in the SVG.
///
/// The renderer draws a node's shapes and text loose in the SVG, unless the
/// node is a link: then they are inside its `<a>`. So for one render, every
/// node without a link is given one that cannot be mistaken for a real
/// href, each `<a>` is matched to its node by position (nodes are drawn in
/// id order), and each is then turned into, or wrapped in, a
/// `<g class="node" data-node-id>`. If any `<a>` is not where it is
/// expected, the diagram is drawn again as before, and only the edges are
/// annotated: a node left unnamed is better than a node misnamed.
pub(crate) fn draw(mut layout: Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let had_links = layout.nodes.values().any(|n| n.link.is_some());
    let mut drawn: Vec<(String, Option<String>)> = Vec::new();
    for node in layout
        .nodes
        .values_mut()
        .filter(|n| !n.hidden && n.anchor_subgraph.is_none())
    {
        let real = node.link.as_ref().map(|l| l.url.clone());
        if real.is_none() {
            node.link = Some(NodeLink {
                url: placeholder(drawn.len()),
                title: None,
                target: None,
            });
        }
        drawn.push((node.id.clone(), real));
    }
    let marked = mermaid_rs_renderer::render_svg(&layout, theme, config);
    let svg = match name_nodes(&marked, &drawn) {
        Some(svg) if had_links => svg,
        // The placeholders made the renderer declare the xlink namespace;
        // without a real link it was not there before.
        Some(svg) => svg.replacen(XLINK, "", 1),
        None => {
            for (node, (_, real)) in layout
                .nodes
                .values_mut()
                .filter(|n| !n.hidden && n.anchor_subgraph.is_none())
                .zip(&drawn)
            {
                if real.is_none() {
                    node.link = None;
                }
            }
            mermaid_rs_renderer::render_svg(&layout, theme, config)
        }
    };
    name_edge_ends(&svg, &layout)
}

const XLINK: &str = " xmlns:xlink=\"http://www.w3.org/1999/xlink\"";

fn placeholder(n: usize) -> String {
    format!("markdoc-mermaid:node:{n}")
}

/// Turn each node's `<a>` into, or wrap it in, its named group. `None` if
/// the links are not the ones the nodes were given, in the order given.
fn name_nodes(svg: &str, drawn: &[(String, Option<String>)]) -> Option<String> {
    let mut out = String::with_capacity(svg.len() + drawn.len() * 48);
    let mut at = 0usize;
    for (n, (id, real)) in drawn.iter().enumerate() {
        let open = at + svg[at..].find("<a ")?;
        let body = open + svg[open..].find('>')? + 1;
        let close = body + svg[body..].find("</a>")?;
        let end = close + "</a>".len();
        out.push_str(&svg[at..open]);
        out.push_str(&format!(
            "<g class=\"node\" data-node-id=\"{}\">",
            escape(id)
        ));
        match real {
            None => {
                let url = placeholder(n);
                if svg[open..body] != format!("<a href=\"{url}\" xlink:href=\"{url}\">") {
                    return None;
                }
                out.push_str(&svg[body..close]);
            }
            Some(url) => {
                if !svg[open..body].starts_with(&format!("<a href=\"{}\"", escape(url))) {
                    return None;
                }
                out.push_str(&svg[open..end]);
            }
        }
        out.push_str("</g>");
        at = end;
    }
    if svg[at..].contains("<a ") {
        return None;
    }
    out.push_str(&svg[at..]);
    Some(out)
}

/// Follow every `data-edge-id` with the ids of the nodes that edge joins.
fn name_edge_ends(svg: &str, layout: &Layout) -> String {
    const ID: &str = "data-edge-id=\"";
    let mut out = String::with_capacity(svg.len() + layout.edges.len() * 96);
    let mut rest = svg;
    while let Some(at) = rest.find(ID) {
        let value = at + ID.len();
        let Some(len) = rest[value..].find('"') else {
            break;
        };
        let end = value + len + 1;
        out.push_str(&rest[..end]);
        let edge = rest[value..value + len]
            .strip_prefix("edge-")
            .and_then(|n| n.parse::<usize>().ok())
            .and_then(|n| layout.edges.get(n));
        if let Some(edge) = edge {
            out.push_str(&format!(
                " data-from=\"{}\" data-to=\"{}\"",
                escape(&edge.from),
                escape(&edge.to)
            ));
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// An attribute value, escaped as the renderer escapes its own.
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Anchor, Options, render};

    fn none(_: &str) -> Option<Anchor> {
        None
    }

    fn graph(source: &str) -> Flowchart {
        flowchart(source, &none).expect("a flowchart")
    }

    /// `(id, from, to, label)` for each edge, to compare in one go.
    fn edges(graph: &Flowchart) -> Vec<(&str, &str, &str, Option<&str>)> {
        graph
            .edges
            .iter()
            .map(|e| {
                (
                    e.id.as_str(),
                    e.from.as_str(),
                    e.to.as_str(),
                    e.label.as_deref(),
                )
            })
            .collect()
    }

    fn node_ids(graph: &Flowchart) -> Vec<&str> {
        graph.nodes.iter().map(|n| n.id.as_str()).collect()
    }

    fn svg(source: &str) -> String {
        render(source, &none, &Options::default())
            .expect("renders")
            .svg
    }

    #[test]
    fn an_edge_label_is_read_in_each_of_its_forms() {
        let g = graph(
            "flowchart TD\n  a --> b\n  a -- \"quoted\" --> c\n  a -->|piped| d\n  a -- bare words --> e\n",
        );
        assert_eq!(node_ids(&g), vec!["a", "b", "c", "d", "e"]);
        assert_eq!(
            edges(&g),
            vec![
                ("edge-0", "a", "b", None),
                ("edge-1", "a", "c", Some("quoted")),
                ("edge-2", "a", "d", Some("piped")),
                ("edge-3", "a", "e", Some("bare words")),
            ]
        );
    }

    #[test]
    fn a_chain_is_one_edge_per_link() {
        let g = graph("flowchart LR\n  a --> b --> c\n  c -.-> d ==> e --- f\n");
        assert_eq!(
            edges(&g),
            vec![
                ("edge-0", "a", "b", None),
                ("edge-1", "b", "c", None),
                ("edge-2", "c", "d", None),
                ("edge-3", "d", "e", None),
                ("edge-4", "e", "f", None),
            ]
        );
    }

    #[test]
    fn an_ampersand_fans_out_and_in() {
        let g = graph("flowchart TD\n  a --> b & c\n  b & c --> d\n");
        assert_eq!(
            edges(&g),
            vec![
                ("edge-0", "a", "b", None),
                ("edge-1", "a", "c", None),
                ("edge-2", "b", "d", None),
                ("edge-3", "c", "d", None),
            ]
        );
    }

    #[test]
    fn every_link_style_is_an_edge() {
        let g = graph(
            "flowchart TD\n  a --- b\n  a -.-> c\n  a ==> d\n  a --o e\n  a --x f\n  a <--> g\n  \
             a -. dotted .-> h\n  a == thick ==> i\n  a ---> j\n",
        );
        assert_eq!(
            edges(&g),
            vec![
                ("edge-0", "a", "b", None),
                ("edge-1", "a", "c", None),
                ("edge-2", "a", "d", None),
                ("edge-3", "a", "e", None),
                ("edge-4", "a", "f", None),
                ("edge-5", "a", "g", None),
                ("edge-6", "a", "h", Some("dotted")),
                ("edge-7", "a", "i", Some("thick")),
                ("edge-8", "a", "j", None),
            ]
        );
    }

    #[test]
    fn an_edge_can_join_a_subgraph_which_is_not_a_node() {
        let g = graph(
            "flowchart TD\n  subgraph power [Power supply]\n    a --> b\n  end\n  c --> power\n  power --> d\n",
        );
        assert_eq!(node_ids(&g), vec!["a", "b", "c", "d"]);
        assert_eq!(
            edges(&g),
            vec![
                ("edge-0", "a", "b", None),
                ("edge-1", "c", "power", None),
                ("edge-2", "power", "d", None),
            ]
        );
        assert_eq!(
            g.subgraphs,
            vec![Subgraph {
                id: Some("power".into()),
                label: "Power supply".into(),
                nodes: vec!["a".into(), "b".into()],
            }]
        );
        // The SVG agrees: no box for it, and the edges name it.
        let svg = svg(
            "flowchart TD\n  subgraph power [Power supply]\n    a --> b\n  end\n  c --> power\n  power --> d\n",
        );
        assert!(!svg.contains("data-node-id=\"power\""), "{svg}");
        assert!(svg.contains("data-from=\"c\" data-to=\"power\""), "{svg}");
    }

    #[test]
    fn a_subgraph_named_by_its_id_and_its_title_is_still_not_a_node() {
        // The renderer draws a box for neither name.
        let source = "flowchart TD\n  subgraph net [Network]\n    a --> b\n  end\n  x --> net\n  \
                      y --> Network\n";
        let g = graph(source);
        assert_eq!(node_ids(&g), vec!["a", "b", "x", "y"]);
        // The renderer's own debug checks reject this layout, so the SVG is
        // compared in a release build only (`cargo test --release`).
        #[cfg(not(debug_assertions))]
        {
            let svg = svg(source);
            assert_eq!(
                svg.matches("<g class=\"node\"").count(),
                g.nodes.len(),
                "{svg}"
            );
            assert!(!svg.contains("data-node-id=\"Network\""), "{svg}");
        }
        // An empty subgraph has no box, so only the first name stands for it.
        let g = graph("flowchart TD\n  subgraph e [E]\n  end\n  a --> e\n  b --> E\n");
        assert_eq!(node_ids(&g), vec!["a", "b", "E"]);
        // A nested subgraph's stand-in is not a member of the one around it.
        let g = graph(
            "flowchart TD\n  subgraph outer\n    subgraph inner\n      c\n    end\n    \
             d --> inner\n  end\n",
        );
        assert_eq!(node_ids(&g), vec!["c", "d"]);
        assert_eq!(g.subgraphs[0].nodes, vec!["c", "d"]);
        assert_eq!(g.subgraphs[1].nodes, vec!["c"]);
    }

    #[test]
    fn a_label_is_the_heading_the_authors_text_or_the_id() {
        let anchors = |id: &str| {
            (id == "step_one").then(|| Anchor {
                label: "Is the \"Power\" light on?".into(),
                href: "#step_one".into(),
            })
        };
        let g = flowchart(
            "flowchart TD\n  step_one --> b[Two<br/>lines] --> c\n",
            &anchors,
        )
        .expect("a flowchart");
        let labels: Vec<&str> = g.nodes.iter().map(|n| n.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["Is the \"Power\" light on?", "Two\nlines", "c"]
        );
    }

    #[test]
    fn only_a_flowchart_has_a_graph() {
        assert_eq!(flowchart("", &none), None);
        assert_eq!(flowchart("   \n", &none), None);
        assert_eq!(flowchart("sequenceDiagram\n  A->>B: hi\n", &none), None);
        assert_eq!(flowchart("stateDiagram-v2\n  [*] --> A\n", &none), None);
        assert_eq!(flowchart("not a diagram at all", &none), None);
        // `graph` is the older keyword for the same thing.
        assert_eq!(graph("graph LR\n  a --> b\n").edges.len(), 1);
    }

    #[test]
    fn every_edge_is_drawn_under_the_id_the_graph_gives_it() {
        // A chain and a fan-out, where a parser of one's own most easily
        // counts differently from the renderer.
        for source in [
            "flowchart TD\n  a --> b --> c\n  c -- \"yes\" --> d\n  c -->|no| a\n",
            "flowchart TD\n  q -- \"one\" --> a & b\n  a & b --> done\n",
        ] {
            let g = graph(source);
            let svg = svg(source);
            assert_eq!(
                svg.matches("class=\"edgePath\"").count(),
                g.edges.len(),
                "{svg}"
            );
            for e in &g.edges {
                let ends = format!(
                    "data-edge-id=\"{}\" data-from=\"{}\" data-to=\"{}\"",
                    e.id, e.from, e.to
                );
                assert!(
                    svg.contains(&format!("class=\"edgePath\" {ends}")),
                    "{}: {svg}",
                    e.id
                );
                if let Some(label) = &e.label {
                    let open = format!("<g class=\"edgeLabel\" {ends}");
                    let at = svg.find(&open).unwrap_or_else(|| panic!("{}: {svg}", e.id));
                    let group = &svg[at..at + svg[at..].find("</g>").unwrap()];
                    assert!(group.contains(label.as_str()), "{}: {group}", e.id);
                }
            }
        }
    }

    #[test]
    fn every_node_is_drawn_once_in_a_group_named_for_it() {
        let source =
            "flowchart TD\n  a[First] --> b{Second?}\n  b -->|yes| c((Third))\n  b -->|no| d\n";
        let g = graph(source);
        let svg = svg(source);
        assert_eq!(
            svg.matches("<g class=\"node\"").count(),
            g.nodes.len(),
            "{svg}"
        );
        for n in &g.nodes {
            let open = format!("<g class=\"node\" data-node-id=\"{}\">", n.id);
            assert_eq!(svg.matches(&open).count(), 1, "{}: {svg}", n.id);
            // The group holds the node's text, and ends with it.
            let at = svg.find(&open).unwrap() + open.len();
            let group = &svg[at..at + svg[at..].find("</g>").unwrap()];
            assert!(
                group.contains(&format!(">{}</tspan>", n.label)),
                "{}: {group}",
                n.id
            );
            assert!(group.ends_with("</text>"), "{}: {group}", n.id);
        }
    }

    #[test]
    fn a_node_link_stays_a_link_inside_its_group() {
        let anchors = |id: &str| {
            (id == "step_two").then(|| Anchor {
                label: "Can you ping the switch?".into(),
                href: "#step_two".into(),
            })
        };
        let out = render(
            "flowchart TD\n  step_one --> step_two\n  click step_one \"/elsewhere\" \"Elsewhere\"\n",
            &anchors,
            &Options::default(),
        )
        .expect("renders");
        assert!(
            out.svg.contains(
                "<g class=\"node\" data-node-id=\"step_two\"><a href=\"#step_two\" xlink:href=\"#step_two\">"
            ),
            "{}",
            out.svg
        );
        assert!(
            out.svg
                .contains("<g class=\"node\" data-node-id=\"step_one\"><a href=\"/elsewhere\""),
            "{}",
            out.svg
        );
        assert!(out.svg.contains("</a></g>"), "{}", out.svg);
        assert!(out.svg.contains("xmlns:xlink"), "{}", out.svg);
    }

    #[test]
    fn nothing_of_the_marking_is_left_behind() {
        // Without a real link the root has no xlink namespace, as before,
        // and no placeholder href reaches the page.
        let out = svg("flowchart TD\n  a --> b\n");
        assert!(!out.contains("xmlns:xlink"), "{out}");
        assert!(!out.contains("<a "), "{out}");
        assert!(!out.contains("markdoc-mermaid:node:"), "{out}");
    }

    #[test]
    fn ids_are_escaped_in_the_svg() {
        let source = "flowchart TD\n  a<b --> c>d\n";
        let g = graph(source);
        assert_eq!(edges(&g), vec![("edge-0", "a<b", "c>d", None)]);
        let out = svg(source);
        assert!(
            out.contains("data-from=\"a&lt;b\" data-to=\"c&gt;d\""),
            "{out}"
        );
        assert!(out.contains("data-node-id=\"a&lt;b\""), "{out}");
        assert!(out.contains("data-node-id=\"c&gt;d\""), "{out}");
        assert!(!out.contains("a<b"), "{out}");
        assert_eq!(escape("&\"'<>"), "&amp;&quot;&apos;&lt;&gt;");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn the_graph_serialises_as_a_page_would_ship_it() {
        let g = graph("flowchart TD\n  a -- \"yes\" --> b\n  a --> c\n");
        assert_eq!(
            serde_json::to_value(&g).unwrap(),
            serde_json::json!({
                "nodes": [
                    { "id": "a", "label": "a" },
                    { "id": "b", "label": "b" },
                    { "id": "c", "label": "c" },
                ],
                "edges": [
                    { "id": "edge-0", "from": "a", "to": "b", "label": "yes" },
                    { "id": "edge-1", "from": "a", "to": "c", "label": null },
                ],
                "subgraphs": [],
            })
        );
    }

    #[test]
    fn the_drawing_is_otherwise_exactly_the_renderers() {
        // Take the names out again and what is left is byte for byte what
        // the renderer drew before this module existed.
        let source = "flowchart TD\n  a[Start] --> b{Choose}\n  b -- \"left\" --> c\n  b -->|right| d\n  \
                      click a \"#a\" \"Start\"\n  subgraph s [Group]\n    c\n  end\n";
        let plain_options = Options {
            themeable: false,
            ..Options::default()
        };
        let ours = render(source, &none, &plain_options).expect("renders").svg;

        let parsed = mermaid_rs_renderer::parse_mermaid_strict(source).unwrap();
        let theme = mermaid_rs_renderer::Theme::modern();
        let config = crate::Layout::default().to_config();
        let mut layout = mermaid_rs_renderer::compute_layout(&parsed.graph, &theme, &config);
        crate::route::tidy(&mut layout, parsed.graph.direction);
        let theirs = mermaid_rs_renderer::render_svg(&layout, &theme, &config);

        let strip = |svg: &str| {
            let mut out = svg.to_string();
            while let Some(at) = out.find(" data-from=\"") {
                let to = at + out[at..].find(" data-to=\"").unwrap() + " data-to=\"".len();
                let end = to + out[to..].find('"').unwrap() + 1;
                out.replace_range(at..end, "");
            }
            // A node group holds no group of its own: its `</g>` is the
            // first one after it.
            while let Some(at) = out.find("<g class=\"node\"") {
                let open = at + out[at..].find('>').unwrap() + 1;
                let close = open + out[open..].find("</g>").unwrap();
                out.replace_range(close..close + "</g>".len(), "");
                out.replace_range(at..open, "");
            }
            out
        };
        assert_eq!(strip(&ours), theirs);
        assert!(ours.contains("data-from"), "{ours}");
    }
}
