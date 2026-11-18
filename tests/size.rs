//! What the diagram says about its own size and text, so a page can reserve
//! room before the SVG loads and size its own text to match.
use markdoc_mermaid::{Anchor, Anchors, Options, render};

struct No;
impl Anchors for No {
    fn get(&self, _: &str) -> Option<Anchor> {
        None
    }
}

/// The `viewBox`, as four numbers.
fn view_box(svg: &str) -> Vec<f32> {
    let at = svg.find("viewBox=\"").expect("a viewBox");
    let rest = &svg[at + 9..];
    rest[..rest.find('"').expect("a quote")]
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().expect("a number"))
        .collect()
}

const KINDS: &[(&str, &str)] = &[
    (
        "flowchart",
        "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n",
    ),
    // Its viewBox does not start at the origin.
    ("sequence", "sequenceDiagram\n  A->>B: Ping\n"),
    ("class", "classDiagram\n  class Animal\n  Animal <|-- Dog\n"),
    ("state", "stateDiagram-v2\n  [*] --> Idle\n"),
    (
        "block",
        "block-beta\n  columns 2\n  a[\"One\"] b[\"Two\"]\n",
    ),
    ("mindmap", "mindmap\n  root((R))\n    one\n"),
    (
        "gantt",
        "gantt\n  title A\n  section S\n  T :a1, 2026-01-01, 3d\n",
    ),
    ("kanban", "kanban\n  Todo\n    task\n"),
    ("pie", "pie title S\n  \"A\" : 60\n"),
    ("treeView", "treeView-beta\n  Root\n    Child\n"),
];

#[test]
fn a_diagram_reports_the_extent_of_its_own_view_box() {
    for (kind, src) in KINDS {
        for themeable in [true, false] {
            let diagram = render(
                src,
                &No,
                &Options {
                    themeable,
                    ..Options::default()
                },
            )
            .unwrap_or_else(|e| panic!("{kind}: {e}"));
            let box_of = view_box(&diagram.svg);
            let size = diagram
                .size
                .unwrap_or_else(|| panic!("{kind} themeable={themeable}: no size"));
            assert_eq!(
                (size.width, size.height),
                (box_of[2], box_of[3]),
                "{kind} themeable={themeable}: not the viewBox's extent"
            );
            // The origin is not in it.
            assert!(
                size.width > 0.0 && size.height > 0.0,
                "{kind}: an empty size"
            );
        }
    }
}

#[test]
fn the_size_is_the_same_whether_or_not_the_page_owns_the_colours() {
    // A page and a PDF get the same shape; only the paints differ.
    for (kind, src) in KINDS {
        let themed = render(src, &No, &Options::default()).expect(kind);
        let kept = render(
            src,
            &No,
            &Options {
                themeable: false,
                ..Options::default()
            },
        )
        .expect(kind);
        assert_eq!(themed.size, kept.size, "{kind} changed shape with theming");
    }
}

#[test]
fn a_diagram_reports_the_text_size_it_sets_for_itself() {
    let diagram = render(KINDS[0].1, &No, &Options::default()).expect("renders");
    assert_eq!(diagram.font_size, Some(16.0));
    // The document's default, not every label's size.
    assert!(
        diagram.svg.contains("font-size:12px"),
        "the fixture no longer has a differently-sized label, so the \
         distinction this documents is untested"
    );

    // Every kind that sets one agrees with its own stylesheet.
    for (kind, src) in KINDS {
        let diagram = render(src, &No, &Options::default()).expect(kind);
        let Some(size) = diagram.font_size else {
            continue;
        };
        assert!(size > 0.0, "{kind}: a size of {size}");
        assert!(
            diagram.svg.contains(&format!("font-size:{size}px")),
            "{kind}: {size}px is in no rule of the diagram's own"
        );
    }
}

#[test]
fn a_kind_that_sets_no_default_says_so_rather_than_guessing() {
    // eventmodeling writes no `font-size` into the document's own rule.
    let diagram = render(
        "eventmodeling\n\ntf 01 ui CartUI\ntf 02 cmd AddItem\n",
        &No,
        &Options::default(),
    )
    .expect("renders");
    assert_eq!(diagram.font_size, None);
    // Its size is still known.
    assert!(diagram.size.is_some());
}

#[test]
fn an_id_merman_rewrites_does_not_hide_the_text_size() {
    // merman makes an id safe for a selector, so `"fig 1"` is written
    // `fig-1`.
    let diagram = render(
        KINDS[0].1,
        &No,
        &Options {
            id: Some("fig 1".into()),
            ..Options::default()
        },
    )
    .expect("renders");
    assert!(
        diagram.svg.contains("id=\"fig-1\""),
        "merman no longer rewrites the id, so this case is moot"
    );
    assert_eq!(diagram.font_size, Some(16.0));
}

