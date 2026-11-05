//! Mermaid diagrams for Markdoc, with node labels resolved from the
//! document's own anchors.
//!
//! ```no_run
//! use markdoc_mermaid::{Anchor, Anchors, Options, render};
//!
//! struct Headings;
//! impl Anchors for Headings {
//!     fn get(&self, id: &str) -> Option<Anchor> {
//!         (id == "step_two").then(|| Anchor {
//!             label: "Can you ping the switch?".into(),
//!             href: "#step_two".into(),
//!         })
//!     }
//! }
//!
//! let diagram = render(
//!     "flowchart TD\n  step_one --> step_two\n",
//!     &Headings,
//!     &Options::default(),
//! )?;
//! println!("{}", diagram.svg);
//! # Ok::<(), markdoc_mermaid::Error>(())
//! ```
//!
//! # What this crate is for
//!
//! Two things a Mermaid renderer on its own will not do.
//!
//! **Labels that cannot drift.** A diagram drawn beside a document repeats
//! that document's words, and a repeat is free to go stale — rename a
//! heading and the picture keeps the old wording with nothing to warn you.
//! Here an author writes **ids**, and the label is the heading that id sits
//! on, read back out of the caller's index at build time. See [`expand`].
//!
//! **Output a page can own.** A renderer emits a finished picture: white
//! canvas, fixed pixel size, colours written into the markup. A document
//! needs the opposite — something that sizes itself to the column and takes
//! its colours from the page, in dark mode and in print. See [`theme`].
//!
//! # Choice of renderer
//!
//! `mermaid-rs-renderer`, for two reasons that are not about features.
//!
//! It is MIT. `flowmaid` is the better engine by most measures and is
//! **GPL-3.0-or-later**, which it would pass to anything linking it;
//! `merman` is `0.10.0-alpha.1`. Neither is a safe foundation for a
//! proprietary documentation pipeline today.
//!
//! And it emits real `<text>` rather than `<foreignObject>`. That decides
//! whether a diagram printed into a PDF is searchable text or a picture of
//! text, which for a manual read offline is the difference between a working
//! document and a screenshot.
//!
//! The renderer is kept behind this crate's own API on the assumption that
//! it will be replaced. Nothing a caller touches names it.

mod expand;
mod layout;
mod route;
mod theme;

pub use expand::{Expanded, expand as expand_source};
pub use layout::Layout;

/// What a document knows about one anchor id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// The heading the id sits on. Becomes the node's label verbatim.
    pub label: String,
    /// Where clicking the node goes.
    ///
    /// Prefer a bare `#fragment` when the target is on the page being
    /// rendered. On the web the two are identical; in a PDF a fragment
    /// becomes an internal destination the reader jumps to, and an absolute
    /// path becomes an external URI action that a viewer opens a browser for,
    /// or refuses. Only the caller knows which page it is rendering, so only
    /// the caller can make that choice.
    pub href: String,
}

/// A document's anchor index, as this crate needs to see it.
///
/// One method, so the obvious implementation over an existing index is a
/// couple of lines and no data has to be copied in advance.
pub trait Anchors {
    fn get(&self, id: &str) -> Option<Anchor>;
}

/// A function is an index too, which keeps tests and one-offs short.
impl<F> Anchors for F
where
    F: Fn(&str) -> Option<Anchor>,
{
    fn get(&self, id: &str) -> Option<Anchor> {
        self(id)
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    /// Class put on the root `<svg>`, for the page's stylesheet to hook.
    pub class: String,
    /// Emit `click` directives, making each resolved node a link.
    pub link: bool,
    /// Strip the canvas and fixed dimensions, and make the colours
    /// overridable. Off leaves exactly what the renderer produced.
    pub themeable: bool,
    /// How much room the diagram is given. See [`Layout`], whose defaults
    /// suit labels read out of a document rather than typed by hand.
    pub layout: Layout,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            class: "markdoc-mermaid".into(),
            link: true,
            themeable: true,
            layout: Layout::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub svg: String,
    /// Ids that were given a label, in the order the diagram names them.
    ///
    /// Empty is worth a word from the caller: the diagram rendered, but
    /// nothing was resolved, so either it is hand-labelled — fine, and the
    /// no-drift guarantee simply does not apply — or the ids are misspelt and
    /// a reader is about to meet a diagram of identifiers.
    pub resolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Nothing to draw.
    Empty,
    /// The renderer refused the source. Carries its own words, because they
    /// name the line and this crate would only blur them.
    Render(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Empty => write!(f, "the diagram is empty"),
            Error::Render(why) => write!(f, "mermaid could not be rendered: {why}"),
        }
    }
}

