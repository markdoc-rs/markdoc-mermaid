//! The id-resolution layer: turn anchor ids in Mermaid source into labels.
//!
//! An author writes ids rather than labels, and this module appends the
//! declaration the kind's own grammar reads:
//!
//! ```text
//!   troubleshooting_cannot_connect_can2["Can you ping the switch?"]
//!   click troubleshooting_cannot_connect_can2 "#troubleshooting_cannot_connect_can2"
//! ```
//!
//! The author's own lines are never edited, so an arrow form this module has
//! never heard of passes through intact. There is no Mermaid parser here: an
//! identifier becomes a label only where the caller's anchor index knows it,
//! and the scanner skips quoted strings, bracketed labels, pipe labels and
//! `%%` comments.

use std::collections::BTreeSet;

use crate::{Anchor, Anchors};

/// Mermaid source with labels and links filled in, and what was filled in.
#[derive(Debug, Clone)]
pub struct Expanded {
    pub source: String,
    /// Ids that were given a label, in the order the diagram names them.
    /// Empty means nothing resolved: the diagram still renders, but it is
    /// hand-labelled or of a kind with no declaration form at all.
    pub resolved: Vec<String>,
}

/// Fill in labels and `click` links for every id the resolver knows.
///
/// `link` controls whether `click` directives are emitted. A bare
/// `#fragment` [`Anchor::href`] becomes an internal jump in a PDF, where an
/// absolute path becomes an external URI action.
/// How one diagram kind is told that an id has a label.
///
/// There is no general form, and most kinds have none at all: a mindmap node
/// *is* its text at an indent, an ER entity is drawn by its id. Those kinds
/// resolve nothing, rather than taking an appended line for another node.
#[derive(Clone, Copy)]
struct Dialect {
    /// Writes the declaration that gives `id` the label `label`.
    declare: fn(id: &str, label: &str) -> String,
    /// Whether the parser reads `click`. Elsewhere the line becomes nodes of
    /// its own words.
    click: bool,
    /// Whether naming one node obliges this kind to name them all.
    declare_all: bool,
    /// Which escape vocabulary this kind's labels are written in.
    escapes: Escapes,
}

/// Which of Mermaid's two escape vocabularies a kind's labels are written in.
///
/// `#lt;` is decoded to the character itself, which the label reader then
/// takes for markup; `&lt;` stays an entity, but its `;` ends an unquoted
/// statement. So entities where the label is quoted, hashes where it is not.
#[derive(Clone, Copy)]
enum Escapes {
    /// `&lt;`, `&gt;`, `&amp;`, `&quot;`.
    Entities,
    /// `#lt;`, `#gt;`, `#amp;`, `#quot;`.
    Hashes,
}

fn dialect(kind: &str) -> Option<Dialect> {
    // merman's own `diagram_type`: a flowchart and a `graph` both report
    // `flowchart-v2`.
    match kind {
        "flowchart-v2" | "flowchart" => Some(Dialect {
            declare: |id, label| format!("  {id}[\"{label}\"]\n"),
            click: true,
            declare_all: false,
            escapes: Escapes::Entities,
        }),
        // A swimlane's body is a flowchart, and merman reports its model as
        // one, so it takes the flowchart's form, `click` included.
        "swimlane" => Some(Dialect {
            declare: |id, label| format!("  {id}[\"{label}\"]\n"),
            click: true,
            declare_all: false,
            escapes: Escapes::Entities,
        }),
        // A use case diagram is not here: in merman an id carries a *kind*,
        // and labelling one without re-declaring its kind is rejected.
        "block" => Some(Dialect {
            declare: |id, label| format!("  {id}[\"{label}\"]\n"),
            click: false,
            declare_all: false,
            escapes: Escapes::Entities,
        }),
        "stateDiagram" => Some(Dialect {
            declare: |id, label| format!("  state \"{label}\" as {id}\n"),
            click: false,
            declare_all: false,
            escapes: Escapes::Entities,
        }),
        "classDiagram" => Some(Dialect {
            declare: |id, label| format!("  class {id}[\"{label}\"]\n"),
            click: false,
            declare_all: false,
            escapes: Escapes::Entities,
        }),
        // A message to a participant no `participant` line declared is
        // rejected, so naming one means naming them all.
        "sequence" => Some(Dialect {
            declare: |id, label| format!("  participant {id} as {label}\n"),
            click: false,
            declare_all: true,
            escapes: Escapes::Hashes,
        }),
        _ => None,
    }
}

