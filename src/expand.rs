//! The id-resolution layer: turn anchor ids in Mermaid source into labels.
//!
//! ## What problem this solves
//!
//! Mermaid puts the label in the source:
//!
//! ```text
//! flowchart TD
//!   A[Can you ping the switch?] --> B[Is the switch powered?]
//! ```
//!
//! That text is a *second copy* of a heading that already exists in the
//! document, and a second copy is free to drift. Rename the heading and the
//! diagram keeps the old wording with nothing to warn you — which is exactly
//! how the hand-drawn diagrams this replaces went stale.
//!
//! So an author writes **ids, not labels**:
//!
//! ```text
//! flowchart TD
//!   troubleshooting_cannot_connect_can2 --> troubleshooting_cannot_connect_is
//! ```
//!
//! and this module appends what Mermaid needs:
//!
//! ```text
//!   troubleshooting_cannot_connect_can2["Can you ping the switch?"]
//!   click troubleshooting_cannot_connect_can2 "#troubleshooting_cannot_connect_can2"
//! ```
//!
//! The box text and the heading are then one string, and renaming the step
//! redraws the diagram.
//!
//! ## Appending, not rewriting
//!
//! The author's own lines are never edited. Mermaid lets a node be declared
//! on a line of its own, anywhere in the body, so a label can be *added*
//! after the fact. Rewriting edge lines in place would mean understanding
//! every arrow form Mermaid has (`-->`, `---`, `-.->`, `==>`, `--x`, `--o`,
//! and the labelled variants of each); appending needs none of that, and a
//! form this module has never heard of still passes through intact.
//!
//! ## Why there is no Mermaid parser here
//!
//! Finding "the identifiers" in arbitrary Mermaid is not a small job, but it
//! does not have to be done well, because **the resolver is the filter**. An
//! identifier only becomes a label if the caller's anchor index knows it, and
//! anchor ids are long and distinctive — `troubleshooting_safety_system_…`.
//! A bare word in an edge label (`-- Ping failed -->`) resolves to nothing
//! and is left alone.
//!
//! The scanner still skips quoted strings, bracketed labels, pipe labels and
//! `%%` comments. Not because an id is likely to appear in prose, but because
//! if one ever did, turning a word of a sentence into a link would be a
//! baffling thing for a reader to meet.

use std::collections::BTreeSet;

use mermaid_rs_renderer::DiagramKind;

use crate::{Anchor, Anchors};

/// Mermaid source with labels and links filled in, and what was filled in.
#[derive(Debug, Clone)]
pub struct Expanded {
    pub source: String,
    /// Ids that were given a label, in the order the diagram names them.
    /// Empty means nothing resolved — the diagram will still render, but it
    /// is hand-labelled, or of a kind with no declaration form at all, and
    /// the no-drift guarantee does not apply to it. Never an id whose label
    /// the diagram's own grammar would not read.
    pub resolved: Vec<String>,
}

/// Fill in labels and `click` links for every id the resolver knows.
///
/// `link` controls whether `click` directives are emitted. They are what make
/// a node navigable, and in a PDF what makes it an internal jump — but only
/// when the href is a bare `#fragment`; an absolute path becomes an external
/// URI action a reader cannot follow offline. Giving [`Anchor::href`] the
/// short form is the caller's job, because only the caller knows which page
/// is being rendered.
/// How one diagram kind is told that an id has a label.
///
/// There is no general form. Each kind's own declaration is the only thing its
/// parser reads, and most kinds have none at all: a mindmap or a treemap node
/// *is* its text at an indent, so there is nothing to attach a label to, and an
/// ER entity is drawn by its id with no alias anywhere in the grammar. Where
/// that is so this module resolves nothing for the kind rather than appending a
/// line the parser will mistake for another node — which is what the flowchart
/// form did when it was used for every kind: `resolved` named the ids while the
/// labels went nowhere, or worse, arrived as extra boxes.
#[derive(Clone, Copy)]
struct Dialect {
    /// Writes the declaration that gives `id` the label `label`.
    declare: fn(id: &str, label: &str) -> String,
    /// Whether the parser reads `click`. Only the flowchart does. For the
    /// block, ER and mindmap parsers a `click` line becomes nodes of its own
    /// words, so emitting one there damages the diagram.
    click: bool,
    /// Whether naming one node obliges this kind to name them all. The
    /// sequence validator rejects a message to a participant that no
    /// `participant` line declared, so declaring one means declaring each.
    declare_all: bool,
}

