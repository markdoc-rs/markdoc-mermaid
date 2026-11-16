//! Integration tests for the strict parse API and `ParseError`.
//!
//! These tests exercise the public `parse_mermaid_strict` and
//! `render_strict` entry points against deliberately-malformed
//! inputs, and assert each produces the correct `ParseError`
//! variant via `matches!`.
//!
//! Coverage: at least 5 cases per `ParseError` variant, 24 total.

use mermaid_rs_renderer::{ParseError, RenderOptions, parse_mermaid_strict, render_strict};

// =====================================================================
// InvalidDirective (5 cases)
// =====================================================================

#[test]
fn invalid_directive_malformed_json() {
    let input = r#"%%{init: {theme dark}}%%
flowchart LR"#;
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::InvalidDirective { ref directive, .. }
            if directive == "init"
    ));
}

#[test]
fn invalid_directive_missing_colon() {
    let input = "%%{init}%%\nflowchart LR\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::InvalidDirective { .. }));
}

#[test]
fn invalid_directive_missing_closing_fence() {
    let input = "%%{init: {\"theme\": \"dark\"}\nflowchart LR\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::InvalidDirective { .. }));
}

#[test]
fn invalid_directive_empty_init_body() {
    let input = "%%{init: }%%\nflowchart LR\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::InvalidDirective { .. }));
}

#[test]
fn invalid_directive_unparseable_nested_json() {
    let input = r#"%%{init: {"theme": "dark", "themeVariables": {primaryColor}}}%%
flowchart LR"#;
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::InvalidDirective { .. }));
}

// =====================================================================
// UnclosedSubgraph (5 cases)
// =====================================================================

#[test]
fn unclosed_subgraph_simple() {
    let input = "flowchart LR\nsubgraph S\n  A --> B\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::UnclosedSubgraph { opened_at: 2 }));
}

#[test]
fn unclosed_subgraph_nested_inner() {
    let input = "flowchart LR\nsubgraph Outer\n  subgraph Inner\n    A --> B\nend\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::UnclosedSubgraph { .. }));
}

#[test]
fn unclosed_subgraph_multiple_opens() {
    let input = "flowchart LR\nsubgraph A\nsubgraph B\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    // Outermost open is reported.
    assert!(matches!(err, ParseError::UnclosedSubgraph { opened_at: 2 }));
}

#[test]
fn unclosed_subgraph_with_nodes_but_no_end() {
    let input = "flowchart TD\nsubgraph DataFlow\n  Input --> Process\n  Process --> Output\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::UnclosedSubgraph { .. }));
}

#[test]
fn unclosed_subgraph_with_title() {
    let input = "flowchart LR\nsubgraph \"My Title\"\n  A --> B\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::UnclosedSubgraph { .. }));
}

// =====================================================================
// UnexpectedToken (7 cases -- three sub-shapes covered)
// =====================================================================

#[test]
fn unexpected_token_stray_end_without_open() {
    let input = "flowchart LR\nA --> B\nend\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken { ref found, ref expected, .. }
            if found == "end" && expected == "matching subgraph"
    ));
}

#[test]
fn unexpected_token_leading_arrow() {
    let input = "flowchart LR\n--> B\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken { ref expected, .. }
            if expected == "node identifier"
    ));
}

#[test]
fn unexpected_token_leading_thick_arrow() {
    let input = "flowchart LR\n==> X\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken { ref expected, .. }
            if expected == "node identifier"
    ));
}

#[test]
fn unexpected_token_leading_dotted_arrow() {
    let input = "flowchart LR\n-.-> Y\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn unexpected_token_click_unbalanced_quote() {
    let input = "flowchart LR\nA --> B\nclick A \"https://example.com\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken { ref expected, .. }
            if expected == "matching double quote"
    ));
}

#[test]
fn unexpected_token_click_three_quotes() {
    let input = "flowchart LR\nA --> B\nclick A \"https://a\" \"tooltip\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn unexpected_token_stray_end_in_sequence() {
    let input = "sequenceDiagram\nAlice->>Bob: hi\nend\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken { ref found, .. }
            if found == "end"
    ));
}

