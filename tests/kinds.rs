//! What each diagram kind gets from this crate.
//!
//! Resolution needs a declaration form the diagram's own grammar accepts, and
//! most grammars have none. The property these tests hold is that
//! [`Diagram::resolved`] and the picture agree: an id named there reached the
//! SVG as a heading, and a kind with no form names none. Until 0.3.0 the
//! flowchart form was appended to every kind, so `resolved` listed ids whose
//! labels went nowhere — the drift this crate exists to prevent, reported as
//! prevented.

use markdoc_mermaid::{Anchor, Anchors, Options, render, resolves};

const HEADING: &str = "Can you ping the switch?";

struct Index;
impl Anchors for Index {
    fn get(&self, id: &str) -> Option<Anchor> {
        matches!(id, "step_one" | "step_two").then(|| Anchor {
            label: HEADING.to_string(),
            href: format!("#{id}"),
        })
    }
}

/// A diagram per kind, each naming `step_one` by id and nothing else.
const SOURCES: &[(&str, &str)] = &[
    ("flowchart", "flowchart TD\n  step_one --> step_two\n"),
    ("sequence", "sequenceDiagram\n  step_one->>step_two: Ping\n"),
    ("class", "classDiagram\n  step_one <|-- step_two\n"),
    ("state", "stateDiagram-v2\n  step_one --> step_two\n"),
    ("block", "block-beta\n  columns 2\n  step_one step_two\n"),
    ("er", "erDiagram\n  step_one ||--o{ step_two : has\n"),
    ("mindmap", "mindmap\n  root((Root))\n    step_one\n"),
    ("kanban", "kanban\n  Todo\n    step_one\n"),
    ("treemap", "treemap-beta\n\"Root\"\n  \"step_one\": 10\n"),
];

/// The kinds with a declaration form, as `resolves` reports them.
const RESOLVING: &[&str] = &["flowchart", "sequence", "class", "state", "block"];

