//! What each diagram kind gets from this crate.
//!
//! The property these tests hold is that `Diagram::resolved` and the picture
//! agree: an id named there reached the SVG as a heading, and a kind with no
//! declaration form names none.

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

/// The SVG's text content, tags dropped and whitespace squeezed: a label
/// wider than its box is wrapped into one `<tspan>` per line.
fn text_of(svg: &str) -> String {
    // The `<style>` block is text content too, and would land in here as
    // though it had been drawn.
    let svg = match (svg.find("<style>"), svg.find("</style>")) {
        (Some(open), Some(close)) if close > open => {
            format!("{}{}", &svg[..open], &svg[close + "</style>".len()..])
        }
        _ => svg.to_string(),
    };
    let svg = svg.as_str();
    let mut out = String::with_capacity(svg.len());
    let mut depth = 0usize;
    for ch in svg.chars() {
        match ch {
            '<' => {
                // One `<tspan>` per line with no space between them.
                out.push(' ');
                depth += 1;
            }
            '>' => depth = depth.saturating_sub(1),
            c if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    // Undo the XML escaping, to assert about a character that had to be
    // escaped.
    out.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
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
    // `resolves` answers on merman's own `diagram_type`, so the table is
    // checked against those rather than by re-parsing.
    for (kind, id) in [
        ("flowchart", "flowchart-v2"),
        ("block", "block"),
        ("state", "stateDiagram"),
        ("class", "classDiagram"),
        ("sequence", "sequence"),
    ] {
        assert!(
            resolves(id),
            "{kind}: `resolves({id})` should be true, as the table says"
        );
    }
    for id in [
        "pie", "gantt", "journey", "treemap", "radar", "er", "mindmap",
    ] {
        assert!(!resolves(id), "`resolves({id})` should be false");
    }
}

#[test]
fn a_sequence_diagram_keeps_parsing_once_one_participant_is_named() {
    // Naming one participant obliges the kind to name them all, so a
    // diagram that parsed before resolution must still parse after it.
    let d = diagram("sequenceDiagram\n  step_one->>step_two: Ping\n  step_two->>step_one: Pong\n");
    assert_eq!(d.resolved, ["step_one", "step_two"]);
    assert!(text_of(&d.svg).contains(HEADING));
}

#[test]
fn click_is_withheld_from_a_parser_that_reads_it_as_content() {
    // A `click` line is read as content by some parsers and rejected
    // outright by others, so it is emitted for the flowchart and the
    // swimlane and withheld everywhere else.
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
            !text_of(&d.svg).contains("click"),
            "{kind}: the word `click` was drawn, so it became content"
        );
    }
}

#[test]
fn nodes_and_edge_ends_are_named_beyond_the_flowchart() {
    // merman names them itself: `data-id` carries the source's own id, and
    // `data-et` says what the element is. Nothing in this crate adds them.
    //
    // `data-et` is not uniform, so this checks `data-id`, which is what a
    // page needs to address an element: `node` and `edge` for a flowchart,
    // `participant`/`message`/`life-line` for a sequence, and edges only for
    // class, state, ER and mindmap. A block diagram carries no `data-id` at
    // all, which is why it is not in this list.
    for kind in ["flowchart", "sequence", "class", "state", "er", "mindmap"] {
        let (_, src) = SOURCES.iter().find(|(k, _)| *k == kind).expect("a source");
        let d = diagram(src);
        assert!(
            d.svg.contains("data-id="),
            "{kind}: nothing is named in the SVG"
        );
    }
}

#[test]
fn an_unsupported_diagram_type_is_an_error_rather_than_a_guess() {
    // A word ending in `-beta` used to come back a two-box flowchart named
    // after the halves of itself, because the lone `-` in it read as a link.
    // merman's own answer names no diagram, so the header is quoted back
    // into the error.
    for header in ["notadiagram-beta", "sillywalk", "flowchartish-beta"] {
        let err = render(
            &format!("{header}\n  title Something\n"),
            &Index,
            &Options::default(),
        )
        .expect_err(&format!("{header} should not render"));
        let text = err.to_string();
        assert!(!text.is_empty(), "{header}: an error should say something");
    }
}

#[test]
fn a_hyphenated_anchor_id_survives() {
    // Documentation slugs are hyphenated, and a hyphen is not an edge.
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
    // The axis and curve labels, and the title.
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
    // A block lists its nodes space-separated, so a quoted multi-word
    // heading must not be cut into tokens.
    let d = diagram("block-beta\n  columns 2\n  step_one step_two\n");
    assert!(
        text_of(&d.svg).contains(HEADING),
        "the heading did not survive"
    );
    // merman emits no `data-id` for a block diagram, so the count is the
    // heading itself, drawn once per node.
    let drawn = text_of(&d.svg);
    assert_eq!(
        drawn.matches(HEADING).count(),
        2,
        "two nodes should carry the heading, in {drawn:?}"
    );
}