fn dialect(kind: DiagramKind) -> Option<Dialect> {
    // Verified against the parsers rather than assumed: each form below labels
    // an existing node and adds none, and no other kind has a form that does.
    match kind {
        DiagramKind::Flowchart => Some(Dialect {
            declare: |id, label| format!("  {id}[\"{label}\"]\n"),
            click: true,
            declare_all: false,
        }),
        // A swimlane and a use case diagram have flowchart bodies, ids the
        // author writes and all, so they take the flowchart's own form.
        // TreeView and EventModeling do not appear here: both derive a node's
        // id from its text, so there is no id for an author to write and
        // nothing resolution could match.
        DiagramKind::Swimlane | DiagramKind::UseCase => Some(Dialect {
            declare: |id, label| format!("  {id}[\"{label}\"]\n"),
            click: true,
            declare_all: false,
        }),
        DiagramKind::Block => Some(Dialect {
            declare: |id, label| format!("  {id}[\"{label}\"]\n"),
            click: false,
            declare_all: false,
        }),
        DiagramKind::State => Some(Dialect {
            declare: |id, label| format!("  state \"{label}\" as {id}\n"),
            click: false,
            declare_all: false,
        }),
        DiagramKind::Class => Some(Dialect {
            declare: |id, label| format!("  class \"{label}\" as {id}\n"),
            click: false,
            declare_all: false,
        }),
        DiagramKind::Sequence => Some(Dialect {
            declare: |id, label| format!("  participant {id} as {label}\n"),
            click: false,
            declare_all: true,
        }),
        _ => None,
    }
}

/// Whether this kind can be told about an id at all.
pub fn resolves(kind: DiagramKind) -> bool {
    dialect(kind).is_some()
}

/// `nodes` is the diagram as the renderer's own parser read it, before any of
/// this: each id with the label it already has. Two things need it. A kind
/// that must name every node needs the full list, and an id the author
/// labelled themselves has to be left alone — which the scanner cannot tell,
/// because it only knows the bracket forms, while `participant x as Name`,
/// `state "Name" as x` and `class "Name" as x` all put the label somewhere it
/// never looks. The parser knows, so it is asked: a label that is not the id
/// is one the author wrote.
pub fn expand(
    source: &str,
    anchors: &dyn Anchors,
    link: bool,
    kind: DiagramKind,
    nodes: &[(String, String)],
) -> Expanded {
    let Some(dialect) = dialect(kind) else {
        return Expanded {
            source: source.to_string(),
            resolved: Vec::new(),
        };
    };
    let scan = scan(source);
    let mut declared: Vec<(String, Anchor)> = Vec::new();

    let authored = |id: &str| {
        nodes
            .iter()
            .any(|(node_id, label)| node_id == id && label != id)
    };

    for id in &scan.bare {
        // An id the author labelled themselves is theirs; this module does
        // not overrule it, and a diagram that mixes the two still works.
        if scan.labelled.contains(id) || authored(id) {
            continue;
        }
        if let Some(anchor) = anchors.get(id) {
            declared.push((id.clone(), anchor));
        }
    }

    if declared.is_empty() {
        return Expanded {
            source: source.to_string(),
            resolved: Vec::new(),
        };
    }

    let mut out = source.trim_end().to_string();
    out.push_str("\n\n%% labels and links resolved from document anchors\n");
    for (id, anchor) in &declared {
        out.push_str(&(dialect.declare)(id, &quote(&anchor.label)));
    }
    if dialect.declare_all {
        // The ones with no heading still have to be named, or the diagram
        // that parsed before this ran stops parsing now. A bare
        // `participant x` does not disturb a label the author gave it.
        for (id, _) in nodes {
            if !declared.iter().any(|(declared_id, _)| declared_id == id) {
                out.push_str(&format!("  participant {id}\n"));
            }
        }
    }
    if link && dialect.click {
        for (id, anchor) in &declared {
            // Skip an id the author already wired up: two `click` lines for
            // one node is Mermaid's problem to resolve, and it should not
            // have to.
            if scan.clicked.contains(id) {
                continue;
            }
            out.push_str(&format!(
                "  click {id} \"{}\" \"{}\"\n",
                quote(&anchor.href),
                quote(&anchor.label)
            ));
        }
    }

    Expanded {
        source: out,
        resolved: declared.into_iter().map(|(id, _)| id).collect(),
    }
}

