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
//! An author writes **ids**, and each label is the heading that id sits on,
//! read out of the caller's index at build time. The SVG sizes itself to the
//! column it is drawn in and takes its colours from the page.
//!
//! # Making a diagram interactive
//!
//! [`flowchart`] returns the nodes and edges [`render`] draws, with the
//! labels as drawn, and with the `serde` feature the graph is `Serialize`.
//! The SVG names the same things in merman's terms: `data-id` on each node
//! and each edge, with `data-et` saying which is which. Take an edge's ends
//! from [`Edge::from`] and [`Edge::to`] rather than splitting [`Edge::id`],
//! which may itself contain `_`.
//!
//! A diagram carries its own stylesheet in a `<style>` element, which a
//! search index or a reading-time estimate should skip. Two diagrams in one
//! document must not share [`Options::id`].
//!
//! # What a diagram kind supports
//!
//! Resolution needs a declaration form the diagram's own grammar accepts,
//! and most grammars have none: a mindmap node *is* its text at an indent,
//! and an ER entity is drawn by its id with no alias in the syntax. Where
//! there is no form, nothing is resolved and [`Diagram::resolved`] is empty.
//! [`resolves`] answers for a kind, and `README.md` has the table.
//!
//! # The renderer
//!
//! [`merman`](https://github.com/Latias94/merman), drawn through
//! `SvgPipeline::resvg_safe()`, which emits Mermaid's HTML labels as real
//! `<text>` -- a `<foreignObject>` is not text to a rasteriser.
//! `default-features = false`, because the default set pulls in
//! `merman-elk-layered` (EPL-2.0) through `layout-elk`; a diagram asking for
//! `layout: elk` falls back to dagre.

mod expand;
mod graph;
mod layout;
mod measure;
mod theme;

pub use expand::{Expanded, expand as expand_source, resolves};
pub use graph::{Edge, Flowchart, Node, Subgraph, flowchart};
pub use layout::{Layout, Palette};
pub use measure::{Measure, Text};

/// What a document knows about one anchor id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// The heading the id sits on. Becomes the node's label verbatim.
    pub label: String,
    /// Where clicking the node goes.
    ///
    /// Prefer a bare `#fragment` when the target is on the page being
    /// rendered: in a PDF that becomes an internal destination, where an
    /// absolute path becomes an external URI action.
    pub href: String,
}

/// A document's anchor index, as this crate needs to see it.
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
    /// What to call this diagram in the SVG, where everything scoped lands:
    /// the root's `id`, the marker ids, and the `#…` every CSS rule and
    /// every label reference is scoped to.
    ///
    /// Two diagrams in one HTML document must not share it, or each will
    /// resolve the other's scoped ids. `None` derives one from the source
    /// and these options.
    ///
    /// What you set is not always what you get: merman makes the value safe
    /// to put in a selector, so `"fig 1"` is drawn as `fig-1`. A page that
    /// looks the diagram up should read the id back from the SVG's root.
    pub id: Option<String>,
    /// Measure the labels with the font that will draw them.
    ///
    /// `None` leaves the renderer's own estimate, which assumes a font it
    /// cannot see, and [`Layout::label_slack`] is the allowance for that. A
    /// caller that has the drawing font loaded can measure instead, leave
    /// `label_slack` at 1.0, and set [`Layout::font_family`] to the same
    /// family.
    pub measure: Option<Measure>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            class: "markdoc-mermaid".into(),
            link: true,
            themeable: true,
            layout: Layout::default(),
            id: None,
            measure: None,
        }
    }
}