#[test]
fn the_floor_is_never_above_the_smallest_word_drawn() {
    // A caller scaling a diagram down and keeping its labels legible needs
    // the smallest size rather than the default, and the floor has to be at
    // or below the smallest size each kind really draws.
    for (kind, src, smallest_drawn) in [
        (
            "gantt",
            "gantt\n  title A\n  section S\n  T :a1, 2026-01-01, 3d\n",
            10.0,
        ),
        (
            "packet",
            "packet-beta\n  0-7: \"Head\"\n  8-15: \"Tail\"\n",
            10.0,
        ),
        ("treemap", "treemap-beta\n\"Root\"\n  \"one\": 10\n", 10.0),
        ("gitGraph", "gitGraph\n  commit\n", 10.0),
        ("cynefin", "cynefin-beta\ncomplex\ncomplicated\n", 11.0),
        (
            "quadrant",
            "quadrantChart\n  title A\n  x-axis L --> H\n  A: [0.3, 0.6]\n",
            12.0,
        ),
        // The case for resolving relative sizes: C4 writes `0.75em`.
        ("c4", "C4Context\n  title A\n  Person(a, \"A\")\n", 10.5),
        ("er", "erDiagram\n  A ||--o{ B : has\n", 14.0),
        ("sankey", "sankey-beta\n\nA,B,10\n", 14.0),
        (
            "journey",
            "journey\n  title A\n  section S\n    T: 5: Me\n",
            14.0,
        ),
        ("treeView", "treeView-beta\n  Root\n    Child\n", 16.0),
        (
            "eventmodeling",
            "eventmodeling\n\ntf 01 ui UI\ntf 02 cmd Cmd\n",
            16.0,
        ),
        (
            "wardley",
            "wardley-beta\n  title A\n  component x [0.5, 0.5]\n",
            10.0,
        ),
        (
            "ishikawa",
            "ishikawa-beta\n  title A\n  cause \"c\"\n",
            14.0,
        ),
        ("zenuml", "zenuml\n  A->B: Hello\n", 12.0),
    ] {
        let diagram =
            render(src, &No, &Options::default()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        let floor = diagram
            .min_font_size
            .unwrap_or_else(|| panic!("{kind}: no size named anywhere"));
        assert!(
            floor <= smallest_drawn,
            "{kind}: the floor says {floor} where text is drawn at {smallest_drawn}"
        );
        // And not uselessly low, or a limit held against it would scale a
        // diagram far larger than it needs to be.
        assert!(
            floor >= smallest_drawn - 2.0,
            "{kind}: the floor says {floor} where the smallest drawn is \
             {smallest_drawn}, which is too conservative to be useful"
        );
    }
}

#[test]
fn the_six_kinds_that_set_no_default_still_have_a_floor() {
    // These have no rule of their own and size every element separately, so
    // `font_size` cannot answer and `min_font_size` can.
    for (kind, src) in [
        (
            "eventmodeling",
            "eventmodeling\n\ntf 01 ui UI\ntf 02 cmd Cmd\n",
        ),
        ("treeView", "treeView-beta\n  Root\n    Child\n"),
        ("ishikawa", "ishikawa-beta\n  title A\n  cause \"c\"\n"),
        ("zenuml", "zenuml\n  A->B: Hello\n"),
        (
            "wardley",
            "wardley-beta\n  title A\n  component x [0.5, 0.5]\n",
        ),
    ] {
        let diagram =
            render(src, &No, &Options::default()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert_eq!(diagram.font_size, None, "{kind} now sets a default");
        assert!(
            diagram.min_font_size.is_some(),
            "{kind}: no size to be had at all"
        );
    }
}

#[test]
fn a_rule_for_mermaids_run_time_tooltip_does_not_lower_the_floor() {
    // merman's flowchart, swimlane and journey stylesheets style the HTML
    // `div` Mermaid.js appends to the page for a tooltip, which nothing in
    // the SVG can match.
    for (kind, src, smallest_drawn) in [
        (
            "flowchart",
            "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n",
            16.0,
        ),
        (
            "swimlane",
            "---\nconfig:\n  layout: swimlane\n---\nflowchart TD\n  a[\"One\"] --> b[\"Two\"]\n",
            16.0,
        ),
        (
            "journey",
            "journey\n  title A\n  section S\n    T: 5: Me\n",
            14.0,
        ),
    ] {
        let diagram =
            render(src, &No, &Options::default()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert!(
            diagram.svg.contains("div.mermaidTooltip"),
            "{kind} no longer carries the rule, so this case is moot"
        );
        assert_eq!(
            diagram.min_font_size,
            Some(smallest_drawn),
            "{kind}: the floor is not the size its text is drawn at"
        );
    }
}