/// A label goes inside `["…"]`, so a quote in it would close the label early.
/// `#quot;` is Mermaid's own escape. A newline becomes `<br/>`, which is how
/// Mermaid breaks a line inside a node.
///
/// Both are kept now that the renderer reads them back, and keeping them is
/// not a leftover: the escape is the only way a quotation mark can travel
/// inside a bracketed label at all, which is why Mermaid has it. What was
/// missing was the other half -- until markdoc-mermaid 0.3.0 nothing decoded
/// `#quot;`, so a heading with a quotation mark drew the escape instead of the
/// mark, and the label was not the heading after all. The round trip is now
/// the same one Mermaid itself does.
fn quote(value: &str) -> String {
    value
        .replace('"', "#quot;")
        .replace("\r\n", "<br/>")
        .replace(['\n', '\r'], "<br/>")
}

#[derive(Default)]
struct Scan {
    /// Identifiers seen outside any label, string or comment, in order of
    /// first appearance so the appended block follows the diagram.
    bare: Vec<String>,
    /// Identifiers the author gave a label to — `A[…]`, `A(…)`, `A{…}`.
    labelled: BTreeSet<String>,
    /// Identifiers already named by a `click` directive.
    clicked: BTreeSet<String>,
}

fn scan(source: &str) -> Scan {
    let mut out = Scan::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    // The identifier immediately before the cursor, so an opening bracket can
    // be attributed to the node it labels.
    let mut previous: Option<String> = None;
    // Set when the word `click` has just been read, so the id after it is
    // recorded as already wired.
    let mut after_click = false;

    while i < chars.len() {
        let c = chars[i];

        // `%%` to end of line is a comment.
        if c == '%' && chars.get(i + 1) == Some(&'%') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            previous = None;
            after_click = false;
            continue;
        }

        // A quoted string, anywhere.
        if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            i += 1;
            previous = None;
            continue;
        }

        // `|edge label|`.
        if c == '|' {
            i += 1;
            while i < chars.len() && chars[i] != '|' {
                i += 1;
            }
            i += 1;
            previous = None;
            continue;
        }

        // A bracketed label belongs to the identifier just before it.
        if let Some(close) = closing(c) {
            if let Some(id) = previous.take() {
                out.labelled.insert(id);
            }
            let mut depth = 1usize;
            i += 1;
            while i < chars.len() && depth > 0 {
                // A quote inside a label may itself hold brackets.
                if chars[i] == '"' {
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        i += 1;
                    }
                } else if chars[i] == c {
                    depth += 1;
                } else if chars[i] == close {
                    depth -= 1;
                }
                i += 1;
            }
            continue;
        }

        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            // A hyphen belongs to the word only when a letter or digit
            // follows it, which is what separates a slug from an arrow:
            // `cannot-connect` is one id, while `a-->b` stops at `a`
            // because the next character is another `-`, and `a-.->b`
            // because it is a `.`. Documentation anchors are usually
            // hyphenated, so a scanner that stopped at the first `-` never
            // saw most of the ids it exists to find.
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '_'
                    || (chars[i] == '-'
                        && chars
                            .get(i + 1)
                            .is_some_and(|n| n.is_ascii_alphanumeric() || *n == '_')))
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if after_click {
                out.clicked.insert(word.clone());
                after_click = false;
            } else if word == "click" {
                after_click = true;
            }
            if seen.insert(word.clone()) {
                out.bare.push(word.clone());
            }
            previous = Some(word);
            continue;
        }

        // A space keeps the previous identifier in view, because `A ["x"]`
        // labels `A` just as `A["x"]` does. A newline, or anything that is
        // not whitespace, breaks the association.
        if c == '\n' {
            previous = None;
            after_click = false;
        } else if !c.is_whitespace() {
            previous = None;
        }
        i += 1;
    }
    out
}