// =====================================================================
// UnknownParticipant (5 cases)
// =====================================================================

#[test]
fn unknown_participant_on_rhs() {
    let input = "sequenceDiagram
participant Alice
participant Bob
Alice->>Carol: hi
";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnknownParticipant { ref name, .. }
            if name == "Carol"
    ));
}

#[test]
fn unknown_participant_on_lhs() {
    let input = "sequenceDiagram
participant Alice
participant Bob
Carol->>Bob: hi
";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnknownParticipant { ref name, .. }
            if name == "Carol"
    ));
}

#[test]
fn unknown_participant_reports_line_number() {
    let input = "sequenceDiagram
participant A
participant B
A->>B: ok
A->>C: bad
";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnknownParticipant { line: 5, .. }
    ));
}

#[test]
fn unknown_participant_with_similar_candidate() {
    let input = "sequenceDiagram
participant Alice
participant Alicia
Alice->>Alicee: typo
";
    let err = parse_mermaid_strict(input).unwrap_err();
    match err {
        ParseError::UnknownParticipant {
            name, candidates, ..
        } => {
            assert_eq!(name, "Alicee");
            assert!(
                candidates.iter().any(|c| c == "Alice" || c == "Alicia"),
                "candidates = {candidates:?}"
            );
        }
        other => panic!("expected UnknownParticipant, got {other:?}"),
    }
}

#[test]
fn unknown_participant_with_actor_declaration() {
    let input = "sequenceDiagram
actor Alice
actor Bob
Alice->>Charlie: hi
";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnknownParticipant { ref name, .. }
            if name == "Charlie"
    ));
}

// =====================================================================
// Happy path (sanity check that valid input still parses / renders)
// =====================================================================

#[test]
fn valid_flowchart_parses_strict() {
    let out = parse_mermaid_strict("flowchart LR\nA --> B\n");
    assert!(out.is_ok(), "got {:?}", out.err());
}

#[test]
fn valid_flowchart_renders_strict() {
    let svg = render_strict("flowchart LR\nA --> B\n", RenderOptions::default())
        .expect("valid flowchart should render");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn valid_sequence_with_declarations_renders_strict() {
    let input = "sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: hi
";
    let svg = render_strict(input, RenderOptions::default()).expect("valid sequence should render");
    assert!(svg.contains("<svg"));
}

// =====================================================================
// Issue #102: `end` inside sequence frames / block-beta groups must
// not be rejected by the subgraph-balance check.
// =====================================================================

#[test]
fn valid_sequence_par_alt_frames_render_strict() {
    let input = "sequenceDiagram
participant A
participant B
par one
A->>B: hello
and two
B->>A: hi
end
alt yes
A->>B: ok
else no
B->>A: nope
end
";
    let svg = render_strict(input, RenderOptions::default())
        .expect("sequence par/alt frames should render");
    assert!(svg.contains("<svg"));
}

#[test]
fn valid_block_beta_named_group_renders_strict() {
    let input = "block-beta
columns 3
a b c
block:group1[\"Group One\"]
d
e
end
f
group1 --> f
";
    let svg = render_strict(input, RenderOptions::default())
        .expect("block-beta named nested group should render");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Group One"));
}

#[test]
fn block_beta_stray_end_is_still_reported() {
    let input = "block-beta\na b\nend\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken { ref found, .. } if found == "end"
    ));
}

#[test]
fn sequence_unclosed_frame_is_reported() {
    let input = "sequenceDiagram\nalt yes\nAlice->>Bob: hi\n";
    let err = parse_mermaid_strict(input).unwrap_err();
    assert!(matches!(err, ParseError::UnclosedSubgraph { opened_at: 2 }));
}

// =====================================================================
// markdoc-mermaid: an unrecognised header is an error, not a flowchart
// =====================================================================

/// Every diagram type mermaid documents but this renderer has no parser for.
/// Each one is a bare word ending in `-beta`, and the lone `-` in that suffix
/// used to satisfy the flowchart-edge fallback in `detect_diagram_kind`, so
/// each came back as a two-box flowchart named after its own header.
const UNSUPPORTED_HEADERS: &[&str] = &["cynefin-beta", "ishikawa-beta", "venn-beta"];

