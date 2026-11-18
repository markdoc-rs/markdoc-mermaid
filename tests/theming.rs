//! A themeable diagram carries no dark paint of its own on text.
//!
//! The kinds below are the ones whose text and whose *surface* can both be
//! tokenised. That pairing is the whole rule: a token is the page's colour,
//! so tokenising text that sits on a surface Mermaid still paints itself
//! makes the contrast worse, not better.
use markdoc_mermaid::{Anchor, Anchors, Options, render};

struct No;
impl Anchors for No {
    fn get(&self, _: &str) -> Option<Anchor> {
        None
    }
}

/// The darks merman writes: a flowchart's label, a class or use case label,
/// and plain `black`.
const DARK: [&str; 6] = ["#333", "#131300", "black", "#000;", "#000\"", "#000000"];

/// Every paint in `svg` that colours text and is a hard-coded dark.
fn dark_text_paints(svg: &str) -> Vec<String> {
    let mut found = Vec::new();
    for rule in svg.split('}') {
        let Some((selector, body)) = rule.rsplit_once('{') else {
            continue;
        };
        // A note is Mermaid's own pair, held by
        // `a_note_keeps_mermaids_own_pair` instead.
        if selector.contains("note") || selector.contains("Note") {
            continue;
        }
        let names_text = ["text", "Text", "tspan", "label", "Label", "title"]
            .iter()
            .any(|name| selector.contains(name));
        if names_text && body.contains("fill") && DARK.iter().any(|d| body.contains(d)) {
            let selector = selector.trim();
            found.push(format!(
                "{} {{{}}}",
                selector.split_whitespace().last().unwrap_or(selector),
                body.trim()
            ));
        }
    }
    for tag in ["<text", "<tspan"] {
        for fragment in svg.split(tag).skip(1) {
            let open: String = fragment.chars().take_while(|c| *c != '>').collect();
            if open.contains("fill") && DARK.iter().any(|d| open.contains(d)) {
                found.push(format!("{tag} with {open}"));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

#[test]
fn no_kind_paints_its_own_text_dark_when_the_page_owns_the_colours() {
    for (kind, src) in [
        ("flowchart", "flowchart TD\n  a[\"One\"] --> b[\"Two\"]\n"),
        (
            "sequence",
            "sequenceDiagram\n  participant A as Operator\n  A->>B: Ping\n  Note over A: a note\n",
        ),
        (
            "class",
            "classDiagram\n  class Animal {\n    +run()\n  }\n  Animal <|-- Dog\n",
        ),
        (
            "state",
            "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Busy: go\n",
        ),
        (
            "block",
            "block-beta\n  columns 2\n  a[\"One\"] b[\"Two\"]\n",
        ),
        ("pie", "pie title Share\n  \"A\" : 60\n  \"B\" : 40\n"),
        ("packet", "packet-beta\n  0-7: \"Head\"\n  8-15: \"Tail\"\n"),
        ("treeView", "treeView-beta\n  Root\n    Child\n"),
        ("er", "erDiagram\n  OPERATOR ||--o{ RELEASE : requests\n"),
        (
            "journey",
            "journey\n  title A\n  section S\n    Task: 5: Me\n",
        ),
        (
            "usecase",
            "usecase-beta\ndirection LR\nactor Customer(\"Customer\")\nsystemBoundary \"Order system\"\n  Checkout(\"Place order\")\nend\nCustomer --> Checkout\n",
        ),
    ] {
        let themed =
            render(src, &No, &Options::default()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        let found = dark_text_paints(&themed.svg);
        assert!(
            found.is_empty(),
            "{kind} paints its own text dark: {found:#?}"
        );
    }
}

#[test]
fn a_diagram_the_page_does_not_own_is_left_exactly_as_drawn() {
    // With `themeable` off nothing is retokenised.
    let plain = render(
        "flowchart TD\n  a[\"One\"] --> b[\"Two\"]\n",
        &No,
        &Options {
            themeable: false,
            ..Options::default()
        },
    )
    .expect("renders");
    assert!(
        !plain.svg.contains("var(--diagram-"),
        "an unthemed diagram was tokenised anyway"
    );
}

/// A label drawn on a surface Mermaid colours itself keeps Mermaid's paint.
///
/// A rule saying `fill:black` says nothing about what the label is drawn on.
/// merman says which is which on the element itself, in
/// `data-merman-source-classes`, and that is what the inlining pass keys on.
#[test]
fn a_label_on_mermaids_own_surface_keeps_mermaids_own_paint() {
    for (kind, src) in [
        (
            "state",
            "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Busy: go\n  note right of Idle: a note\n",
        ),
        // Not a sequence diagram: it draws plain `<text>` with no source
        // classes to key on.
        (
            "class",
            "classDiagram\n  class A\n  note for A \"a note\"\n",
        ),
        ("mindmap", "mindmap\n  root((Root))\n    one\n      two\n"),
        ("flowchart", "flowchart TD\n  a[\"One\"] --> b[\"Two\"]\n"),
    ] {
        let themed =
            render(src, &No, &Options::default()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        let mut checked = 0;
        for fragment in themed.svg.split("<text").skip(1) {
            let open: String = fragment.chars().take_while(|c| *c != '>').collect();
            let Some(at) = open.find("data-merman-source-classes=\"") else {
                continue;
            };
            let classes = open[at + 28..].split('"').next().unwrap_or("");
            let Some(fill) = open
                .split("fill=\"")
                .nth(1)
                .and_then(|f| f.split('"').next())
            else {
                continue;
            };
            checked += 1;
            let mermaids = classes.contains("noteLabel") || classes.contains("mindmapDiagram");
            let tokenised = fill.starts_with("var(--");
            assert_eq!(
                tokenised, !mermaids,
                "{kind}: a label with classes {classes:?} is painted {fill:?}"
            );
        }
        assert!(checked > 0, "{kind}: no label carried a paint to check");
    }
}

/// A note keeps its pale yellow *and* the dark text that reads on it.
#[test]
fn a_note_keeps_mermaids_own_pair() {
    for (kind, src, surface) in [
        (
            "sequence",
            "sequenceDiagram\n  A->>B: hi\n  Note over A: a note\n",
            ".note",
        ),
        (
            "state",
            "stateDiagram-v2\n  [*] --> Idle\n  note right of Idle: a note\n",
            "statediagram-note rect",
        ),
        (
            "class",
            "classDiagram\n  class A\n  note for A \"a note\"\n",
            "noteGroup",
        ),
    ] {
        let themed =
            render(src, &No, &Options::default()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        let rule_for = |needle: &str| -> Option<String> {
            themed.svg.split('}').find_map(|rule| {
                let (selector, body) = rule.rsplit_once('{')?;
                selector.contains(needle).then(|| body.to_string())
            })
        };
        if let Some(body) = rule_for(surface) {
            assert!(
                !body.contains("var(--"),
                "{kind}: the note's own surface was tokenised, so its text should be too: {body}"
            );
        }
        // And whatever paints the note's text is not the page's colour.
        for rule in themed.svg.split('}') {
            let Some((selector, body)) = rule.rsplit_once('{') else {
                continue;
            };
            if (selector.contains("noteText") || selector.contains("note text"))
                && body.contains("fill")
            {
                assert!(
                    !body.contains("var(--"),
                    "{kind}: a note's text took the page's colour over Mermaid's yellow: {body}"
                );
            }
        }
    }
}

/// A state's box and its label are coloured by the same hand: `mainBkg` does
/// not reach `stateBkg` in merman's `default` theme.
#[test]
fn a_state_box_and_its_label_are_both_the_pages() {
    let themed = render(
        "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Busy: go\n",
        &No,
        &Options::default(),
    )
    .expect("renders");
    for needle in ["g.stateGroup rect", ".stateLabel text", ".node rect"] {
        let body = themed
            .svg
            .split('}')
            .find_map(|rule| {
                let (selector, body) = rule.rsplit_once('{')?;
                selector.contains(needle).then_some(body)
            })
            .unwrap_or_else(|| panic!("no rule for {needle}"));
        assert!(
            body.contains("var(--"),
            "{needle} is not the page's colour: {body}"
        );
    }
}

/// The kinds still on Mermaid's own palette, so the list reads as deliberate
/// rather than forgotten: each paints text on a surface of its own, so
/// tokenising the text alone would put the page's colour on Mermaid's.
#[test]
fn the_kinds_on_mermaids_own_palette_are_known_and_listed() {
    for (kind, src) in [
        ("mindmap", "mindmap\n  root((Root))\n    one\n"),
        ("timeline", "timeline\n  title A\n  2026 : one\n"),
        (
            "gantt",
            "gantt\n  title A\n  section S\n  Task :a1, 2026-01-01, 3d\n",
        ),
        ("treemap", "treemap-beta\n\"Root\"\n  \"one\": 10\n"),
    ] {
        let themed =
            render(src, &No, &Options::default()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert!(
            !dark_text_paints(&themed.svg).is_empty(),
            "{kind} is now tokenised: move it into the test above"
        );
    }
}

#[test]
fn a_kept_palette_is_the_one_asked_for() {
    use markdoc_mermaid::{Layout, Palette};
    const SRC: &str = "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n";
    let drawn = |palette: Palette, themeable: bool| {
        let mut layout = Layout::default();
        layout.palette = palette;
        render(
            SRC,
            &No,
            &Options {
                themeable,
                layout,
                ..Options::default()
            },
        )
        .expect("renders")
        .svg
    };

    // Kept colours: Mermaid's own lavender, or the greys a printed page
    // wants. These are the literals each scheme emits.
    let lavender = drawn(Palette::Default, false);
    assert!(
        lavender.contains("#ECECFF"),
        "the default lost its lavender"
    );
    let grey = drawn(Palette::Neutral, false);
    assert!(
        grey.contains("#eee") && grey.contains("#999"),
        "neutral did not reach the drawing"
    );
    assert!(
        !grey.contains("#ECECFF"),
        "neutral kept the default's lavender"
    );

    // Owned colours: the same drawing either way, because the page decides
    // them. The ids differ and are normalised out, since the derived id
    // hashes the whole of `Layout`.
    let without_id = |svg: &str| -> String {
        let at = svg.find("id=\"").expect("a root id");
        let rest = &svg[at + 4..];
        let id = &rest[..rest.find('"').expect("a quote")];
        svg.replace(id, "<id>")
    };
    let themed_default = drawn(Palette::Default, true);
    let themed_neutral = drawn(Palette::Neutral, true);
    assert_eq!(
        without_id(&themed_default),
        without_id(&themed_neutral),
        "a palette changed output the page was supposed to own"
    );
    assert!(dark_text_paints(&themed_neutral).is_empty());
}

#[test]
fn the_neo_look_draws_in_a_themed_page() {
    use markdoc_mermaid::Layout;
    // Naming no scheme at all is the path in merman that reads
    // `var(--diagram-line)` as a colour and refuses the diagram.
    for (kind, src) in [
        (
            "flowchart",
            "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n",
        ),
        ("sequence", "sequenceDiagram\n  A->>B: hi\n"),
        ("state", "stateDiagram-v2\n  [*] --> Idle\n"),
        ("class", "classDiagram\n  A <|-- B\n"),
    ] {
        let mut layout = Layout::default();
        layout.classic_look = false;
        let themed = render(
            src,
            &No,
            &Options {
                layout,
                ..Options::default()
            },
        )
        .unwrap_or_else(|e| panic!("{kind} in the neo look: {e}"));
        assert!(
            themed.svg.contains("var(--diagram-"),
            "{kind}: the neo look drew without the page's colours"
        );
    }
}

#[test]
fn a_kept_palette_reaches_the_neo_look_too() {
    use markdoc_mermaid::{Layout, Palette};
    let mut layout = Layout::default();
    layout.classic_look = false;
    layout.palette = Palette::Neutral;
    let svg = render(
        "flowchart TD\n  a[\"One\"] --> b[\"Two\"]\n",
        &No,
        &Options {
            themeable: false,
            layout,
            ..Options::default()
        },
    )
    .expect("renders")
    .svg;
    assert!(
        svg.contains("#eee"),
        "the palette was dropped with the classic look"
    );
}

#[test]
fn naming_no_scheme_is_a_scheme_of_its_own() {
    use markdoc_mermaid::{Layout, Palette};
    // Naming no scheme is answered with one of merman's own: white boxes
    // and `#28253D` borders, under either look.
    const SRC: &str = "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n";
    for classic in [true, false] {
        let mut layout = Layout::default();
        layout.classic_look = classic;
        layout.palette = Palette::Unnamed;
        let svg = render(
            SRC,
            &No,
            &Options {
                themeable: false,
                layout,
                ..Options::default()
            },
        )
        .expect("renders")
        .svg;
        assert!(
            svg.contains("#28253D"),
            "classic_look={classic}: the unnamed scheme is not what was drawn"
        );
        assert!(
            !svg.contains("#ECECFF"),
            "classic_look={classic}: the default scheme's lavender came through"
        );
    }

    // And where the page owns the colours the default is named instead, so
    // the diagram still draws.
    let mut layout = Layout::default();
    layout.palette = Palette::Unnamed;
    let themed = render(
        SRC,
        &No,
        &Options {
            layout,
            ..Options::default()
        },
    )
    .expect("a themed diagram still renders");
    assert!(themed.svg.contains("var(--diagram-"));
    assert!(dark_text_paints(&themed.svg).is_empty());
}

#[test]
fn the_slate_scheme_is_the_four_colours_it_names() {
    use markdoc_mermaid::{Layout, Palette};
    // `base` on its own is a scheme to be adjusted, so the four variables
    // are the scheme. Each is checked where it lands, since one merman does
    // not pass through would leave `base`'s own colour there.
    let mut layout = Layout::default();
    layout.palette = Palette::Slate;
    let svg = render(
        "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n",
        &No,
        &Options {
            themeable: false,
            layout: layout.clone(),
            ..Options::default()
        },
    )
    .expect("renders")
    .svg;
    for (what, rule) in [
        ("the box", ".node rect,"),
        ("the line", ".edgePaths .path{"),
        ("the label", ".label text,"),
    ] {
        let declarations = svg
            .split('}')
            .find(|rule_| rule_.contains(rule))
            .unwrap_or_else(|| panic!("no rule for {what}"));
        for colour in match what {
            "the box" => ["#F8FAFC", "#94A3B8"],
            "the line" => ["#64748B", "#64748B"],
            _ => ["#0F172A", "#0F172A"],
        } {
            assert!(
                declarations.contains(colour),
                "{what} is not {colour}: {declarations}"
            );
        }
    }
    // Not Mermaid's own lavender, and no theme variable left as a token.
    assert!(
        !svg.contains("#ECECFF") && !svg.contains("#9370DB"),
        "{svg}"
    );
    assert!(!svg.contains("var(--"));

    // Where the page owns the colours it is ignored, like every other
    // scheme.
    let themed = render(
        "flowchart TD\n  a[\"One\"] -- yes --> b[\"Two\"]\n",
        &No,
        &Options {
            layout,
            ..Options::default()
        },
    )
    .expect("renders")
    .svg;
    assert!(
        !themed.contains("#F8FAFC"),
        "the scheme reached a themed page"
    );
    assert!(dark_text_paints(&themed).is_empty());
}

#[test]
fn a_corner_radius_rounds_the_boxes_that_have_none_of_their_own() {
    use markdoc_mermaid::Layout;
    // Written on the box as `rx`/`ry` rather than as a rule, which is what
    // makes it reach a PDF.
    let drawn = |radius: Option<f32>| {
        let mut layout = Layout::default();
        layout.corner_radius = radius;
        render(
            "flowchart TD\n  a[\"Box\"] --> b(\"Stadium\")\n",
            &No,
            &Options {
                themeable: false,
                layout,
                ..Options::default()
            },
        )
        .expect("renders")
    };

    let square = drawn(None);
    let rounded = drawn(Some(3.0));
    // A block diagram says "square" as `rx="0"`, a flowchart by writing no
    // `rx` at all.
    let block = |radius: Option<f32>| {
        let mut layout = Layout::default();
        layout.corner_radius = radius;
        render(
            "block-beta\n  columns 2\n  a[\"Plain\"] b(\"Rounded\")\n",
            &No,
            &Options {
                themeable: false,
                layout,
                ..Options::default()
            },
        )
        .expect("renders")
        .svg
    };
    assert!(
        block(None).contains("rx=\"0\""),
        "merman no longer writes a zero radius, so this case is moot"
    );
    let block = block(Some(3.0));
    assert!(block.contains("rx=\"3\" ry=\"3\""), "{block}");
    assert!(!block.contains("rx=\"0\""), "{block}");
    assert!(
        block.contains("rx=\"5\" ry=\"5\""),
        "the author's own: {block}"
    );
    // The box asked for, and the stadium's own radius left alone.
    assert!(rounded.svg.contains("rx=\"3\" ry=\"3\""), "{}", rounded.svg);
    for svg in [&square.svg, &rounded.svg] {
        assert_eq!(
            svg.matches("rx=\"5\" ry=\"5\"").count(),
            1,
            "the stadium lost the radius merman gave it: {svg}"
        );
    }
    assert!(!square.svg.contains("rx=\"3\""));
    // And nothing else moved.
    assert_eq!(square.size, rounded.size);
    assert_eq!(
        square.svg.matches("<rect").count(),
        rounded.svg.matches("<rect").count()
    );
    assert_eq!(
        square.svg.matches("<text").count(),
        rounded.svg.matches("<text").count()
    );
}

/// A page or a sheet of paper that wants a heavier line than one pixel.
///
/// The weight is a rule, where a corner radius had to be an attribute:
/// `stroke-width` is a presentation property, so print draws the same weight
/// as a page.
#[test]
fn a_link_is_drawn_at_the_weight_asked_for() {
    use markdoc_mermaid::Layout;
    let drawn = |source: &str, width: Option<f32>| {
        let mut layout = Layout::default();
        layout.line_width = width;
        render(
            source,
            &No,
            &Options {
                themeable: false,
                layout,
                ..Options::default()
            },
        )
        .expect("renders")
    };
    const LINKS: &str = "flowchart TD\n  a --> b\n  b ==> c\n  c ~~~ d\n";

    let thin = drawn(LINKS, None);
    // merman's own weights, which are the ones being scaled.
    assert!(
        thin.svg
            .contains(".edge-thickness-normal{stroke-width:1px;}"),
        "{}",
        thin.svg
    );
    assert!(
        thin.svg
            .contains(".edge-thickness-thick{stroke-width:3.5px;}"),
        "{}",
        thin.svg
    );

    let heavy = drawn(LINKS, Some(2.0));
    assert!(
        heavy
            .svg
            .contains(".edge-thickness-normal{stroke-width:2px;}"),
        "{}",
        heavy.svg
    );
    // A thick link is a weight of Mermaid's own, not a multiple of the
    // normal one.
    assert!(
        heavy
            .svg
            .contains(".edge-thickness-thick{stroke-width:3.5px;}"),
        "{}",
        heavy.svg
    );
    // Unless the weight asked for is heavier still, or a `==>` would come
    // out lighter than the `-->` beside it.
    let heaviest = drawn(LINKS, Some(4.0));
    assert!(
        heaviest
            .svg
            .contains(".edge-thickness-normal{stroke-width:4px;}"),
        "{}",
        heaviest.svg
    );
    assert!(
        heaviest
            .svg
            .contains(".edge-thickness-thick{stroke-width:4px;}"),
        "{}",
        heaviest.svg
    );
    assert!(
        heavy
            .svg
            .contains(".edge-thickness-invisible{stroke-width:0;"),
        "an invisible link drawn at a weight: {}",
        heavy.svg
    );
    // The box's border is not a line between two things.
    assert_eq!(
        heavy.svg.matches("stroke-width:1px;").count(),
        thin.svg.matches("stroke-width:1px;").count() - 1,
        "a border moved with the links: {}",
        heavy.svg
    );
    // A head follows the line, and a thick link shares it as it does in
    // Mermaid.
    let heads = |svg: &str, size: &str| {
        svg.matches(&format!("markerWidth=\"{size}\" markerHeight=\"{size}\""))
            .count()
    };
    assert!(heads(&thin.svg, "8") > 0, "no head to keep: {}", thin.svg);
    assert_eq!(heads(&heavy.svg, "8"), heads(&thin.svg, "8"));
    assert_eq!(heads(&heaviest.svg, "16"), 1, "{}", heaviest.svg);
    // And the drawing is the same shape: a weight is paint, not layout.
    assert_eq!(thin.size, heavy.size);
    assert_eq!(
        thin.svg.matches("<path").count(),
        heavy.svg.matches("<path").count()
    );

    // And a themed page gets the weight too.
    let mut layout = Layout::default();
    layout.line_width = Some(2.0);
    let themed = render(
        LINKS,
        &No,
        &Options {
            layout,
            ..Options::default()
        },
    )
    .expect("renders")
    .svg;
    assert!(
        themed.contains(".edge-thickness-normal{stroke-width:2px;}"),
        "{themed}"
    );
    // Heads included.
    assert!(themed.contains("overflow=\"visible\""), "{themed}");

    // A kind that weighs its own lines keeps them: a state diagram's
    // transitions are named in a later rule, which wins.
    let transitions = |svg: &str| {
        svg.split(".transition{")
            .skip(1)
            .filter_map(|rule| rule.split('}').next())
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    const STATES: &str = "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Done\n";
    let own = drawn(STATES, None);
    assert!(
        transitions(&own.svg)
            .iter()
            .any(|rule| rule.contains("stroke-width:1;")),
        "{}",
        own.svg
    );
    assert_eq!(
        transitions(&drawn(STATES, Some(2.0)).svg),
        transitions(&own.svg)
    );
    // A mindmap's branches taper by depth, which a rule appended at the end
    // would have flattened.
    let taper = |svg: &str| {
        svg.split(".edge-depth-")
            .skip(1)
            .filter_map(|rule| rule.split('}').next())
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let plain = drawn("mindmap\n  root((R))\n    one\n      two\n", None);
    let asked = drawn("mindmap\n  root((R))\n    one\n      two\n", Some(2.0));
    assert!(!taper(&plain.svg).is_empty(), "no taper: {}", plain.svg);
    assert_eq!(taper(&asked.svg), taper(&plain.svg));
}

/// A head is drawn whole, at a size that follows its line: merman's marker
/// is clipped to its own viewport and reset with a dash a PDF renderer reads
/// its own way.
#[test]
fn a_head_is_drawn_whole_at_a_size_that_follows_the_line() {
    use markdoc_mermaid::Layout;
    let drawn = |source: &str, width: Option<f32>| {
        let mut layout = Layout::default();
        layout.line_width = width;
        render(
            source,
            &No,
            &Options {
                themeable: false,
                layout,
                ..Options::default()
            },
        )
        .expect("renders")
        .svg
    };
    /// Each `<marker>` element in a document, whole.
    fn markers(svg: &str) -> Vec<String> {
        svg.split("<marker")
            .skip(1)
            .map(|element| match element.split_once("</marker>") {
                Some((body, _)) => body.to_string(),
                None => element.to_string(),
            })
            .collect()
    }
    /// A marker with its id emptied: the id is derived from the options,
    /// which is what differs between the two renders compared here.
    fn unnamed(element: &str) -> String {
        let Some(at) = element.find("id=\"") else {
            return element.to_string();
        };
        let from = at + 4;
        let end = element[from..]
            .find('"')
            .map_or(element.len(), |e| from + e);
        format!("{}{}", &element[..from], &element[end..])
    }
    let marker = |svg: &str, id: &str| {
        markers(svg)
            .into_iter()
            .find(|element| element.contains(&format!("{id}\"")))
            .unwrap_or_else(|| panic!("no {id} among {:?}", markers(svg)))
    };
    // A head for each of the three kinds a link can end in.
    const LINKS: &str = "flowchart TD\n  a --> b\n  b --o c\n  c --x d\n";

    // merman's own, which is what is being redrawn.
    let plain = drawn(LINKS, None);
    assert!(!plain.contains("overflow="), "{plain}");
    for written in [
        "markerWidth=\"8\" markerHeight=\"8\"",
        "markerWidth=\"11\" markerHeight=\"11\"",
        "class=\"arrowMarkerPath\" style=\"stroke-width:1;stroke-dasharray:1, 0\"",
    ] {
        assert!(plain.contains(written), "{written} in {plain}");
    }

    // Larger than the size Mermaid wrote, drawn whole and without the dash.
    let printed = drawn(LINKS, Some(2.5));
    for (id, size) in [
        ("pointEnd", "10"),
        ("circleEnd", "13.75"),
        ("crossEnd", "13.75"),
    ] {
        let head = marker(&printed, id);
        let drawn_at = format!("markerWidth=\"{size}\" markerHeight=\"{size}\"");
        assert!(head.contains(&drawn_at), "{drawn_at} in {head}");
        assert!(head.contains("overflow=\"visible\""), "{head}");
        assert!(!head.contains("stroke-dasharray"), "{head}");
    }
    // Mermaid defines a head for either end of a link and a wider pair for
    // a link with a margin. The ones no link points at are left as they are.
    for id in ["pointStart", "circleStart", "crossStart", "pointEnd-margin"] {
        assert_eq!(
            unnamed(&marker(&printed, id)),
            unnamed(&marker(&plain, id)),
            "{id} was redrawn"
        );
    }
    // At the pinned weight the head keeps merman's size and is only drawn
    // whole.
    let paired = drawn(LINKS, Some(2.0));
    assert!(
        paired
            .contains("markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\" overflow=\"visible\""),
        "{paired}"
    );
    // Nothing else is drawn.
    assert_eq!(
        plain.matches("<path").count(),
        printed.matches("<path").count()
    );

    // A mindmap defines the pair and points no branch at either, so there is
    // nothing to redraw at any weight.
    const BRANCHES: &str = "mindmap\n  root((R))\n    one\n      two\n";
    let defined = |source, width| {
        markers(&drawn(source, width))
            .iter()
            .map(|element| unnamed(element))
            .collect::<Vec<_>>()
    };
    assert!(!defined(BRANCHES, None).is_empty(), "no head defined");
    assert_eq!(defined(BRANCHES, Some(2.5)), defined(BRANCHES, None));
    // A usecase diagram draws Mermaid's head on lines of a weight it never
    // declares for a normal link, so neither moves.
    const USECASE: &str = "usecase-beta\n  actor U\n  U --> L\n";
    let own = drawn(USECASE, Some(2.5));
    assert!(own.contains("arrowMarkerPath"), "no head drawn: {own}");
    assert!(!own.contains("overflow="), "{own}");
    assert_eq!(defined(USECASE, Some(2.5)), defined(USECASE, None));
    // A weight of 0 asks for no normal link at all, so the lines go to 0
    // and every marker is left as merman wrote it.
    let silent = drawn(LINKS, Some(0.0));
    assert!(
        silent.contains(".edge-thickness-normal{stroke-width:0px;}"),
        "{silent}"
    );
    assert!(!silent.contains("overflow="), "{silent}");
    assert_eq!(defined(LINKS, Some(0.0)), defined(LINKS, None));
}