fn closing(open: char) -> Option<char> {
    match open {
        '[' => Some(']'),
        '(' => Some(')'),
        '{' => Some('}'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    // Every test here is a flowchart; `expand`'s other arguments are the
    // diagram kind and, for a sequence diagram, the participants to name.
    fn expand_flowchart(source: &str, anchors: &dyn Anchors, link: bool) -> Expanded {
        expand(source, anchors, link, DiagramKind::Flowchart, &[])
    }

    use super::*;
    use std::collections::HashMap;

    struct Map(HashMap<&'static str, (&'static str, &'static str)>);

    impl Anchors for Map {
        fn get(&self, id: &str) -> Option<Anchor> {
            self.0.get(id).map(|(label, href)| Anchor {
                label: (*label).to_string(),
                href: (*href).to_string(),
            })
        }
    }

    fn anchors() -> Map {
        Map(HashMap::from([
            ("ts_can2", ("Can you ping the switch?", "#ts_can2")),
            ("ts_is", ("Is the switch powered?", "#ts_is")),
            (
                "ts_does",
                ("Does the power cable deliver 19 V?", "#ts_does"),
            ),
        ]))
    }

    #[test]
    fn a_bare_id_gets_the_heading_as_its_label() {
        let out = expand_flowchart("flowchart TD\n  ts_can2 --> ts_is\n", &anchors(), true);
        assert!(
            out.source
                .contains(r#"ts_can2["Can you ping the switch?"]"#),
            "{}",
            out.source
        );
        assert!(
            out.source.contains(r#"ts_is["Is the switch powered?"]"#),
            "{}",
            out.source
        );
        assert_eq!(out.resolved, vec!["ts_can2", "ts_is"]);
    }

    #[test]
    fn the_authors_own_lines_are_never_touched() {
        // Appending is what lets an arrow form this crate has never heard of
        // pass through intact.
        let src = "flowchart TD\n  ts_can2 -. retry .-> ts_is\n  ts_is ==> ts_does\n";
        let out = expand_flowchart(src, &anchors(), true);
        assert!(out.source.starts_with(src.trim_end()), "{}", out.source);
    }

    #[test]
    fn a_node_becomes_a_link() {
        let out = expand_flowchart("flowchart TD\n  ts_can2 --> ts_is\n", &anchors(), true);
        assert!(
            out.source.contains("click ts_can2 \"#ts_can2\""),
            "{}",
            out.source
        );
    }

    #[test]
    fn links_can_be_left_out() {
        let out = expand_flowchart("flowchart TD\n  ts_can2 --> ts_is\n", &anchors(), false);
        assert!(!out.source.contains("click"), "{}", out.source);
        assert!(out.source.contains("ts_can2[\""), "{}", out.source);
    }

    #[test]
    fn a_label_the_author_wrote_wins() {
        // Mixing the two is allowed; this module does not overrule a person.
        let out = expand_flowchart(
            "flowchart TD\n  ts_can2[Ping the switch] --> ts_is\n",
            &anchors(),
            true,
        );
        assert!(!out.source.contains("ts_can2[\"Can you"), "{}", out.source);
        assert!(
            out.source.contains(r#"ts_is["Is the switch powered?"]"#),
            "{}",
            out.source
        );
        assert_eq!(out.resolved, vec!["ts_is"]);
    }

    #[test]
    fn a_label_separated_by_a_space_still_counts_as_the_authors() {
        let out = expand_flowchart(
            "flowchart TD\n  ts_can2 [Mine] --> ts_is\n",
            &anchors(),
            true,
        );
        assert_eq!(out.resolved, vec!["ts_is"]);
    }

    #[test]
    fn an_edge_label_is_not_mistaken_for_an_id() {
        // The resolver is the filter: `Ping` and `failed` resolve to nothing.
        let out = expand_flowchart(
            "flowchart TD\n  ts_can2 -- Ping failed --> ts_is\n",
            &anchors(),
            true,
        );
        assert_eq!(out.resolved, vec!["ts_can2", "ts_is"]);
        assert!(!out.source.contains("Ping[\""), "{}", out.source);
    }

    #[test]
    fn an_id_inside_prose_is_left_alone() {
        // Turning a word of a sentence into a link would baffle a reader.
        let out = expand_flowchart(
            "flowchart TD\n  a[\"See ts_can2 for detail\"] --> b\n",
            &anchors(),
            true,
        );
        assert!(out.resolved.is_empty(), "{:?}", out.resolved);
    }

    #[test]
    fn a_comment_is_not_scanned() {
        let out = expand_flowchart(
            "flowchart TD\n  %% ts_can2 is next\n  ts_is --> ts_does\n",
            &anchors(),
            true,
        );
        assert_eq!(out.resolved, vec!["ts_is", "ts_does"]);
    }

    #[test]
    fn a_pipe_label_is_not_scanned() {
        let out = expand_flowchart(
            "flowchart TD\n  ts_is -->|ts_can2| ts_does\n",
            &anchors(),
            true,
        );
        assert_eq!(out.resolved, vec!["ts_is", "ts_does"]);
    }

    #[test]
    fn a_click_the_author_wrote_is_not_duplicated() {
        let src = "flowchart TD\n  ts_can2 --> ts_is\n  click ts_can2 \"/elsewhere\"\n";
        let out = expand_flowchart(src, &anchors(), true);
        assert_eq!(
            out.source.matches("click ts_can2").count(),
            1,
            "{}",
            out.source
        );
        // and it still gets its label
        assert!(out.source.contains("ts_can2[\"Can you"), "{}", out.source);
    }

    #[test]
    fn a_quote_in_a_heading_does_not_break_the_label() {
        let map = Map(HashMap::from([("q", (r#"The "Power" button"#, "#q"))]));
        let out = expand_flowchart("flowchart TD\n  q --> q2\n", &map, true);
        assert!(
            out.source.contains(r#"q["The #quot;Power#quot; button"]"#),
            "{}",
            out.source
        );
    }

    #[test]
    fn nothing_resolving_leaves_the_source_exactly_as_it_was() {
        let src = "flowchart TD\n  A[One] --> B[Two]\n";
        let out = expand_flowchart(src, &anchors(), true);
        assert_eq!(out.source, src);
        assert!(out.resolved.is_empty());
    }

    #[test]
    fn declarations_follow_the_order_of_the_diagram() {
        // So a human reading the expanded source can follow it against the
        // original rather than against an alphabet.
        let out = expand_flowchart(
            "flowchart TD\n  ts_does --> ts_can2\n  ts_can2 --> ts_is\n",
            &anchors(),
            true,
        );
        assert_eq!(out.resolved, vec!["ts_does", "ts_can2", "ts_is"]);
    }
}