/// How much room a diagram wants, in the units its own coordinates are in.
///
/// One unit is one CSS pixel to a browser. Nothing here is a pixel count:
/// an SVG has no resolution, and the numbers say the shape and the scale the
/// renderer laid out at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Diagram {
    pub svg: String,
    /// Ids that were given a label, in the order the diagram names them. An
    /// id named here reached the picture as its heading.
    ///
    /// Empty means the diagram rendered with none: it is hand-labelled, the
    /// ids are misspelt, or the kind cannot carry a label at all
    /// ([`resolves`]).
    pub resolved: Vec<String>,
    /// What the renderer laid the diagram out to need, from its `viewBox`:
    /// the extent, not the origin.
    ///
    /// A page needs it to reserve room before the SVG loads, or to give a
    /// container the diagram's aspect ratio. `None` only if the root carries
    /// no `viewBox` this can read.
    pub size: Option<Size>,
    /// The size, in the same units, that this diagram sets for text with no
    /// size of its own.
    ///
    /// It is the document's default, not every label's size: the renderer
    /// gives several roles their own. Use it to size a line of a caller's
    /// own text to match the diagram's -- an outline beside it, a caption
    /// under it.
    ///
    /// `None` where the diagram sets no default and every size is set
    /// element by element. See [`Self::min_font_size`] for those.
    pub font_size: Option<f32>,
    /// The smallest size, in the same units, that the diagram asks for
    /// anywhere -- a floor under every word it draws.
    ///
    /// It is a floor, not a measurement: the smallest size the stylesheet or
    /// the elements *declare*, so the smallest text drawn may be larger, and
    /// is never smaller. A caller scaling a diagram down to fit can hold
    /// this above a legibility limit and know every label cleared it.
    ///
    /// `None` only where the diagram names no size anywhere.
    pub min_font_size: Option<f32>,
}

/// The smallest `font-size` the document names, wherever it names one.
///
/// Every form merman writes: a length in a rule or an inlined style, an
/// attribute in user units, and the relative `em`, `ex` and percentage
/// forms. A relative size is resolved against the smallest absolute one,
/// which keeps the answer a floor, with `1ex` taken as half an em. A rule
/// that cannot match anything here is left out; see [`drawable`].
fn min_font_size(svg: &str) -> Option<f32> {
    let mut absolute: Option<f32> = None;
    let mut relative: Option<f32> = None;
    for rest in drawable(svg).flat_map(|part| part.split("font-size").skip(1)) {
        let rest = rest.trim_start_matches([':', '=', '"', '\'', ' ']);
        let digits: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let Some(size) = digits.parse::<f32>().ok().filter(|size| *size > 0.0) else {
            continue;
        };
        let unit: String = rest[digits.len()..]
            .chars()
            .take_while(|c| c.is_ascii_alphabetic() || *c == '%')
            .collect();
        // A factor of the parent's size, or a size of its own.
        let factor = match unit.as_str() {
            "em" | "rem" => Some(size),
            "ex" => Some(size / 2.0),
            "%" => Some(size / 100.0),
            // `px` or nothing: a length. Anything else is a unit merman does
            // not write, and guessing at it would not be a floor.
            "px" | "" => None,
            _ => continue,
        };
        let slot = if factor.is_some() {
            &mut relative
        } else {
            &mut absolute
        };
        let value = factor.unwrap_or(size);
        *slot = Some(slot.map_or(value, |least: f32| least.min(value)));
    }
    let absolute = absolute?;
    Some(match relative {
        Some(factor) => absolute.min(absolute * factor),
        None => absolute,
    })
}

