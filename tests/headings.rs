//! A heading reaches the picture as written, in every kind that takes one.
//!
//! Mermaid's two escape vocabularies are not interchangeable: `#lt;` is
//! decoded to the character, which the label reader then takes for markup,
//! and `&lt;` stays an entity but its `;` ends an unquoted statement. Hence
//! one table per kind, and this test says which belongs where.
use markdoc_mermaid::{Anchor, Anchors, Options, render};

struct H(&'static str);
impl Anchors for H {
    fn get(&self, id: &str) -> Option<Anchor> {
        matches!(id, "q1" | "step_one" | "a").then(|| Anchor {
            label: self.0.into(),
            href: "#x".into(),
        })
    }
}

/// Every character the diagram draws, with no whitespace: wrapping is the
/// renderer's business, and it breaks a label into several `<text>`
/// elements.
fn drawn(svg: &str) -> String {
    let body = match (svg.find("<style>"), svg.find("</style>")) {
        (Some(open), Some(close)) if close > open => {
            format!("{}{}", &svg[..open], &svg[close + 8..])
        }
        _ => svg.to_string(),
    };
    let mut text = String::new();
    for fragment in body.split("<text").skip(1) {
        let Some((_, rest)) = fragment.split_once('>') else {
            continue;
        };
        let Some(end) = rest.find("</text>") else {
            continue;
        };
        // Tags out, their content kept. A bare `>` is legal text, so this
        // looks for a tag's opening rather than counting angle brackets.
        let mut rest = &rest[..end];
        while let Some(at) = rest.find('<') {
            text.push_str(&rest[..at]);
            match rest[at..].find('>') {
                Some(close) => rest = &rest[at + close + 1..],
                None => {
                    rest = "";
                    break;
                }
            }
        }
        text.push_str(rest);
        text.push(' ');
    }
    let text = text
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

const HEADINGS: [&str; 10] = [
    "Press <Enter> to confirm",
    "<img src=x onerror=alert(1)>",
    "C# and issue #42 and #notanentity;",
    "Fuse rated #amperes; and #lta; values",
    "Voltage < 24 V & > 12 V",
    "Check the \"E-stop\" relay",
    // Markdown, which two kinds read in a label.
    "`reset` the controller",
    "**not bold** and *not italic*",
    "max_speed_limit and _under_",
    "100% of 3 * 4 | 5",
];

/// A state label merman reads as markdown whatever the escape, so paired
/// `*…*`, `**…**` and `_…_` are lost. Upstream.
fn merman_reads_a_state_label_as_markdown(kind: &str, heading: &str) -> bool {
    kind == "state" && (heading.contains("**") || heading.contains("_under_"))
}

#[test]
fn every_kind_that_takes_a_heading_draws_it_as_written() {
    for (kind, src) in [
        ("flowchart", "flowchart TD\n  step_one --> step_two\n"),
        ("sequence", "sequenceDiagram\n  q1->>q2: hi\n"),
        (
            "state",
            "stateDiagram-v2\n  q1 --> q2\n  state \"Plain\" as q2\n",
        ),
        ("block", "block-beta\n  columns 2\n  a b\n"),
        ("class", "classDiagram\n  q1 --> q2\n"),
        ("swimlane", "swimlane-beta LR\n  subgraph C\n    a\n  end\n"),
    ] {
        for heading in HEADINGS {
            if merman_reads_a_state_label_as_markdown(kind, heading) {
                continue;
            }
            let diagram = render(src, &H(heading), &Options::default())
                .unwrap_or_else(|e| panic!("{kind} with {heading:?}: {e}"));
            let want: String = heading.chars().filter(|c| !c.is_whitespace()).collect();
            let got = drawn(&diagram.svg);
            assert!(
                got.contains(&want),
                "{kind} lost {heading:?}, drawing {got:?}"
            );
        }
    }
}

#[test]
fn one_awkward_heading_does_not_empty_the_other_labels() {
    // One such heading in a state diagram took every other label with it.
    let diagram = render(
        "stateDiagram-v2\n  q1 --> q2\n  state \"Plain one\" as q2\n",
        &H("Press <Enter> to confirm"),
        &Options::default(),
    )
    .expect("renders");
    let got = drawn(&diagram.svg);
    assert!(
        got.contains("Plainone"),
        "the plain label went too: {got:?}"
    );
    assert!(
        got.contains("Press<Enter>toconfirm"),
        "the heading is not drawn: {got:?}"
    );
}

#[test]
fn a_link_carries_no_tooltip_to_repeat_the_label() {
    // Mermaid's third `click` argument becomes a `title`, which a screen
    // reader announces before the link's own name.
    let diagram = render(
        "flowchart TD\n  step_one --> step_two\n",
        &H("Press <Enter> to confirm"),
        &Options::default(),
    )
    .expect("renders");
    assert!(
        !diagram.svg.contains("<title"),
        "a tooltip is still emitted: {}",
        diagram.svg
    );
    assert!(
        diagram.svg.contains("#x"),
        "the link itself is gone: {}",
        diagram.svg
    );
}

#[test]
fn mermans_own_placeholders_do_not_reach_the_page() {
    // merman rewrites `&name;` as `ﬂ°name¶ß` while it parses, and some of
    // those reach the output.
    for heading in HEADINGS {
        let diagram = render(
            "flowchart TD\n  step_one --> step_two\n",
            &H(heading),
            &Options::default(),
        )
        .expect("renders");
        assert!(
            !diagram.svg.contains('\u{fb02}') && !diagram.svg.contains('\u{b6}'),
            "a placeholder reached the page for {heading:?}"
        );
    }
}

#[test]
fn an_empty_heading_draws_an_empty_box_rather_than_failing() {
    // A heading can be empty -- one holding only an image -- and `id[""]`
    // is not something merman will parse.
    for (kind, src) in [
        ("flowchart", "flowchart TD\n  step_one --> step_two\n"),
        ("sequence", "sequenceDiagram\n  q1->>q2: hi\n"),
        (
            "state",
            "stateDiagram-v2\n  q1 --> q2\n  state \"Plain\" as q2\n",
        ),
        ("block", "block-beta\n  columns 2\n  a b\n"),
        ("class", "classDiagram\n  q1 --> q2\n"),
        ("swimlane", "swimlane-beta LR\n  subgraph C\n    a\n  end\n"),
    ] {
        for heading in ["", "   ", "\n"] {
            let diagram = render(src, &H(heading), &Options::default())
                .unwrap_or_else(|e| panic!("{kind} with {heading:?}: {e}"));
            // Empty rather than falling back to the id.
            let drawn = drawn(&diagram.svg);
            for id in ["q1", "step_one"] {
                assert!(
                    !drawn.contains(id),
                    "{kind}: an empty heading drew the id {id:?}: {drawn:?}"
                );
            }
        }
    }
}