/// Whether this kind can be told about an id at all.
pub fn resolves(kind: &str) -> bool {
    dialect(kind).is_some()
}

/// `nodes` is the diagram as the renderer's own parser read it: each id with
/// the label it already has. A kind that must name every node needs the full
/// list, and an id the author labelled themselves is left alone -- a label
/// that is not the id is one they wrote.
pub fn expand(
    source: &str,
    anchors: &dyn Anchors,
    link: bool,
    kind: &str,
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

    // The candidates are merman's nodes, in its order: the parser has run
    // and knows which ids there are, so `scan` only says which of them the
    // author labelled or wired up.
    for (id, _) in nodes {
        // An id the author labelled themselves is theirs.
        if scan.labelled.contains(id) || authored(id) {
            continue;
        }
        // A node merman made up rather than read is not one to label.
        if !written(source, id) {
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
        out.push_str(&(dialect.declare)(
            id,
            &quote(&anchor.label, dialect.escapes),
        ));
    }
    if dialect.declare_all {
        // The ones with no heading still have to be named, or the diagram
        // that parsed before this ran stops parsing now.
        for (id, _) in nodes {
            if !declared.iter().any(|(declared_id, _)| declared_id == id) {
                out.push_str(&format!("  participant {id}\n"));
            }
        }
    }
    if link && dialect.click {
        for (id, anchor) in &declared {
            // Skip an id the author already wired up.
            if scan.clicked.contains(id) {
                continue;
            }
            // The href is a URL, not label text: `quote` would write its
            // `#` as `#35;`, so only the quote character has to go. No
            // tooltip, since the box already draws the heading.
            out.push_str(&format!(
                "  click {id} \"{}\"\n",
                anchor.href.replace('"', "%22")
            ));
        }
    }

    Expanded {
        source: out,
        resolved: declared.into_iter().map(|(id, _)| id).collect(),
    }
}

/// A label goes inside `["…"]`, so a quote in it is written as Mermaid's own
/// escape, which would otherwise close the label early. A newline becomes
/// `<br/>`, which is how Mermaid breaks a line inside a node.
fn quote(value: &str, escapes: Escapes) -> String {
    // Mermaid reads a label as HTML, so a heading's `<`, `>`, `&` and `#…;`
    // are each written as the escape Mermaid defines for it.
    //
    // One pass, because every escape introduces characters that would
    // themselves be escaped -- a `;` among them, which ends an unquoted
    // statement, and the `<br/>` a line break becomes.
    //
    // An empty label fails the diagram, and a heading can be empty, so a
    // no-break space draws the empty box it asks for.
    if value.trim().is_empty() {
        return "#160;".to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '#' => out.push_str("#35;"),
            ';' => out.push_str("#59;"),
            // Escaped, so the kinds that read a label as markdown draw a
            // backtick and a `**` as the heading has them.
            '`' => out.push_str("#96;"),
            '*' => out.push_str("#42;"),
            '_' => out.push_str("#95;"),
            '"' => out.push_str(match escapes {
                Escapes::Entities => "&quot;",
                Escapes::Hashes => "#quot;",
            }),
            '<' => out.push_str(match escapes {
                Escapes::Entities => "&lt;",
                Escapes::Hashes => "#lt;",
            }),
            '>' => out.push_str(match escapes {
                Escapes::Entities => "&gt;",
                Escapes::Hashes => "#gt;",
            }),
            '&' => out.push_str(match escapes {
                Escapes::Entities => "&amp;",
                Escapes::Hashes => "#amp;",
            }),
            '\r' | '\n' => {
                // One break for `\r\n`, not two.
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str("<br/>");
            }
            c => out.push(c),
        }
    }
    out
}

/// Whether `id` appears in the source as a word of its own, not inside a
/// longer identifier. A hyphen ends a word, and an arrowhead written against
/// an id -- the `x` of `-xbob` -- belongs to the arrow rather than the word.
fn written(source: &str, id: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    !id.is_empty()
        && source.match_indices(id).any(|(at, _)| {
            let mut before = source[..at].chars().rev();
            let starts = match before.next() {
                None => true,
                Some('x' | 'o') => matches!(before.next(), Some('-' | '=' | '.')),
                Some(c) => !word(c),
            };
            let ends = source[at + id.len()..]
                .chars()
                .next()
                .is_none_or(|c| !word(c));
            starts && ends
        })
}

#[derive(Default)]
struct Scan {
    /// Identifiers the author gave a label to — `A[…]`, `A(…)`, `A{…}`.
    labelled: BTreeSet<String>,
    /// Identifiers already named by a `click` directive.
    clicked: BTreeSet<String>,
}