#[test]
fn a_participant_the_author_declared_keeps_its_own_label() {
    // The author's label wins, and only the parse can see this one:
    // `participant x as Name` puts it where the scanner never looks. Both
    // participants are declared, since a message to an undeclared one is
    // refused.
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

/// Four kinds that are each a graph of nodes, edges and groups.
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
        // merman draws a directory's name and its `/` as separate elements.
        &["my-project", "src", "index.js", "package.json"],
    ),
    (
        "eventmodeling",
        "eventmodeling\n\ntf 01 ui CartUI\ntf 02 cmd AddItem\ntf 03 evt ItemAdded\n",
        // merman draws what the lane keywords mean, not the keywords.
        &[
            "CartUI",
            "AddItem",
            "ItemAdded",
            "UI/Automation",
            "Command/Read Model",
            "Events",
        ],
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
        // A diagram printed into a PDF has to be text, not a picture of it.
        assert!(
            !d.svg.contains("foreignObject"),
            "{name}: drew its text in a foreignObject"
        );
        assert!(d.svg.contains("<text"), "{name}: drew no text at all");
    }
}

#[test]
fn the_added_kinds_resolve_where_an_id_is_the_author_s_to_write() {
    // A swimlane's body is a flowchart and its ids are the author's, so it
    // resolves. A use case diagram does not: in merman an id carries a
    // *kind*, and labelling one without re-declaring its kind is rejected.
    // TreeView and EventModeling derive a node's id from its own text, so
    // there is no id to match an anchor against.
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
        ("usecase-beta\nactor Browse\nBrowse --> Pay\n", false),
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

#[test]
fn a_heading_with_a_quotation_mark_in_it_draws_the_mark() {
    // `expand` writes a `"` as Mermaid's `#quot;`, since a bare one would
    // close the bracketed label early, and the renderer reads it back.
    struct Quoted;
    impl Anchors for Quoted {
        fn get(&self, id: &str) -> Option<Anchor> {
            (id == "step_one").then(|| Anchor {
                label: "He said \"no\" & left".into(),
                href: "#step_one".into(),
            })
        }
    }
    for (kind, src) in SOURCES.iter().filter(|(k, _)| RESOLVING.contains(k)) {
        let d = render(src, &Quoted, &Options::default())
            .unwrap_or_else(|e| panic!("{kind} should render: {e}"));
        assert_eq!(d.resolved, ["step_one"], "{kind}");
        let text = text_of(&d.svg);
        assert!(
            text.contains("He said \"no\" & left"),
            "{kind}: the heading is not drawn as written: {text}"
        );
        for leak in ["#quot;", "#amp;"] {
            assert!(
                !d.svg.contains(leak),
                "{kind}: {leak} reached the picture, in a label or a link title"
            );
        }
    }
}

#[test]
fn a_literal_hash_in_a_heading_survives() {
    // Only `#name;` and `#123;` are escapes, so a heading about C# or an
    // issue number has to come through as the author wrote it.
    //
    // merman decodes the longest valid entity *prefix* without requiring the
    // terminating semicolon, so `#notanentity;` would come out `¬anentity;`.
    // `quote` escapes the `#` itself, which is what keeps a resolved heading
    // safe from it. A label an author types is still exposed: that is
    // merman's to fix, and is reported.
    struct Hashes;
    impl Anchors for Hashes {
        fn get(&self, id: &str) -> Option<Anchor> {
            (id == "step_one").then(|| Anchor {
                label: "C# and issue #42 and #notanentity;".into(),
                href: "#step_one".into(),
            })
        }
    }
    let d = render(
        "flowchart TD\n  step_one --> step_two\n",
        &Hashes,
        &Options::default(),
    )
    .expect("should render");
    let text = text_of(&d.svg);
    assert!(
        text.contains("C# and issue #42 and #notanentity;"),
        "a literal hash did not survive: {text}"
    );
}

#[test]
fn mermaid_s_own_escapes_are_read_in_a_hand_written_label() {
    // An author writing Mermaid by hand, rather than through resolution, gets
    // the escapes too: these are the two examples in mermaid's own docs.
    let d = render(
        "flowchart TD\n  a[\"I #9829; you\"] --> b[\"#infin; times more\"]\n",
        &Index,
        &Options::default(),
    )
    .expect("should render");
    let text = text_of(&d.svg);
    assert!(text.contains("I \u{2665} you"), "got {text}");
    assert!(text.contains("\u{221E} times more"), "got {text}");
}

#[test]
fn a_heading_of_two_lines_is_drawn_on_two_lines() {
    // `quote` writes a newline as Mermaid's `<br/>`, for the same reason it
    // writes `#quot;`: the heading has to survive inside a bracketed label.
    struct TwoLines;
    impl Anchors for TwoLines {
        fn get(&self, id: &str) -> Option<Anchor> {
            (id == "step_one").then(|| Anchor {
                label: "Line one\nLine two".into(),
                href: "#step_one".into(),
            })
        }
    }
    let d = render(
        "flowchart TD\n  step_one --> step_two\n",
        &TwoLines,
        &Options::default(),
    )
    .expect("should render");
    // merman draws each line as its own `<text>` at its own `y`, rather than
    // as `<tspan>`s inside one.
    let lines: Vec<&str> = d
        .svg
        .split("<text")
        .skip(1)
        .filter_map(|s| s.split("</text>").next())
        .filter_map(|s| s.split_once('>').map(|(_, t)| t))
        .map(str::trim)
        .collect();
    assert!(
        lines.contains(&"Line one") && lines.contains(&"Line two"),
        "expected two lines, got {lines:?}"
    );
    // Not as the tag's own characters, in the label or the link's hover text.
    assert!(!d.svg.contains("br/"), "a <br/> reached the picture");
}

/// The geometry-driven kinds: a layout module of their own, because position
/// is what the diagram says rather than something to be computed.
const GEOMETRY: &[(&str, &str, &[&str])] = &[
    (
        "ishikawa",
        "ishikawa-beta\n    Blurry Photo\n    Process\n        Out of focus\n        Shutter speed too slow\n    User\n        Shaky hands\n    Equipment\n        LENS\n            Dirty lens\n    Environment\n        Too dark\n",
        &[
            "Blurry Photo",
            "Process",
            "User",
            "Equipment",
            "Environment",
            "Out of focus",
            "Shaky hands",
            "LENS",
            "Dirty lens",
            "Too dark",
        ],
    ),
    (
        "venn",
        "venn-beta\n  title What makes a good feature\n  set Desirable\n  set Feasible\n  set Viable\n  union Desirable,Feasible[\"Buildable\"]\n  union Feasible,Viable[\"Sustainable\"]\n  union Desirable,Viable[\"Marketable\"]\n  union Desirable,Feasible,Viable[\"Ship it\"]\n",
        &[
            "What makes a good feature",
            "Desirable",
            "Feasible",
            "Viable",
            "Buildable",
            "Sustainable",
            "Marketable",
            "Ship it",
        ],
    ),
    (
        "cynefin",
        "cynefin-beta\ntitle Our Situation\n\ncomplex\n\"Probe first\"\n\ncomplicated\n\"Expert analysis needed\"\n\nclear\n\"Known procedure\"\n\nchaotic\n\"Crisis response\"\n\nconfusion\n\"One\"\n\"Two\"\n\"Three\"\n\"Four\"\n\ncomplex --> complicated : \"Pattern identified\"\n",
        &[
            "Our Situation",
            "Complex",
            "Complicated",
            "Chaotic",
            "Clear",
            "Confusion",
            "Probe first",
            "Expert analysis needed",
            "Pattern identified",
            // The centre ellipse is small, so it shows three and counts the rest.
            "+1 more",
        ],
    ),
    (
        "wardley",
        "wardley-beta\ntitle Tea Shop Value Chain\n\nanchor Business [0.95, 0.63]\ncomponent Cup of Tea [0.79, 0.61]\ncomponent Kettle [0.43, 0.35]\ncomponent Power [0.10, 0.70]\n\nBusiness -> Cup of Tea\nKettle -> Power\n\nevolve Kettle 0.62\n\nnote \"Standardising power helps\" [0.30, 0.49]\n",
        &[
            "Tea Shop Value Chain",
            "Business",
            "Cup of Tea",
            "Kettle",
            "Power",
            "Genesis",
            "Commodity",
            "Visibility",
            "Evolution",
            "Standardising power helps",
        ],
    ),
];

#[test]
fn the_geometry_kinds_draw_their_content() {
    for (name, src, wants) in GEOMETRY {
        let d = render(src, &Index, &Options::default())
            .unwrap_or_else(|e| panic!("{name} should render: {e}"));
        let text = text_of(&d.svg);
        for want in *wants {
            assert!(text.contains(want), "{name}: {want} is missing from {text}");
        }
        assert!(
            !d.svg.contains("foreignObject"),
            "{name}: drew its text in a foreignObject"
        );
    }
}

#[test]
fn a_wardley_position_is_visibility_then_evolution() {
    // Mermaid's own documentation calls this "opposite of typical (x, y)
    // notation", so getting it backwards would be easy and silent: the map
    // would still draw, with every component in the wrong place.
    //
    // `[0.95, 0.10]` is nearly all visibility and almost no evolution, so it
    // belongs at the top left. Reversed it would sit at the bottom right.
    let d = render(
        "wardley-beta\ncomponent Corner [0.95, 0.10]\n",
        &Index,
        &Options::default(),
    )
    .expect("should render");
    let circle = d
        .svg
        .split("<circle")
        .nth(1)
        .expect("a component is a circle");
    let number = |attr: &str| -> f32 {
        circle
            .split_once(&format!("{attr}=\""))
            .and_then(|(_, rest)| rest.split_once('"'))
            .and_then(|(value, _)| value.parse().ok())
            .unwrap_or_else(|| panic!("no {attr} in {circle}"))
    };
    let (cx, cy) = (number("cx"), number("cy"));
    let width = d
        .svg
        .split_once("viewBox=\"0 0 ")
        .and_then(|(_, rest)| rest.split_once(' '))
        .and_then(|(w, _)| w.parse::<f32>().ok())
        .expect("a viewBox");
    assert!(
        cx < width / 2.0,
        "evolution 0.10 should be left of centre, got cx {cx} of {width}"
    );
    assert!(
        cy < 220.0,
        "visibility 0.95 should be near the top, got cy {cy}"
    );
}

#[test]
fn a_venn_union_sits_in_the_region_its_sets_share() {
    // A union of every set belongs in the middle; a union of only some belongs
    // out in the lens those share and the others do not. Placing both at the
    // plain centroid would stack them, and the diagram would say nothing.
    let d = render(
        "venn-beta\nset A\nset B\nset C\nunion A,B[\"pair\"]\nunion A,B,C[\"all\"]\n",
        &Index,
        &Options::default(),
    )
    .expect("should render");

    // merman draws a Venn with paths rather than circles, so the geometry
    // cannot be read off circle centres the way the vendored renderer's
    // hand-written one allowed. What still holds, and is the point, is that
    // the two unions are not drawn on top of each other.
    // The name has to be matched with its leading space: merman writes
    // `dy=".35em"` on the same element, and a bare `y="` finds that first.
    let attr = |hay: &str, name: &str| -> f32 {
        hay.split_once(&format!(" {name}=\""))
            .and_then(|(_, rest)| rest.split_once('"'))
            .and_then(|(value, _)| value.parse().ok())
            .unwrap_or_else(|| panic!("no {name} in {hay}"))
    };
    let label_at = |needle: &str| -> (f32, f32) {
        let before = d
            .svg
            .split_once(&format!(">{needle}<"))
            .unwrap_or_else(|| panic!("{needle} is not drawn"))
            .0;
        let tag = before.rfind("<text").expect("a text element");
        (attr(&before[tag..], "x"), attr(&before[tag..], "y"))
    };
    let all = label_at("all");
    let pair = label_at("pair");
    let far = |p: (f32, f32), q: (f32, f32)| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();

    assert!(
        far(all, pair) > 40.0,
        "the pairwise and all-three labels are in the same place"
    );
    // The A,B lens is the part of A and B that C does not reach, so the label
    // has to be nearer both of them than C. Each set's own label stands in
    // for its circle, there being no circle to read a centre from.
    let (a, b, c) = (label_at("A"), label_at("B"), label_at("C"));
    assert!(
        far(pair, a) < far(pair, c) && far(pair, b) < far(pair, c),
        "the A,B label is not in the A,B lens: {:.0} from A, {:.0} from B, {:.0} from C",
        far(pair, a),
        far(pair, b),
        far(pair, c)
    );
    // The all-three label belongs where every circle reaches: the middle.
    let spread = [a, b, c].map(|centre| far(all, centre));
    let (lo, hi) = (
        spread.iter().cloned().fold(f32::MAX, f32::min),
        spread.iter().cloned().fold(0.0, f32::max),
    );
    // Proportional, not the 1px the vendored renderer's exact circle centres
    // allowed: a set's label stands in for its circle here, and a label sits
    // somewhere on its circle rather than at the middle of it.
    assert!(
        (hi - lo) / lo < 0.05,
        "the all-three label should be about equidistant from the three sets, got {spread:?}"
    );
}