/// The parts of the document that can decide how large a word is drawn:
/// everything outside the stylesheet, and the stylesheet's rules bar those
/// whose selectors can match nothing in an SVG.
///
/// merman's flowchart, swimlane and journey stylesheets style the HTML `div`
/// Mermaid.js appends to the page for a tooltip, which this crate never
/// draws. A rule is skipped only when *every* selector in its list is one of
/// those, and anything this cannot parse as a rule is handed back whole:
/// more text can only make the answer smaller, and smaller is still a floor.
fn drawable(svg: &str) -> impl Iterator<Item = &str> {
    // One `<style>` element holding no at-rule, so a rule is a selector
    // list up to `{` and a body up to `}`, with no nesting to track.
    let sheet = match (svg.find("<style"), svg.find("</style>")) {
        (Some(open), Some(close)) if close > open => {
            svg[open..close].find('>').map(|at| (open + at + 1, close))
        }
        _ => None,
    };
    let (outside, mut rules) = match sheet {
        Some((open, close)) => ([&svg[..open], &svg[close..]], &svg[open..close]),
        None => ([svg, ""], ""),
    };
    let sheet = std::iter::from_fn(move || {
        let brace = rules.find('{')?;
        let (selectors, body) = (&rules[..brace], &rules[brace + 1..]);
        let end = body.find('}').unwrap_or(body.len());
        let (body, rest) = (&body[..end], &body[(end + 1).min(body.len())..]);
        rules = rest;
        Some((selectors, body))
    })
    .filter_map(|(selectors, body)| {
        let html_only = selectors.split(',').all(names_an_html_element);
        (!html_only).then_some(body)
    });
    outside.into_iter().chain(sheet)
}