fn scan(source: &str) -> Scan {
    let mut out = Scan::default();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    // The identifier immediately before the cursor, so an opening bracket can
    // be attributed to the node it labels.
    let mut previous: Option<String> = None;
    // Set when the word `click` has just been read, so the id after it is
    // recorded as already wired.
    let mut after_click = false;
    // Set when the identifier just read was named by `class` or `state`,
    // whose `{` opens a body rather than a label.
    let mut names_a_body = false;

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

        // `|edge label|`, which ends with the line: a class diagram's arrows
        // carry a `|` of their own (`<|--`, `--|>`), and no Mermaid edge
        // label spans a line.
        if c == '|' {
            let mut end = i + 1;
            while end < chars.len() && chars[end] != '|' && chars[end] != '\n' {
                end += 1;
            }
            if chars.get(end) == Some(&'|') {
                i = end + 1;
                previous = None;
                continue;
            }
            // Part of an arrow: step over it and carry on reading the line.
            i += 1;
            continue;
        }

        // An arrow. The text of `a -- yes --> b` is read on as words, and
        // `yes` dropped in `expand`, where merman has no such node.
        if matches!(c, '-' | '=' | '.') {
            let start = i;
            while i < chars.len() && matches!(chars[i], '-' | '=' | '.' | '>' | '<') {
                i += 1;
            }
            // `--o` and `--x` are arrowheads whatever follows, as Mermaid's
            // lexer takes them -- but only after an arrow that still wants a
            // head, so the `o` of `a-->one` stays a word.
            let open_head = chars
                .get(i.saturating_sub(1))
                .is_some_and(|last| matches!(last, '-' | '='))
                && i > start;
            if open_head && matches!(chars.get(i), Some('o' | 'x')) {
                i += 1;
            }
            previous = None;
            continue;
        }

        // A bracketed label belongs to the identifier just before it, except
        // after `class` or `state`, whose brace opens a body.
        if let Some(close) = closing(c) {
            if let Some(id) = previous.take()
                && !(c == '{' && names_a_body)
            {
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
            // follows, which separates a slug from an arrow:
            // `cannot-connect` is one id, while `a-->b` stops at `a`.
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
            names_a_body = matches!(previous.as_deref(), Some("class" | "state"));
            previous = Some(word);
            continue;
        }

        // A space keeps the previous identifier in view, because `A ["x"]`
        // labels `A` as `A["x"]` does. Anything else breaks the association.
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
    // Every test here is a flowchart, expanded as `render` expands it.
    fn expand_flowchart(source: &str, anchors: &dyn Anchors, link: bool) -> Expanded {
        let probe = crate::parse(
            &crate::engine(&crate::Layout::default(), false, "", 0),
            source,
        )
        .expect("the test's source parses");
        let nodes = crate::labelled_nodes(probe.model());
        expand(source, anchors, link, probe.diagram_type(), &nodes)
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
        // An arrow form this crate has never heard of passes through intact.
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
        // A flowchart's label is quoted, so it takes the entity vocabulary.
        let map = Map(HashMap::from([("q", (r#"The "Power" button"#, "#q"))]));
        let out = expand_flowchart("flowchart TD\n  q --> q2\n", &map, true);
        assert!(
            out.source.contains(r#"q["The &quot;Power&quot; button"]"#),
            "{}",
            out.source
        );
    }

    #[test]
    fn a_sequence_participant_takes_mermaids_own_escapes_instead() {
        // `participant {id} as {label}` is unquoted, so an entity's own `;`
        // would end the statement.
        let map = Map(HashMap::from([("q", (r#"Press <Enter>; go"#, "#q"))]));
        let out = expand(
            "sequenceDiagram\n  q->>q2: hi\n",
            &map,
            false,
            "sequence",
            // label == id is how the model reports a participant nobody
            // labelled.
            &[
                ("q".to_string(), "q".to_string()),
                ("q2".to_string(), "q2".to_string()),
            ],
        );
        assert!(
            out.source
                .contains("participant q as Press #lt;Enter#gt;#59; go"),
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
        // So the expanded source can be read against the original.
        let out = expand_flowchart(
            "flowchart TD\n  ts_does --> ts_can2\n  ts_can2 --> ts_is\n",
            &anchors(),
            true,
        );
        assert_eq!(out.resolved, vec!["ts_does", "ts_can2", "ts_is"]);
    }
}