/// The four that are parsed now, each of which is a graph of nodes, edges and
/// groups. The lone `-` in their `-beta` suffix used to satisfy the flowchart
/// fallback in `detect_diagram_kind`, so each came back as a two-box flowchart
/// named after the halves of its own header.
const ADDED_HEADERS: &[(&str, mermaid_rs_renderer::DiagramKind)] = &[
    (
        "swimlane-beta LR",
        mermaid_rs_renderer::DiagramKind::Swimlane,
    ),
    ("usecase-beta", mermaid_rs_renderer::DiagramKind::UseCase),
    ("treeView-beta", mermaid_rs_renderer::DiagramKind::TreeView),
    (
        "eventmodeling",
        mermaid_rs_renderer::DiagramKind::EventModeling,
    ),
];

#[test]
fn the_added_diagram_types_are_their_own_kind() {
    for (header, kind) in ADDED_HEADERS {
        let parsed = parse_mermaid_strict(&format!("{header}\n"))
            .unwrap_or_else(|e| panic!("{header} should parse: {e:?}"));
        assert_eq!(parsed.graph.kind, *kind, "{header}");
    }
}

#[test]
fn unsupported_diagram_header_is_an_error() {
    for header in UNSUPPORTED_HEADERS {
        let input = format!("{header}\n  title Something\n");
        let err = parse_mermaid_strict(&input)
            .map(|p| p.graph.kind)
            .unwrap_err_or_kind();
        assert!(
            err.contains(header),
            "{header}: expected the error to name the header, got {err}"
        );
    }
}

trait UnwrapErrOrKind {
    fn unwrap_err_or_kind(self) -> String;
}

impl UnwrapErrOrKind for Result<mermaid_rs_renderer::DiagramKind, ParseError> {
    fn unwrap_err_or_kind(self) -> String {
        match self {
            Ok(kind) => panic!("expected an error, got {kind:?}"),
            Err(err) => format!("{err:?}"),
        }
    }
}

#[test]
fn a_hyphenated_node_id_is_not_an_edge() {
    // The counterpart to the fix: a flowchart with no header at all is still
    // detected by its edge syntax, and a hyphen inside an id stays an id.
    let parsed = parse_mermaid_strict("alpha-one --> beta-two\n").expect("headerless flowchart");
    let ids: Vec<&str> = parsed.graph.nodes.keys().map(String::as_str).collect();
    assert!(
        ids.contains(&"alpha-one") && ids.contains(&"beta-two"),
        "expected the hyphenated ids to survive, got {ids:?}"
    );
}

#[test]
fn a_lone_hyphen_does_not_make_a_flowchart() {
    // `cynefin-beta` reduced: one word, one hyphen, no diagram.
    assert!(parse_mermaid_strict("some-word\n").is_err());
}

// markdoc-mermaid: the geometry-driven kinds added after the four graph-shaped
// ones, each with its own layout module. Their coordinates come from the
// source, so `validate_layout_invariants` is the thing that holds them to the
// layout they declare.
#[test]
fn the_geometry_kinds_satisfy_the_layout_invariants() {
    let cases: &[(&str, &str)] = &[(
        "wardley",
        "wardley-beta\ntitle Tea\nanchor Business [0.95, 0.63]\ncomponent Kettle [0.43, 0.35]\ncomponent Power [0.10, 0.70]\nBusiness -> Kettle\nKettle -> Power\nevolve Kettle 0.62\nnote \"n\" [0.3, 0.5]\n",
    )];
    let theme = mermaid_rs_renderer::Theme::modern();
    let config = mermaid_rs_renderer::LayoutConfig::default();
    for (name, src) in cases {
        let parsed = parse_mermaid_strict(src).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let layout = mermaid_rs_renderer::compute_layout(&parsed.graph, &theme, &config);
        if let Err(errors) = mermaid_rs_renderer::layout::validate_layout_invariants(&layout) {
            panic!(
                "{name}: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
    }
}