/// Whether a selector names an HTML element as the type it matches: `div`
/// is the only one merman's stylesheets name, so `div.mermaidTooltip` is one
/// and `.divider` is not.
fn names_an_html_element(selector: &str) -> bool {
    let part = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    selector.match_indices("div").any(|(at, word)| {
        selector[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !(part(c) || c == '.' || c == '#'))
            && selector[at + word.len()..]
                .chars()
                .next()
                .is_none_or(|c| !part(c))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Nothing to draw.
    Empty,
    /// The renderer refused the source, in its own words.
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

/// merman, configured the one way this crate uses it: `strict()` parsing, so
/// a header it does not draw is an error rather than a guess, and the site
/// config from `Layout`.
pub(crate) fn engine(layout: &Layout, themeable: bool, kind: &str, seed: u32) -> merman::Renderer {
    merman::Renderer::new()
        .with_presentation_engine(merman::MermaidEngine::new().with_site_config(
            merman::SourcePresentationConfig::from_value(
                layout.to_site_config(themeable, kind, seed),
            ),
        ))
        .with_parse_options(merman::ParseOptions::strict())
}

/// The first line of the source, for an error message to name: merman's own
/// words say no type was detected, without saying which diagram.
fn header_of(source: &str) -> String {
    source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(60).collect())
        .unwrap_or_default()
}

/// The source as merman reads it, semantically: the diagram type and its
/// model, before anything is appended.
pub(crate) fn parse(
    renderer: &merman::Renderer,
    source: &str,
) -> Result<merman::SemanticArtifact, Error> {
    let request = merman::RenderRequest::semantic(source, merman::OperationControl::new());
    match renderer.render(request) {
        Ok(merman::RenderOutput::Semantic(Some(artifact))) => Ok(artifact),
        Ok(_) => Err(Error::Render(format!(
            "no semantic model for {:?}",
            header_of(source)
        ))),
        Err(e) => Err(Error::Render(format!("{:?}: {e}", header_of(source)))),
    }
}

/// Each node's id with the label it already carries, which `expand` needs to
/// leave an author's own label alone and to name every node of a kind that
/// must name them all.
///
/// One arm per kind `expand::dialect` admits, so the two lists stay in step.
pub(crate) fn labelled_nodes(model: &merman::model::SemanticDiagramModel) -> Vec<(String, String)> {
    use merman::model::SemanticDiagramModel as M;
    match model {
        M::Flowchart(d) => d
            .nodes
            .iter()
            .map(|n| {
                (
                    n.id.clone(),
                    n.label.clone().unwrap_or_else(|| n.id.clone()),
                )
            })
            .collect(),
        M::Usecase(d) => d
            .nodes
            .iter()
            .map(|n| (n.id.clone(), n.label.clone()))
            .collect(),
        M::Block(d) => d
            .blocks_flat
            .iter()
            .map(|n| (n.id.clone(), n.label.clone()))
            .collect(),
        M::Class(d) => d
            .classes
            .iter()
            .map(|(id, n)| (id.clone(), n.label.clone()))
            .collect(),
        // A state's label is Mermaid's own JSON, a string where there is one;
        // `description` is where a multi-line one lands.
        M::State(d) => d
            .nodes
            .iter()
            .map(|n| {
                let label = n
                    .label
                    .as_ref()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .or_else(|| n.description.as_ref().map(|lines| lines.join("\n")))
                    .unwrap_or_else(|| n.id.clone());
                (n.id.clone(), label)
            })
            .collect(),
        // `actor_order` rather than the map, so the list is the source's.
        M::Sequence(d) => d
            .actor_order
            .iter()
            .filter_map(|name| {
                d.actors
                    .get(name)
                    .map(|actor| (name.clone(), actor.description.clone()))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// An id for a diagram the caller did not name: short, stable, and derived
/// from what is drawn, so two different diagrams on a page differ.
///
/// The options go in as well as the source, because one source rendered two
/// ways is two different pictures.
fn diagram_id(source: &str, options: &Options) -> String {
    let mut parts = vec![source.as_bytes().to_vec()];
    // Everything that changes the drawing. A measurer is a closure, so only
    // its presence can be told -- two different measurers on one source in
    // one document still collide, and that caller sets `Options::id`.
    parts.push(
        format!(
            "\u{1f}{:?}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            options.layout,
            options.class,
            options.link,
            options.themeable,
            options.measure.is_some(),
        )
        .into_bytes(),
    );
    // Leading letter: an id has to be usable in a CSS selector.
    format!("d{:016x}", fnv1a(&parts))
}

/// FNV-1a over `parts`, 64-bit.
///
/// Written out rather than taken from the standard library, because both
/// callers need the same number next year and `DefaultHasher` is not stable
/// between Rust releases.
fn fnv1a(parts: &[Vec<u8>]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in parts.iter().flatten() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// A seed for the one kind that draws from one: a cynefin diagram's
/// boundaries wobble, and merman seeds that from the diagram id whenever the
/// configuration names no seed of its own.
///
/// The source alone decides it, so the curves follow what was written rather
/// than what else was asked for -- and only the bytes that decide the
/// drawing: line ends are read as `\n`, trailing whitespace is dropped, and
/// a byte-order mark and the blank lines at either end go. An indent or a
/// comment *is* an edit, and redraws the curves.
///
/// Folded to 32 bits, as Mermaid's own key is read with ECMAScript's
/// `ToUint32`.
fn wobble_seed(source: &str) -> u32 {
    let mut written = String::with_capacity(source.len());
    // `lines` drops a `\r` before the `\n`, so CRLF and LF arrive the same.
    for line in source.trim_start_matches('\u{feff}').lines() {
        written.push_str(line.trim_end());
        written.push('\n');
    }
    // A blank line at either end is a line of nothing by now.
    let hash = fnv1a(&[written.trim_matches('\n').as_bytes().to_vec()]);
    ((hash >> 32) ^ hash) as u32
}

/// Resolve the ids, render, and hand back an SVG a page can style.
pub fn render(source: &str, anchors: &dyn Anchors, options: &Options) -> Result<Diagram, Error> {
    if source.trim().is_empty() {
        return Err(Error::Empty);
    }
    let probe_engine = engine(&options.layout, false, "", wobble_seed(source));
    // Parsed first, because how a label can be declared depends on the kind
    // and on the nodes it already has (`expand`).
    let probe = parse(&probe_engine, source)?;
    let nodes = labelled_nodes(probe.model());
    let expanded = expand::expand(source, anchors, options.link, probe.diagram_type(), &nodes);
    // Nothing resolved means nothing was appended, so the first parse stands.
    let drawn = if expanded.resolved.is_empty() {
        source.to_string()
    } else {
        expanded.source.clone()
    };

    // The seed comes from the source as drawn, which for a kind that takes
    // no declarations is the source itself.
    let renderer = engine(
        &options.layout,
        options.themeable,
        probe.diagram_type(),
        wobble_seed(&drawn),
    );

    // `resvg_safe` is what turns Mermaid's HTML labels into real `<text>`.
    let mut request = merman::SvgRequest {
        pipeline: Some(merman::svg::SvgPipeline::resvg_safe()),
        ..Default::default()
    };
    request.options.diagram_id = Some(match &options.id {
        Some(id) => id.clone(),
        None => diagram_id(&drawn, options),
    });
    // A caller's own measurement, or an allowance on the renderer's.
    // Neither is installed where the caller asked for neither.
    if let Some(measure) = &options.measure {
        request.environment = merman::SvgEnvironment::deterministic()
            .with_text_measurement_policy(measure::policy(measure));
    } else if options.layout.label_slack != 1.0 {
        request.environment = merman::SvgEnvironment::deterministic()
            .with_text_measurement_policy(measure::slack_policy(options.layout.label_slack));
    }
    let svg = match renderer.render(merman::RenderRequest::svg(
        &drawn,
        merman::OperationControl::new(),
        request,
    )) {
        Ok(merman::RenderOutput::Svg(Some(out))) => out.svg().to_string(),
        Ok(_) => {
            return Err(Error::Render(format!(
                "nothing drawn for {:?}",
                header_of(&drawn)
            )));
        }
        Err(e) => {
            let why = e.to_string();
            // merman's own words name a limit and two numbers, which say
            // nothing about what to do about it.
            if why.contains("svg_fallback_selector_index") {
                return Err(Error::Render(format!(
                    "{:?}: the diagram is too big for the renderer's label \
                     pass, which has a fixed ceiling it reaches at roughly \
                     230 boxes and no setting to raise. Split it into \
                     several diagrams. ({why})",
                    header_of(&drawn)
                )));
            }
            return Err(Error::Render(format!("{:?}: {why}", header_of(&drawn))));
        }
    };

    // Whether or not it is themed: one structure for a page and for a PDF.
    let svg = theme::decode_entity_placeholders(&svg);
    let svg = theme::inline_label_references(&svg);
    let svg = match options.layout.corner_radius {
        Some(radius) => theme::round_boxes(&svg, radius),
        None => svg,
    };
    let svg = match options.layout.line_width {
        // The line and the head a link ends in, weighed together.
        Some(width) => theme::scale_heads(&theme::weigh_lines(&svg, width), width),
        None => svg,
    };
    let svg = if options.themeable {
        theme::themeable(&svg, &options.class)
    } else {
        svg
    };
    Ok(Diagram {
        size: natural_size(&svg),
        font_size: default_font_size(&svg),
        min_font_size: min_font_size(&svg),
        svg,
        resolved: expanded.resolved,
    })
}

/// One attribute of the root `<svg>` element.
///
/// Anchored to that element rather than taken as the document's first
/// `viewBox=` or `id=`, both of which appear again further down. The scan
/// stops at the end of the start tag.
fn root_attribute<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
    let open = svg.find("<svg")?;
    let tag = &svg[open..][..svg[open..].find('>')?];
    let at = tag.find(&format!("{name}=\""))?;
    let value = &tag[at + name.len() + 2..];
    Some(&value[..value.find('"')?])
}

/// The extent of the root's `viewBox`, which is what the diagram was laid
/// out to need. The first two numbers are the origin and are skipped, both
/// separators SVG allows are read, and a zero or negative extent is none.
fn natural_size(svg: &str) -> Option<Size> {
    let value = root_attribute(svg, "viewBox")?;
    let mut numbers = value
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .skip(2);
    let width: f32 = numbers.next()?.parse().ok()?;
    let height: f32 = numbers.next()?.parse().ok()?;
    (width > 0.0 && height > 0.0).then_some(Size { width, height })
}

/// The `font-size` the document's own rule sets, in units.
///
/// Read from the rule whose selector is the diagram's id alone, rather than
/// from the first rule in the stylesheet, so the order merman writes them in
/// does not matter. The id comes from the root element, since merman makes
/// the value asked for safe to put in a selector.
fn default_font_size(svg: &str) -> Option<f32> {
    let id = root_attribute(svg, "id")?;
    let rule = format!("#{id}{{");
    let at = svg.find(&rule)?;
    let body = &svg[at + rule.len()..];
    let body = &body[..body.find('}')?];
    body.split("font-size:")
        .nth(1)?
        .split(';')
        .next()?
        .trim()
        .strip_suffix("px")?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    /// The root's attributes, not the document's first: a label definition
    /// carries an id, and a nested `<svg>` a `viewBox`.
    #[test]
    fn an_attribute_is_read_from_the_root_element_alone() {
        let svg = concat!(
            "<!-- id=\"a comment\" viewBox=\"0 0 1 1\" -->",
            "<svg id=\"root\" class=\"x\" viewBox=\"-50 -10 450 217\">",
            "<defs><g id=\"label\"><svg viewBox=\"0 0 9 9\"><text>x</text></svg></g></defs>",
            "</svg>"
        );
        assert_eq!(super::root_attribute(svg, "id"), Some("root"));
        assert_eq!(
            super::root_attribute(svg, "viewBox"),
            Some("-50 -10 450 217")
        );
        assert_eq!(super::root_attribute(svg, "stroke"), None);
        // And the extent skips the origin.
        assert_eq!(
            super::natural_size(svg),
            Some(super::Size {
                width: 450.0,
                height: 217.0
            })
        );
    }

    /// The floor comes from the rules that can draw something: merman styles
    /// Mermaid.js's run-time tooltip, an HTML `div` the SVG never holds.
    #[test]
    fn a_rule_for_something_the_svg_cannot_hold_sets_no_floor() {
        let sheet =
            |rules: &str| format!("<svg id=\"d\"><style>{rules}</style><text>x</text></svg>");

        let drawn = sheet("#d{font-size:16px;}#d div.mermaidTooltip{font-size:12px;}");
        assert_eq!(super::min_font_size(&drawn), Some(16.0));

        // Not the word anywhere in a selector: only the element it names.
        for selector in [".divider text", "#divider", ".node div-text"] {
            let svg = sheet(&format!(
                "#d{{font-size:16px;}}#d {selector}{{font-size:12px;}}"
            ));
            assert_eq!(
                super::min_font_size(&svg),
                Some(12.0),
                "{selector} was read as an HTML element"
            );
        }

        // A list that also names something drawable keeps its size.
        let both = sheet("#d{font-size:16px;}#d div.mermaidTooltip,#d .edgeLabel{font-size:12px;}");
        assert_eq!(super::min_font_size(&both), Some(12.0));

        // Sizes outside the stylesheet are the element's own and always count.
        let inline = "<svg id=\"d\"><style>#d div.x{font-size:12px;}</style>\
                      <text font-size=\"9\">x</text></svg>";
        assert_eq!(super::min_font_size(inline), Some(9.0));
    }

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
        // A `<foreignObject>` would make a printed diagram unsearchable.
        let out = render(SRC, &anchors, &Options::default()).expect("renders");
        assert!(out.svg.contains("<text"), "{}", out.svg);
        assert!(!out.svg.contains("foreignObject"), "{}", out.svg);
    }

    #[test]
    fn a_themed_diagram_sizes_itself_and_takes_the_pages_colours() {
        let out = render(SRC, &anchors, &Options::default()).expect("renders");
        let open = &out.svg[..out.svg.find('>').unwrap()];
        assert!(!open.contains("width="), "{open}");
        assert!(!open.contains("max-width"), "{open}");
        assert!(open.contains("viewBox"), "{open}");
        // Merged into the root's own class, not a second attribute beside it.
        assert!(open.contains("markdoc-mermaid"), "{open}");
        assert_eq!(open.matches("class=").count(), 1, "{open}");
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
        // A diagram naming a step that does not exist still draws, with the
        // id as its label.
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

    /// And the one kind that draws from a seed follows the source alone,
    /// not the options the derived id also carries.
    #[test]
    fn a_wobble_follows_the_source_and_not_the_options() {
        const CYNEFIN: &str = "cynefin-beta\ncomplex\ncomplicated\n";
        let boundaries = |svg: &str| -> Vec<String> {
            svg.split("<path")
                .skip(1)
                .filter_map(|tag| tag.split('>').next())
                .filter(|tag| tag.contains("cynefinBoundary"))
                .map(str::to_string)
                .collect()
        };
        let drawn = |options: Options| {
            let svg = render(CYNEFIN, &anchors, &options).expect("renders").svg;
            let curves = boundaries(&svg);
            assert!(!curves.is_empty(), "no boundary was drawn: {svg}");
            curves
        };

        let plain = drawn(Options {
            themeable: false,
            ..Options::default()
        });
        // An id of the caller's, which the seed used to come from.
        assert_eq!(
            plain,
            drawn(Options {
                themeable: false,
                id: Some("fig-1".into()),
                ..Options::default()
            })
        );
        // And options that have nothing to do with a boundary.
        assert_eq!(
            plain,
            drawn(Options {
                themeable: false,
                layout: Layout {
                    corner_radius: Some(3.0),
                    wrapping_width: Some(260.0),
                    ..Layout::default()
                },
                ..Options::default()
            })
        );
        // A different diagram still draws differently.
        let other = render(
            "cynefin-beta\nclear\nchaotic\n",
            &anchors,
            &Options {
                themeable: false,
                ..Options::default()
            },
        )
        .expect("renders")
        .svg;
        assert_ne!(plain, boundaries(&other));
    }

    /// The same diagram written two ways wobbles the same way: a line end,
    /// a byte-order mark and trailing whitespace are nobody's decision,
    /// where a comment or an indent is an edit.
    #[test]
    fn the_same_diagram_spelled_two_ways_draws_the_same_curves() {
        const SOURCE: &str = "cynefin-beta\ncomplex\ncomplicated\n";
        let seed = wobble_seed(SOURCE);
        for same in [
            SOURCE.replace('\n', "\r\n"),
            SOURCE.trim_end().to_string(),
            format!("{SOURCE}\n\n"),
            SOURCE.replace('\n', "   \n"),
            SOURCE.replace('\n', "\t\r\n"),
            // A blank line before the header, and a byte-order mark.
            format!("\n{SOURCE}"),
            format!("  \n\n{SOURCE}"),
            format!("\u{feff}{SOURCE}"),
            format!("\u{feff}\r\n{SOURCE}"),
        ] {
            assert_eq!(wobble_seed(&same), seed, "{same:?}");
        }
        for edited in [
            "cynefin-beta\n%% a note\ncomplex\ncomplicated\n",
            "cynefin-beta\n  complex\n  complicated\n",
            "cynefin-beta\ncomplex\nchaotic\n",
        ] {
            assert_ne!(wobble_seed(edited), seed, "{edited:?}");
        }
    }

    /// A real decision tree: ten steps, thirteen branches, two of them
    /// pointing back up.
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
        // node names.
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

    /// How many edge labels are drawn mostly underneath a node box, told
    /// apart by the colours the renderer paints each in.
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
            // The canvas is the same colour, and is the size of the whole
            // drawing.
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
        // Why `Layout::default()` overrides the renderer's clearance: on a
        // decision tree the edge labels are the branch conditions.
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
    fn spacing_asked_for_is_spacing_given() {
        // The height comes from the viewBox: merman's root carries no
        // height attribute at all.
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
            let head = &out.svg[..out.svg.find('>').expect("an open tag")];
            let at = head.find("viewBox=\"").expect("a viewBox") + 9;
            let box_ = &head[at..][..head[at..].find('"').expect("a closing quote")];
            box_.split_whitespace()
                .nth(3)
                .expect("four viewBox numbers")
                .parse::<f32>()
                .expect("a number")
        };
        assert!(
            tall(120.0) > tall(60.0) + 1.0,
            "120 gave {} and 60 gave {}",
            tall(120.0),
            tall(60.0)
        );
    }
}
