//! The SVG is XML, and XML defines five entity names.
//!
//! merman replaces every `&name;` and `#name;` in the source with
//! placeholders of its own while it parses, and keeps them in four
//! attributes. Written back as `&name;` they would be names XML does not
//! define, which usvg refuses the whole diagram over, so the references are
//! resolved and the characters written out.
use markdoc_mermaid::{Anchor, Anchors, Options, render};

struct H(&'static str);
impl Anchors for H {
    fn get(&self, id: &str) -> Option<Anchor> {
        matches!(id, "q1" | "step_one" | "a").then(|| Anchor {
            label: self.0.into(),
            href: "https://example.com/a#b".into(),
        })
    }
}

/// Every entity name in `svg` that XML does not define.
fn undefined_entities(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(at) = rest.find('&') {
        rest = &rest[at + 1..];
        let Some(end) = rest.find(';').filter(|&end| end <= 40) else {
            continue;
        };
        let name = &rest[..end];
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        if !matches!(name, "lt" | "gt" | "amp" | "quot" | "apos")
            && name.chars().all(|c| c.is_ascii_alphanumeric())
        {
            out.push(name.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn no_diagram_carries_an_entity_name_that_xml_does_not_define() {
    // The four attributes merman keeps placeholders in, and a label for
    // comparison.
    for (what, src) in [
        (
            "a click tooltip",
            "flowchart TD\n  a --> b\n  click a \"https://example.com\" \"Opens the guide #mdash; in a new tab\"\n",
        ),
        (
            "a kanban card id",
            "kanban\n  Todo\n    Write the manual #hellip;\n",
        ),
        (
            "a gitGraph commit",
            "gitGraph\n  commit id: \"Fix #hellip;\"\n",
        ),
        (
            "a flowchart label",
            "flowchart TD\n  a[\"Fine #hellip; here\"] --> b\n",
        ),
        (
            "a label and a tooltip together",
            "flowchart TD\n  a[\"Tip #copy; #nbsp; #hellip;\"] --> b\n  click a \"/x\" \"Tip #copy; #nbsp; #hellip;\"\n",
        ),
    ] {
        let diagram = render(src, &H("A heading"), &Options::default())
            .unwrap_or_else(|e| panic!("{what}: {e}"));
        assert!(
            undefined_entities(&diagram.svg).is_empty(),
            "{what} left {:?} in the SVG",
            undefined_entities(&diagram.svg)
        );
    }
}

#[test]
fn a_tooltip_reads_as_the_author_wrote_it() {
    let diagram = render(
        "flowchart TD\n  a --> b\n  click a \"/x\" \"Opens the guide #mdash; #copy; 2026\"\n",
        &H("A heading"),
        &Options::default(),
    )
    .expect("renders");
    let title = diagram
        .svg
        .split("title=\"")
        .nth(1)
        .and_then(|f| f.split('"').next())
        .expect("a title");
    assert_eq!(title, "Opens the guide \u{2014} \u{a9} 2026");
}

#[test]
fn an_escape_in_a_heading_survives_every_kind_that_takes_one() {
    // A heading of Mermaid's escapes for names XML does not define, through
    // each kind's own declaration form.
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
        let diagram = render(
            src,
            &H("Fuse #copy; rated #nbsp; 10 A #hellip;"),
            &Options::default(),
        )
        .unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert!(
            undefined_entities(&diagram.svg).is_empty(),
            "{kind} left {:?} in the SVG",
            undefined_entities(&diagram.svg)
        );
    }
}
