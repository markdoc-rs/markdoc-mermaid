//! What a document's own text and a page's own structure need.
//!
//! Three promises: a heading reaches the picture as written, whatever is in
//! it; two diagrams in one HTML document share no ids; and `flowchart()`
//! reports the text that was drawn, not the HTML behind it.
use markdoc_mermaid::{Anchor, Anchors, Options, render};
struct H(&'static str);
impl Anchors for H {
    fn get(&self, id: &str) -> Option<Anchor> {
        (id == "step_one").then(|| Anchor {
            label: self.0.into(),
            href: "#step_one".into(),
        })
    }
}
fn text_of(svg: &str) -> String {
    let svg = match (svg.find("<style>"), svg.find("</style>")) {
        (Some(o), Some(c)) if c > o => format!("{}{}", &svg[..o], &svg[c + 8..]),
        _ => svg.to_string(),
    };
    let mut out = String::new();
    for frag in svg.split("<text").skip(1) {
        if let Some((_, rest)) = frag.split_once('>')
            && let Some(end) = rest.find("</text>")
        {
            let mut d = 0;
            for c in rest[..end].chars() {
                match c {
                    '<' => d += 1,
                    '>' => d -= 1,
                    c if d == 0 => out.push(c),
                    _ => {}
                }
            }
            out.push(' ');
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    out.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
const SRC: &str = "flowchart TD\n  step_one --> step_two\n";

#[test]
fn a_heading_reaches_the_picture_however_it_is_written() {
    for heading in [
        "Press <Enter> to confirm",
        "<img src=x onerror=alert(1)>",
        "C# and issue #42 and #notanentity;",
        "Fuse rated #amperes; and #lta; values",
        "Voltage < 24 V & > 12 V",
        "Check the \"E-stop\" relay",
    ] {
        let d = render(SRC, &H(heading), &Options::default()).expect("renders");
        let drawn = text_of(&d.svg);
        let want = heading.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(drawn.contains(&want), "lost: {heading:?} drew {drawn:?}");
    }
}

#[test]
fn two_diagrams_in_one_document_share_no_ids() {
    let a = render(
        "flowchart TD\n  a[Diagram A] --> b[B]\n",
        &H("x"),
        &Options::default(),
    )
    .expect("a");
    let b = render(
        "flowchart TD\n  c[Diagram B] --> d[D]\n",
        &H("x"),
        &Options::default(),
    )
    .expect("b");
    let ids = |svg: &str| -> std::collections::BTreeSet<String> {
        svg.split(" id=\"")
            .skip(1)
            .filter_map(|f| f.split('"').next().map(str::to_string))
            .collect()
    };
    let shared: Vec<_> = ids(&a.svg).intersection(&ids(&b.svg)).cloned().collect();
    println!(
        "FB1 shared ids: {} {:?}",
        shared.len(),
        shared.iter().take(4).collect::<Vec<_>>()
    );
    assert!(shared.is_empty(), "two diagrams share {} ids", shared.len());
}

#[test]
fn flowchart_reports_the_text_that_was_drawn() {
    for heading in [
        "Voltage < 24 V & > 12 V",
        "Press <Enter> to confirm",
        "</script><b>&</b>",
        "Check the \"E-stop\" relay",
    ] {
        let f = markdoc_mermaid::flowchart(SRC, &H(heading)).expect("a flowchart");
        let got = &f
            .nodes
            .iter()
            .find(|n| n.id == "step_one")
            .expect("step_one")
            .label;
        assert_eq!(got, heading, "flowchart() label is not the drawn text");
    }
    // An author's own label: `&amp;` decodes, which is the common case.
    // `#lt;tag#gt;` is not asserted to round-trip, because merman's model
    // and its drawing disagree over it.
    let src = "flowchart TD\n  x[\"A &amp; more\"] -- \"a #amp; b\" --> y\n";
    let f = markdoc_mermaid::flowchart(src, &H("unused")).expect("a flowchart");
    let d = render(src, &H("unused"), &Options::default()).expect("renders");
    let drawn = text_of(&d.svg);
    let x = &f.nodes.iter().find(|n| n.id == "x").expect("x").label;
    assert_eq!(x, "A & more", "an author's `&amp;` should decode");
    assert!(drawn.contains(x.as_str()), "{x:?} is not in {drawn:?}");
    let label = f
        .edges
        .first()
        .and_then(|e| e.label.clone())
        .expect("an edge label");
    assert_eq!(label, "a & b", "an author's `#amp;` should decode");
}

#[test]
fn a_fragment_href_reaches_the_picture_unescaped() {
    // `quote` escapes a heading's `#` as `#35;`, but an href is not label
    // text: escaping the `#` of `#step_one` would kill the link.
    struct Href;
    impl Anchors for Href {
        fn get(&self, id: &str) -> Option<Anchor> {
            (id == "step_one").then(|| Anchor {
                label: "A heading".into(),
                href: "#step_one".into(),
            })
        }
    }
    let d = render(SRC, &Href, &Options::default()).expect("renders");
    assert!(
        d.svg.contains("#step_one"),
        "the fragment href did not reach the SVG: {}",
        d.svg
    );
    assert!(
        !d.svg.contains("#35;step_one"),
        "the href was escaped as if it were label text"
    );
}

#[test]
fn a_label_is_drawn_where_it_is_used_not_referred_to() {
    // `resvg_safe` defines each label in `<defs>` and draws it with
    // `<use href="#…">`, which a page sanitising against an element
    // allowlist would drop, losing every label silently.
    let d = render(
        "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n",
        &H("unused"),
        &Options::default(),
    )
    .expect("renders");

    assert_eq!(
        d.svg.matches("<use").count(),
        0,
        "a label is still drawn through a reference: {}",
        d.svg
    );
    assert_eq!(
        d.svg.matches("-foreignobject-label-").count(),
        0,
        "a label definition is still there to be referred to"
    );
    // Across every kind that defines labels this way: eventmodeling draws
    // its through a `<use>` with a non-zero offset.
    for (kind, src) in [
        (
            "sequence",
            "sequenceDiagram\n  participant A\n  participant B\n  A->>B: hi\n",
        ),
        ("class", "classDiagram\n  class Animal\n"),
        ("state", "stateDiagram-v2\n  [*] --> Still\n"),
        ("block", "block-beta\n  columns 2\n  a[\"One\"] b\n"),
        (
            "eventmodeling",
            "eventmodeling\n\ntf 01 ui CartUI\ntf 02 cmd AddItem\n",
        ),
        ("treeView", "treeView-beta\n  Root\n    Child\n"),
        (
            "swimlane",
            "swimlane-beta LR\n  subgraph C\n    x[X]\n  end\n",
        ),
        ("mindmap", "mindmap\n  root((R))\n    a\n"),
    ] {
        let k = render(src, &H("unused"), &Options::default()).expect(kind);
        assert_eq!(
            k.svg.matches("<use").count(),
            0,
            "{kind}: a label is still drawn through a reference"
        );
    }

    // And the labels are drawn.
    let drawn = text_of(&d.svg);
    for want in ["One", "Two", "yes"] {
        assert!(drawn.contains(want), "{want:?} is not drawn, in {drawn:?}");
    }
}

#[test]
fn an_id_the_author_labelled_is_reported_as_the_author_wrote_it() {
    // `expand` leaves an id the author labelled alone, so the report has to
    // follow the picture rather than the anchors.
    struct Known;
    impl Anchors for Known {
        fn get(&self, id: &str) -> Option<Anchor> {
            matches!(id, "a" | "b" | "c").then(|| Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    let src = "flowchart TD\n  start{Is the light on?} -->|yes| a\n  start -- no --> c\n  \
               a --> b --> c\n  a[Check it]\n";
    let f = markdoc_mermaid::flowchart(src, &Known).expect("a flowchart");
    let label = |id: &str| {
        f.nodes
            .iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| panic!("{id}"))
            .label
            .clone()
    };
    assert_eq!(
        label("a"),
        "Check it",
        "the author's own label was overruled"
    );
    // The ones the author left bare still come from the anchors.
    assert_eq!(label("b"), "Heading b");
    assert_eq!(label("c"), "Heading c");
    // And the picture agrees with the report.
    let drawn = text_of(
        &render(src, &Known, &Options::default())
            .expect("renders")
            .svg,
    );
    assert!(drawn.contains("Check it"), "{drawn:?}");
    assert!(!drawn.contains("Heading a"), "{drawn:?}");
}

#[test]
fn a_label_that_is_not_ascii_is_reported_rather_than_panicking() {
    // Byte 12 can fall inside a character, where a slice would panic.
    for (src, id, want) in [
        ("flowchart TD\n  a --> b[\"&ÆØÅÆØÅ\"]\n", "b", "&ÆØÅÆØÅ"),
        (
            "flowchart TD\n  a --> b[\"ÆØÅÆØÅÆØÅ&\"]\n",
            "b",
            "ÆØÅÆØÅÆØÅ&",
        ),
        (
            "flowchart TD\n  a --> b[\"日本語のラベル\"]\n",
            "b",
            "日本語のラベル",
        ),
    ] {
        let f = markdoc_mermaid::flowchart(src, &H("unused")).expect("a flowchart");
        let got = &f.nodes.iter().find(|n| n.id == id).expect(id).label;
        assert_eq!(got, want, "a label came back wrong");
    }
    // An edge label too, which takes the same path.
    let f = markdoc_mermaid::flowchart(
        "flowchart TD\n  a -- \"電圧 < 電圧電圧\" --> b\n",
        &H("unused"),
    )
    .expect("a flowchart");
    assert_eq!(
        f.edges.first().and_then(|e| e.label.clone()),
        Some("電圧 < 電圧電圧".to_string())
    );
}

#[test]
fn one_source_drawn_two_ways_gets_two_ids() {
    // One source rendered two ways is two pictures, so the options go into
    // the derived id as well.
    let id_of = |options: &Options| {
        let svg = render(SRC, &H("A heading"), options).expect("renders").svg;
        let at = svg.find("id=\"").expect("a root id");
        let rest = &svg[at + 4..];
        rest[..rest.find('"').expect("a quote")].to_string()
    };
    let mut wide = markdoc_mermaid::Layout::default();
    wide.label_slack = 1.3;
    let ids = [
        id_of(&Options::default()),
        id_of(&Options {
            themeable: false,
            ..Options::default()
        }),
        id_of(&Options {
            link: false,
            ..Options::default()
        }),
        id_of(&Options {
            class: "other".into(),
            ..Options::default()
        }),
        id_of(&Options {
            layout: wide,
            ..Options::default()
        }),
        id_of(&Options {
            measure: Some(markdoc_mermaid::Measure::new(
                |t: markdoc_mermaid::Text<'_>| t.text.chars().count() as f64 * t.size,
            )),
            ..Options::default()
        }),
    ];
    let unique: std::collections::BTreeSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "two of these share an id: {ids:?}");
    // And the same options twice give the same id.
    assert_eq!(id_of(&Options::default()), ids[0]);
}

#[test]
fn every_class_in_a_chain_of_inheritance_is_labelled() {
    // The `|` of `<|--` is not the start of an edge label, and no label
    // spans a line.
    struct All;
    impl Anchors for All {
        fn get(&self, id: &str) -> Option<Anchor> {
            id.starts_with('q').then(|| Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    for (arrow, src) in [
        (
            "<|--",
            "classDiagram\n  q1 <|-- q2\n  q2 <|-- q3\n  q3 <|-- q4\n",
        ),
        (
            "..|>",
            "classDiagram\n  q1 ..|> q2\n  q2 ..|> q3\n  q3 ..|> q4\n",
        ),
        (
            "--|>",
            "classDiagram\n  q1 --|> q2\n  q2 --|> q3\n  q3 --|> q4\n",
        ),
        (
            "-->",
            "classDiagram\n  q1 --> q2\n  q2 --> q3\n  q3 --> q4\n",
        ),
    ] {
        let diagram = render(src, &All, &Options::default()).expect("renders");
        assert_eq!(
            diagram.resolved,
            vec!["q1", "q2", "q3", "q4"],
            "{arrow} left a class unlabelled"
        );
    }
}

#[test]
fn an_edge_labels_own_text_is_still_not_read_as_an_id() {
    // A real `|…|` label is skipped, so a word inside it that matches an
    // anchor is not resolved.
    struct All;
    impl Anchors for All {
        fn get(&self, id: &str) -> Option<Anchor> {
            id.starts_with('q').then(|| Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    // Both forms Mermaid has for an edge label, and a chain that must not be
    // mistaken for one.
    for (form, src) in [
        ("|…|", "flowchart TD\n  q1 -->|q9| q2\n"),
        ("-- … -->", "flowchart TD\n  q1 -- q9 --> q2\n"),
        ("== … ==>", "flowchart TD\n  q1 == q9 ==> q2\n"),
        ("-. … .->", "flowchart TD\n  q1 -. q9 .-> q2\n"),
        ("-- … ---", "flowchart TD\n  q1 -- q9 --- q2\n"),
    ] {
        let diagram = render(src, &All, &Options::default()).expect("renders");
        assert_eq!(
            diagram.resolved,
            vec!["q1", "q2"],
            "{form}: the label's own text was taken for a node"
        );
        // And no third box was invented for it.
        let graph = markdoc_mermaid::flowchart(src, &All).expect("a flowchart");
        assert_eq!(
            graph.nodes.len(),
            2,
            "{form}: the diagram grew a node the author never wrote: {:?}",
            graph.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
        );
    }

    // A chain is not a labelled edge: `-->` opens nothing, so `b` is a node.
    let chain = render(
        "flowchart TD\n  q1 --> q2 --> q3\n",
        &All,
        &Options::default(),
    )
    .expect("renders");
    assert_eq!(chain.resolved, vec!["q1", "q2", "q3"]);

    // Nor is an arrowhead the start of a word.
    let heads = render(
        "flowchart TD\n  q1 --o q2\n  q2 --x q3\n",
        &All,
        &Options::default(),
    )
    .expect("renders");
    assert_eq!(heads.resolved, vec!["q1", "q2", "q3"]);
}

#[test]
fn only_what_merman_parses_as_a_node_is_declared() {
    // Every word has an anchor here, so anything `scan` takes for an id is
    // tried -- and only the nodes in merman's own parse may be declared.
    struct Every;
    impl Anchors for Every {
        fn get(&self, id: &str) -> Option<Anchor> {
            Some(Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    for (what, src, nodes) in [
        (
            "a multiplicity",
            "classDiagram\n  customer \"1\" -- \"0..*\" order\n  order \"1\" *-- \"1..*\" line\n",
            &["customer", "order", "line"][..],
        ),
        (
            "a relation's label",
            "classDiagram\n  order *-- item : holds 1..n\n",
            &["order", "item"][..],
        ),
        (
            "a message",
            "sequenceDiagram\n  alice--)bob: see the overview...\n",
            &["alice", "bob"][..],
        ),
        (
            "an edge's label",
            "flowchart LR\n  a -- in->out --> b\n",
            &["a", "b"][..],
        ),
        (
            "a keyword",
            "flowchart LR\n  a --> b\n  style a fill:#eee\n",
            &["a", "b"][..],
        ),
        (
            "a subgraph",
            "flowchart LR\n  subgraph group\n    c --> d\n  end\n",
            &["c", "d"][..],
        ),
        (
            "a block's space",
            "block-beta\n  columns 3\n  a space b\n",
            &["a", "b"][..],
        ),
    ] {
        let diagram = render(src, &Every, &Options::default())
            .unwrap_or_else(|e| panic!("{what}: the diagram failed: {e}"));
        assert_eq!(diagram.resolved, nodes, "{what}");
    }
}

#[test]
fn an_arrowhead_against_the_next_id_does_not_cost_it_its_heading() {
    // Mermaid's lexer reads `--x` and `--o` as the whole arrow whatever
    // follows, `carol--xdave` included.
    struct All;
    impl Anchors for All {
        fn get(&self, id: &str) -> Option<Anchor> {
            id.starts_with('q').then(|| Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    for (what, src, want) in [
        (
            "a cross arrow, spaced",
            "flowchart TD\n  q1 --x q2\n",
            vec!["q1", "q2"],
        ),
        (
            "a cross arrow, tight",
            "flowchart TD\n  q1 --xq2\n",
            vec!["q1", "q2"],
        ),
        (
            "a circle arrow, tight",
            "flowchart TD\n  q1 --oq2\n",
            vec!["q1", "q2"],
        ),
        (
            "a sequence cross arrow",
            "sequenceDiagram\n  q1--xq2: done\n",
            vec!["q1", "q2"],
        ),
        // An arrow that already has its head keeps the next word whole.
        (
            "an id after a complete arrow",
            "flowchart TD\n  q1 --> q2\n",
            vec!["q1", "q2"],
        ),
    ] {
        let diagram =
            render(src, &All, &Options::default()).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(diagram.resolved, want, "{what}");
    }

    // The head is only taken from an arrow that wants one: `-->` does not,
    // so a node whose id starts with `o` or `x` survives.
    struct Ox;
    impl Anchors for Ox {
        fn get(&self, id: &str) -> Option<Anchor> {
            matches!(id, "one" | "xray" | "a").then(|| Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    for src in ["flowchart TD\n  a-->one\n", "flowchart TD\n  a-->xray\n"] {
        let diagram = render(src, &Ox, &Options::default()).expect("renders");
        assert_eq!(
            diagram.resolved.len(),
            2,
            "a word after a complete arrow lost its head: {:?}",
            diagram.resolved
        );
    }
}

#[test]
fn a_node_the_scanner_misreads_still_gets_its_heading() {
    // The candidates are merman's nodes, so how `scan` reads an arrow does
    // not decide which ids are labelled -- and the same arrow is lexed
    // differently from one kind to the next.
    struct Every;
    impl Anchors for Every {
        fn get(&self, id: &str) -> Option<Anchor> {
            Some(Anchor {
                label: format!("Heading {id}"),
                href: format!("#{id}"),
            })
        }
    }
    for (what, src, nodes) in [
        (
            "a sequence diagram's -x",
            "sequenceDiagram\n  alice-xbob: hi\n",
            &["alice", "bob"][..],
        ),
        (
            "a class relation written against its target",
            "classDiagram\n  a --ob\n",
            &["a", "ob"][..],
        ),
        // A composite state's `{` opens its body, not a label.
        (
            "a composite state and the states inside it",
            "stateDiagram-v2\n  state active {\n    a --> b\n  }\n",
            &["active", "a", "b"][..],
        ),
        // So does a class's member body, the usual way to list members.
        (
            "a class with a member body",
            "classDiagram\n  class Animal {\n    +int age\n  }\n  Animal <|-- Duck\n",
            &["Animal", "Duck"][..],
        ),
        (
            "a class with a member body, the brace against the name",
            "classDiagram\n  class Animal{\n    +int age\n  }\n  Animal <|-- Duck\n",
            &["Animal", "Duck"][..],
        ),
        // But a label written before the body is still theirs, and a
        // flowchart's `{…}` is still a label.
        (
            "a class with a label and a body",
            "classDiagram\n  class Animal[\"An animal\"] {\n    +int age\n  }\n  Animal <|-- Duck\n",
            &["Duck"][..],
        ),
        (
            "a flowchart's decision",
            "flowchart TD\n  a{a} --> b\n",
            &["b"][..],
        ),
    ] {
        let diagram = render(src, &Every, &Options::default())
            .unwrap_or_else(|e| panic!("{what}: the diagram failed: {e}"));
        assert_eq!(diagram.resolved, nodes, "{what}");
    }
}