/// The SVG's text content, tags dropped and whitespace squeezed.
///
/// A label wider than its box is wrapped into one `<tspan>` per line, so
/// searching the markup for a heading finds nothing once the heading is long
/// enough to wrap -- which every real heading is.
fn text_of(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut depth = 0usize;
    for ch in svg.chars() {
        match ch {
            '<' => {
                // A wrapped label is one `<tspan>` per line with no space
                // between them, so the boundary has to supply one.
                out.push(' ');
                depth += 1;
            }
            '>' => depth = depth.saturating_sub(1),
            c if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn diagram(src: &str) -> markdoc_mermaid::Diagram {
    render(src, &Index, &Options::default()).expect("should render")
}

#[test]
fn resolved_ids_reach_the_picture() {
    for (kind, src) in SOURCES {
        let d = diagram(src);
        if d.resolved.is_empty() {
            continue;
        }
        assert!(
            text_of(&d.svg).contains(HEADING),
            "{kind}: resolved {:?} but the heading is not in the SVG",
            d.resolved
        );
    }
}

#[test]
fn a_kind_without_a_form_resolves_nothing() {
    for (kind, src) in SOURCES {
        let d = diagram(src);
        let expected = RESOLVING.contains(kind);
        assert_eq!(
            !d.resolved.is_empty(),
            expected,
            "{kind}: resolved {:?}, expected resolution: {expected}",
            d.resolved
        );
    }
}

#[test]
fn resolves_agrees_with_what_happens() {
    for (kind, src) in SOURCES {
        let parsed = mermaid_rs_renderer::parse_mermaid_strict(src).expect("should parse");
        assert_eq!(
            resolves(parsed.graph.kind),
            RESOLVING.contains(kind),
            "{kind}: `resolves` disagrees with the table"
        );
    }
}

#[test]
fn a_sequence_diagram_keeps_parsing_once_one_participant_is_named() {
    // Naming one participant obliges the kind to name them all: the validator
    // rejects a message to a participant no `participant` line declared, so a
    // diagram that parsed before resolution must still parse after it.
    let d = diagram("sequenceDiagram\n  step_one->>step_two: Ping\n  step_two->>step_one: Pong\n");
    assert_eq!(d.resolved, ["step_one", "step_two"]);
    assert!(text_of(&d.svg).contains(HEADING));
}

#[test]
fn click_is_withheld_from_a_parser_that_reads_it_as_content() {
    // For the block, ER and mindmap parsers a `click` line becomes nodes of
    // its own words, so it is never emitted for them. The flowchart gets one,
    // and so do the swimlane and use case diagrams, whose bodies are
    // flowcharts (`the_added_kinds_*`).
    let flow = diagram("flowchart TD\n  step_one --> step_two\n");
    assert!(
        flow.svg.contains("#step_one"),
        "the flowchart should carry its link"
    );
    for (kind, src) in SOURCES.iter().filter(|(k, _)| *k != "flowchart") {
        let d = diagram(src);
        assert!(
            !d.svg.contains("#step_one"),
            "{kind}: a click link reached a kind whose parser does not read one"
        );
        assert!(
            !d.svg.contains("click"),
            "{kind}: the word `click` reached the SVG, so it became content"
        );
    }
}

#[test]
fn nodes_and_edge_ends_are_named_beyond_the_flowchart() {
    // `draw` works from the layout's ids rather than the flowchart's, and
    // falls back to the renderer's own output where the marks cannot be
    // placed. These are the kinds where they can.
    for kind in ["flowchart", "sequence", "class", "state", "er", "mindmap"] {
        let (_, src) = SOURCES.iter().find(|(k, _)| *k == kind).expect("a source");
        let d = diagram(src);
        assert!(
            d.svg.contains("data-node-id"),
            "{kind}: no node is named in the SVG"
        );
    }
}

#[test]
fn an_unsupported_diagram_type_is_an_error_naming_itself() {
    // These four have no parser here; the other four mermaid documents do
    // now (`the_added_kinds_*` below). All of them used to draw a two-box
    // flowchart named after the halves of their own header, because a lone
    // `-` read as a link.
    for header in ["cynefin-beta", "ishikawa-beta", "venn-beta", "wardley-beta"] {
        let err = render(
            &format!("{header}\n  title Something\n"),
            &Index,
            &Options::default(),
        )
        .expect_err(&format!("{header} should not render"));
        let text = err.to_string();
        assert!(
            text.contains(header),
            "{header}: the error should name it, got {text}"
        );
    }
}

#[test]
fn a_hyphenated_anchor_id_survives() {
    // Documentation slugs are hyphenated, and a hyphen used to read as an
    // edge: `alpha-one --> beta-two` parsed as four nodes.
    struct Slugs;
    impl Anchors for Slugs {
        fn get(&self, id: &str) -> Option<Anchor> {
            matches!(id, "alpha-one" | "beta-two").then(|| Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    let d = render(
        "flowchart TD\n  alpha-one --> beta-two\n",
        &Slugs,
        &Options::default(),
    )
    .expect("should render");
    assert_eq!(d.resolved, ["alpha-one", "beta-two"]);
    let text = text_of(&d.svg);
    assert!(text.contains("Heading alpha-one"), "got {text}");
    assert!(text.contains("Heading beta-two"), "got {text}");
}

#[test]
fn radar_draws_its_labels_and_its_title() {
    // The axis and curve labels used to come through as raw `a["Alpha"]`, and
    // the title element was emitted with its text hardcoded empty.
    let d = render(
        "radar-beta\n  title Skills\n  axis a[\"Alpha\"], b[\"Beta\"]\n  curve x[\"Xcurve\"]{3, 4}\n",
        &Index,
        &Options::default(),
    )
    .expect("should render");
    let text = text_of(&d.svg);
    for want in ["Alpha", "Beta", "Xcurve", "Skills"] {
        assert!(text.contains(want), "radar is missing {want}, got {text}");
    }
    assert!(
        !d.svg.contains("[&quot;"),
        "the bracket syntax leaked into the picture"
    );
}

#[test]
fn a_block_diagram_takes_a_label_with_spaces_in_it() {
    // A block lists its nodes space-separated, and the splitter used to cut a
    // quoted label into tokens, so every multi-word heading became nodes.
    let d = diagram("block-beta\n  columns 2\n  step_one step_two\n");
    assert!(
        text_of(&d.svg).contains(HEADING),
        "the heading did not survive"
    );
    // Split into tokens it would have become four nodes, not two.
    assert_eq!(d.svg.matches("data-node-id").count(), 2);
}

#[test]
fn a_participant_the_author_declared_keeps_its_own_label() {
    // The author's label wins, and the scanner cannot see this one: it knows
    // the bracket forms, and `participant x as Name` puts the label somewhere
    // it never looks. The parse this crate does first is what knows.
    //
    // Both participants are declared because the renderer's validator refuses
    // a message to one that no `participant` line named -- stricter than
    // Mermaid itself, and the reason resolution declares them all.
    let d = render(
        "sequenceDiagram\n  participant step_one\n  participant step_two as Switch\n  step_one->>step_two: Ping\n",
        &Index,
        &Options::default(),
    )
    .expect("should render");
    let text = text_of(&d.svg);
    assert_eq!(d.resolved, ["step_one"], "step_two is the author's");
    assert!(
        text.contains("Switch"),
        "the author's label is gone: {text}"
    );
    assert!(
        text.contains(HEADING),
        "the resolved heading is gone: {text}"
    );
}

/// The four diagram types mermaid documents that this crate added, each a
/// graph of nodes, edges and groups. The other four it does not draw need
/// geometry of their own; `vendor/mermaid-rs-renderer/VENDORED.md` says what.
const ADDED: &[(&str, &str, &[&str])] = &[
    (
        "swimlane",
        "swimlane-beta LR\n  subgraph Customer\n    Browse[Browse catalogue]\n    Pay[Pay]\n  end\n  subgraph Warehouse\n    Pick[Pick items]\n  end\n  Browse --> Pay\n  Pay --> Pick\n",
        &["Browse catalogue", "Pick items", "Customer", "Warehouse"],
    ),
    (
        "usecase",
        "usecase-beta\ndirection LR\nactor Customer(\"Customer\")\nsystemBoundary \"Order system\"\n  Checkout(\"Place order\")\nend\nCustomer --> Checkout\n",
        &["Customer", "Place order", "Order system"],
    ),
    (
        "treeView",
        "treeView-beta\n    my-project/\n        src/\n            index.js\n        package.json\n",
        &["my-project/", "src/", "index.js", "package.json"],
    ),
    (
        "eventmodeling",
        "eventmodeling\n\ntf 01 ui CartUI\ntf 02 cmd AddItem\ntf 03 evt ItemAdded\n",
        &["CartUI", "AddItem", "ItemAdded", "ui", "cmd", "evt"],
    ),
];

#[test]
fn the_added_kinds_draw_their_content() {
    for (name, src, wants) in ADDED {
        let d = render(src, &Index, &Options::default())
            .unwrap_or_else(|e| panic!("{name} should render: {e}"));
        let text = text_of(&d.svg);
        for want in *wants {
            assert!(text.contains(want), "{name}: {want} is missing from {text}");
        }
        // The reason this renderer was chosen: a diagram printed into a PDF
        // has to be text, not a picture of text.
        assert!(
            !d.svg.contains("foreignObject"),
            "{name}: drew its text in a foreignObject"
        );
        assert!(d.svg.contains("<text"), "{name}: drew no text at all");
    }
}

#[test]
fn the_added_kinds_resolve_where_an_id_is_the_author_s_to_write() {
    // A swimlane and a use case diagram have flowchart bodies and ids the
    // author writes, so both resolve. TreeView and EventModeling derive a
    // node's id from its own text, so there is no id to match an anchor
    // against and they resolve nothing.
    struct Steps;
    impl Anchors for Steps {
        fn get(&self, id: &str) -> Option<Anchor> {
            (id == "Browse").then(|| Anchor {
                label: HEADING.to_string(),
                href: "#Browse".to_string(),
            })
        }
    }
    let expected = [
        ("swimlane-beta LR\n  Browse --> Pay\n", true),
        ("usecase-beta\nactor Browse\nBrowse --> Pay\n", true),
        ("treeView-beta\n    Browse\n", false),
        ("eventmodeling\n\ntf 01 ui Browse\n", false),
    ];
    for (src, resolving) in expected {
        let d = render(src, &Steps, &Options::default()).expect("should render");
        assert_eq!(
            !d.resolved.is_empty(),
            resolving,
            "{src:?} resolved {:?}",
            d.resolved
        );
        if resolving {
            assert!(
                text_of(&d.svg).contains(HEADING),
                "{src:?} lost the heading"
            );
        }
    }
}