impl std::error::Error for Error {}

/// Resolve the ids, render, and hand back an SVG a page can style.
pub fn render(source: &str, anchors: &dyn Anchors, options: &Options) -> Result<Diagram, Error> {
    if source.trim().is_empty() {
        return Err(Error::Empty);
    }
    let expanded = expand::expand(source, anchors, options.link);
    // `render_with_options` is these three steps; they are taken one at a
    // time so the edges can be rerouted between layout and drawing (`route`).
    let parsed = mermaid_rs_renderer::parse_mermaid_strict(&expanded.source)
        .map_err(|e| Error::Render(format!("{e:?}")))?;
    let theme = mermaid_rs_renderer::Theme::modern();
    let config = options.layout.to_config();
    let mut layout = mermaid_rs_renderer::compute_layout(&parsed.graph, &theme, &config);
    if options.layout.tidy_edges && parsed.graph.kind == mermaid_rs_renderer::DiagramKind::Flowchart
    {
        route::tidy(&mut layout, parsed.graph.direction);
    }
    let svg = mermaid_rs_renderer::render_svg(&layout, &theme, &config);
    let svg = if options.themeable {
        theme::themeable(&svg, &options.class)
    } else {
        svg
    };
    Ok(Diagram {
        svg,
        resolved: expanded.resolved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchors(id: &str) -> Option<Anchor> {
        match id {
            "step_one" => Some(Anchor {
                label: "Cannot connect".into(),
                href: "#step_one".into(),
            }),
            "step_two" => Some(Anchor {
                label: "Can you ping the switch?".into(),
                href: "#step_two".into(),
            }),
            _ => None,
        }
    }

    const SRC: &str = "flowchart TD\n  step_one --> step_two\n";

    #[test]
    fn a_diagram_is_labelled_from_the_document() {
        let out = render(SRC, &anchors, &Options::default()).expect("renders");
        assert!(out.svg.contains("Cannot connect"), "{}", out.svg);
        assert!(out.svg.contains("Can you ping the switch?"), "{}", out.svg);
        assert_eq!(out.resolved, vec!["step_one", "step_two"]);
    }

    #[test]
    fn a_node_carries_the_href_the_caller_chose() {
        // A fragment, so a PDF can follow it without leaving the document.
        let out = render(SRC, &anchors, &Options::default()).expect("renders");
        assert!(out.svg.contains("href=\"#step_two\""), "{}", out.svg);
    }

    #[test]
    fn the_output_is_text_rather_than_a_picture_of_text() {
        // The whole reason for this renderer over the alternatives: a
        // foreignObject would make a printed diagram unsearchable.
        let out = render(SRC, &anchors, &Options::default()).expect("renders");
        assert!(out.svg.contains("<text"), "{}", out.svg);
        assert!(!out.svg.contains("foreignObject"), "{}", out.svg);
    }

    #[test]
    fn a_themed_diagram_sizes_itself_and_takes_the_pages_colours() {
        let out = render(SRC, &anchors, &Options::default()).expect("renders");
        let open = &out.svg[..out.svg.find('>').unwrap()];
        assert!(!open.contains("width="), "{open}");
        assert!(open.contains("viewBox"), "{open}");
        assert!(open.contains("class=\"markdoc-mermaid\""), "{open}");
        assert!(out.svg.contains("var(--diagram-"), "{}", out.svg);
    }

    #[test]
    fn theming_can_be_declined() {
        let plain = Options {
            themeable: false,
            ..Options::default()
        };
        let out = render(SRC, &anchors, &plain).expect("renders");
        assert!(!out.svg.contains("var(--diagram-"), "{}", out.svg);
    }

    #[test]
    fn an_unknown_id_renders_as_itself_rather_than_failing() {
        // A diagram naming a step that does not exist is a content mistake,
        // and the reader is better served by seeing which id is wrong than by
        // a missing diagram.
        let out = render(
            "flowchart TD\n  step_one --> ghost_step\n",
            &anchors,
            &Options::default(),
        )
        .expect("renders");
        assert!(out.svg.contains("ghost_step"), "{}", out.svg);
        assert_eq!(out.resolved, vec!["step_one"]);
    }

    #[test]
    fn an_empty_source_is_an_error_not_an_empty_picture() {
        assert_eq!(
            render("   \n", &anchors, &Options::default()).unwrap_err(),
            Error::Empty
        );
    }

    #[test]
    fn a_closure_can_stand_in_for_an_index() {
        let out = render(
            "flowchart TD\n  only_one --> b\n",
            &|id: &str| {
                (id == "only_one").then(|| Anchor {
                    label: "Only one".into(),
                    href: "#only_one".into(),
                })
            },
            &Options::default(),
        )
        .expect("renders");
        assert_eq!(out.resolved, vec!["only_one"]);
    }

    #[test]
    fn the_same_source_renders_the_same_bytes() {
        // An export keyed by content hash should not churn between builds.
        let a = render(SRC, &anchors, &Options::default()).expect("renders");
        let b = render(SRC, &anchors, &Options::default()).expect("renders");
        assert_eq!(a.svg, b.svg);
    }

    /// The real safety-system tree: ten steps, thirteen branches, two of
    /// them pointing back up. It is the graph in this corpus that the
    /// renderer handles least gracefully, which is exactly why it is the one
    /// pinned here.
    const TREE: &str = "flowchart TD\n\
        a --> b\n\
        b -- \"Laser scanner issues\" --> c\n\
        b -- \"Connection issues\" --> e\n\
        c -- \"None of that\" --> d\n\
        d -- \"Marks or warnings\" --> e\n\
        d -- \"Blank, robot on\" --> f\n\
        f -- \"Both off\" --> g\n\
        g -- \"Reported\" --> i\n\
        g -- \"Not reported\" --> h\n\
        h -- \"24 V on all pairs\" --> i\n\
        i -- \"All pairs 0-5 Ohm\" --> e\n\
        i -- \"PLC not connected\" --> j\n\
        j -- \"PLC has no power\" --> g\n";

    fn tree_anchors(id: &str) -> Option<Anchor> {
        // The lengths are what matter: these are headings, not hand-typed
        // node names, and their length is why the defaults need adjusting.
        let label = match id {
            "a" => "Safety system",
            "b" => "What is the issue type?",
            "c" => "Slow driving, stop-and-go-behavior, or detecting non-existent obstacles?",
            "d" => "What does the scanner display show?",
            "e" => "Can you ping the safety laser scanners?",
            "f" => "Is only one scanner off?",
            "g" => "Is IsoPowerX: PGood failed reported at start up?",
            "h" => "Measure voltage from power board",
            "i" => "Measure resistance across power board and safety components",
            "j" => "Is the PLC powered?",
            _ => return None,
        };
        Some(Anchor {
            label: label.into(),
            href: format!("#{id}"),
        })
    }

    #[derive(Clone, Copy)]
    struct Box2 {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    }

    impl Box2 {
        fn covered_by(&self, o: &Box2) -> f32 {
            let x = (self.x + self.w).min(o.x + o.w) - self.x.max(o.x);
            let y = (self.y + self.h).min(o.y + o.h) - self.y.max(o.y);
            if x > 0.0 && y > 0.0 { x * y } else { 0.0 }
        }
    }

    /// How many edge labels are drawn mostly underneath a node box.
    ///
    /// The renderer paints a node body and an edge-label backplate in
    /// different colours, so the two can be told apart without laying out
    /// the diagram again. Themed output would have replaced those colours
    /// with custom properties, so this reads the raw render.
    fn labels_lost_behind_nodes(svg: &str) -> usize {
        fn number(tag: &str, name: &str) -> Option<f32> {
            let needle = format!("{name}=\"");
            let at = tag.find(&needle)? + needle.len();
            let end = tag[at..].find('"')? + at;
            tag[at..end].parse().ok()
        }

        let head = &svg[..svg.find('>').unwrap_or(0)];
        let (width, height) = (
            number(head, "width").unwrap_or(0.0),
            number(head, "height").unwrap_or(0.0),
        );

        let (mut nodes, mut labels) = (Vec::new(), Vec::new());
        for raw in svg.split("<rect").skip(1) {
            let tag = &raw[..raw.find('>').unwrap_or(raw.len())];
            let (Some(x), Some(y), Some(w), Some(h)) = (
                number(tag, "x"),
                number(tag, "y"),
                number(tag, "width"),
                number(tag, "height"),
            ) else {
                continue;
            };
            // The canvas is the same colour as a label backplate, and is
            // told apart by being the size of the whole drawing.
            if x == 0.0 && y == 0.0 && (w - width).abs() < 0.5 && (h - height).abs() < 0.5 {
                continue;
            }
            let rect = Box2 { x, y, w, h };
            let upper = tag.to_ascii_uppercase();
            if upper.contains("FILL=\"#F8FAFC\"") {
                nodes.push(rect);
            } else if upper.contains("FILL=\"#FFFFFF\"") {
                labels.push(rect);
            }
        }

        labels
            .iter()
            .filter(|label| {
                let area = (label.w * label.h).max(1.0);
                let hidden: f32 = nodes.iter().map(|n| label.covered_by(n)).sum();
                hidden / area > 0.40
            })
            .count()
    }

    #[test]
    fn no_branch_condition_is_drawn_underneath_a_box() {
        // The reason Layout::default() overrides the renderer's clearance.
        // On a decision tree the edge labels are the branch conditions, so a
        // label hidden behind a node is not untidiness: it is a reader who
        // cannot tell which way to go.
        let out = render(
            TREE,
            &tree_anchors,
            &Options {
                themeable: false,
                ..Options::default()
            },
        )
        .expect("renders");
        assert_eq!(labels_lost_behind_nodes(&out.svg), 0, "{}", out.svg);
    }

    #[test]
    fn the_renderers_own_clearance_is_what_loses_them() {
        // Pins the premise of the override rather than its value. If a
        // renderer bump fixes this upstream, this test fails and says so,
        // and the override can go.
        let out = render(
            TREE,
            &tree_anchors,
            &Options {
                themeable: false,
                layout: Layout {
                    label_clearance: 0.55,
                    ..Layout::default()
                },
                ..Options::default()
            },
        )
        .expect("renders");
        assert!(
            labels_lost_behind_nodes(&out.svg) > 0,
            "the renderer no longer loses labels at its own default clearance, \
             so markdoc-mermaid's override may have outlived its reason"
        );
    }

    #[test]
    fn spacing_asked_for_is_spacing_given() {
        // Not a tautology: the renderer's adaptive pass treats a configured
        // rank spacing as a ceiling, so without turning that pass off these
        // two would render identically.
        let tall = |rank: f32| {
            let out = render(
                TREE,
                &tree_anchors,
                &Options {
                    themeable: false,
                    layout: Layout {
                        rank_spacing: Some(rank),
                        ..Layout::default()
                    },
                    ..Options::default()
                },
            )
            .expect("renders");
            let head = &out.svg[..out.svg.find('>').unwrap()];
            let at = head.find("height=\"").unwrap() + 8;
            head[at..][..head[at..].find('"').unwrap()]
                .parse::<f32>()
                .unwrap()
        };
        assert!(
            tall(120.0) > tall(60.0) + 1.0,
            "120 gave {} and 60 gave {}",
            tall(120.0),
            tall(60.0)
        );
    }
}
