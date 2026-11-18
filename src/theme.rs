//! Make the renderer's SVG fit a page it knows nothing about.
//!
//! merman pins the drawing with `width="100%"` *and* a `max-width` style,
//! and some kinds draw a full-bleed white canvas behind it. Both go, leaving
//! the `viewBox` for CSS to size. A class asked for is merged into the
//! root's own, since two `class` attributes are not valid markup. The
//! colours are dealt with before it draws, through `themeVariables`
//! ([`crate::Layout`]).
//!
/// The paints `SvgPipeline::resvg_safe()` writes out as literals, whatever
/// `themeVariables` asked for, turned back into the page's own.
///
/// The pipeline inlines each label's *computed* colour, and a `var(--…)` has
/// none it can resolve, so it writes the theme's own instead: the label text
/// and the plate behind an edge label. The label paint is taken on the
/// elements carrying merman's fallback-text class, rather than from every
/// `#333` in the file.
fn tokenise_inlined_paints(body: &str) -> String {
    const FALLBACK_TEXT: &str = "merman-foreignobject-fallback-text";
    // The edge-label plate, in both the forms merman writes it.
    let mut out = body
        .replace("rgba(232,232,232, 0.8)", "var(--diagram-surface)")
        .replace("rgba(232, 232, 232, 0.5)", "var(--diagram-surface)");

    // What `themeVariables` cannot reach, rule by rule.
    //
    // Each one is a pair, and both halves move together: tokenising text
    // that sits on a surface Mermaid still paints itself loses contrast
    // rather than gaining it. So gantt's bars, a mindmap's sections, a pie
    // slice and a yellow note keep Mermaid's own matched colours.
    for (from, to) in [
        // The document's inherited default, for a kind with no rule of its
        // own. It sits on the page.
        (
            "font-size:16px;fill:#333;",
            "font-size:16px;fill:var(--diagram-text);",
        ),
        (
            "font-size:16px; fill: black;",
            "font-size:16px; fill: var(--diagram-text);",
        ),
        // An edge label, on a plate already tokenised above.
        (
            ".label text{fill:#333;}",
            ".label text{fill:var(--diagram-text);}",
        ),
        // A packet diagram's block is a light grey of its own, so the block
        // goes with the text.
        (
            ".packetByte.start{fill:black;}",
            ".packetByte.start{fill:var(--diagram-text);}",
        ),
        (
            ".packetByte.end{fill:black;}",
            ".packetByte.end{fill:var(--diagram-text);}",
        ),
        (
            ".packetLabel{fill:black;",
            ".packetLabel{fill:var(--diagram-text);",
        ),
        (
            ".packetTitle{fill:black;",
            ".packetTitle{fill:var(--diagram-text);",
        ),
        (
            ".packetBlock{stroke:black;stroke-width:1;fill:#efefef;}",
            ".packetBlock{stroke:var(--diagram-border);stroke-width:1;fill:var(--diagram-node);}",
        ),
        // A use case diagram paints its text, stereotypes and JSON titles
        // this one dark, and every shape under them is tokenised above.
        (
            "fill:#131300;color:#131300",
            "fill:var(--diagram-text);color:var(--diagram-text)",
        ),
        // A tree view draws no boxes, so its labels and lines are both on
        // the page.
        (
            ".treeView-node-label{fill:black;",
            ".treeView-node-label{fill:var(--diagram-text);",
        ),
        (
            ".treeView-node-line { stroke: black;}",
            ".treeView-node-line { stroke: var(--diagram-line);}",
        ),
    ] {
        out = out.replace(from, to);
    }

    // The label paint, element by element, so nothing else named `#333` moves.
    let mut rebuilt = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(at) = rest.find("<text") {
        let (before, from) = rest.split_at(at);
        rebuilt.push_str(before);
        let end = from.find('>').map_or(from.len(), |i| i + 1);
        let (tag, after) = from.split_at(end);
        // A mindmap's sections and a note keep Mermaid's own colours, so a
        // label drawn on one keeps Mermaid's too. merman says which is which
        // in `data-merman-source-classes`.
        let on_mermaids_own_surface = tag.contains("mindmapDiagram") || tag.contains("noteLabel");
        if tag.contains(FALLBACK_TEXT) && !on_mermaids_own_surface {
            // Every dark a label paints itself with, whichever kind drew it.
            let mut tag = tag.to_string();
            for dark in ["#333", "#131300", "black"] {
                tag = tag
                    .replace(&format!("fill=\"{dark}\""), "fill=\"var(--diagram-text)\"")
                    .replace(&format!("fill:{dark};"), "fill:var(--diagram-text);");
            }
            rebuilt.push_str(&tag);
        } else {
            rebuilt.push_str(tag);
        }
        rest = after;
    }
    rebuilt.push_str(rest);
    out = rebuilt;
    out
}

/// Rework a rendered SVG so a document can style and size it.
pub fn themeable(svg: &str, class: &str) -> String {
    let Some(open_end) = svg.find('>') else {
        return svg.to_string();
    };
    let (open, rest) = svg.split_at(open_end + 1);

    let width = attribute(open, "width");
    let height = attribute(open, "height");

    let mut head = open.to_string();
    for name in ["width", "height"] {
        head = remove_attribute(&head, name);
    }
    // Only the `max-width` declaration goes; anything else in the style is
    // the renderer's and stays.
    if let Some(style) = attribute(&head, "style") {
        let kept: Vec<&str> = style
            .split(';')
            .map(str::trim)
            .filter(|rule| !rule.is_empty() && !rule.starts_with("max-width"))
            .collect();
        head = if kept.is_empty() {
            remove_attribute(&head, "style")
        } else {
            head.replace(
                &format!("style=\"{style}\""),
                &format!("style=\"{}\"", kept.join(";")),
            )
        };
    }
    if !class.is_empty() {
        head = match attribute(&head, "class") {
            // merman's root already carries one, so join rather than repeat.
            Some(existing) => head.replace(
                &format!("class=\"{existing}\""),
                &format!("class=\"{existing} {class}\""),
            ),
            None => match head.strip_suffix('>') {
                Some(body) => format!("{body} class=\"{class}\">"),
                None => head,
            },
        };
    }

    let mut body = rest.to_string();
    if let (Some(w), Some(h)) = (width, height) {
        body = remove_canvas(&body, &w, &h);
    }
    body = tokenise_inlined_paints(&body);
    format!("{head}{body}")
}

fn attribute(open: &str, name: &str) -> Option<String> {
    let needle = format!(" {name}=\"");
    let at = open.find(&needle)? + needle.len();
    let end = open[at..].find('"')? + at;
    Some(open[at..end].to_string())
}

fn remove_attribute(open: &str, name: &str) -> String {
    let needle = format!(" {name}=\"");
    let Some(at) = open.find(&needle) else {
        return open.to_string();
    };
    let Some(end) = open[at + needle.len()..].find('"') else {
        return open.to_string();
    };
    let end = at + needle.len() + end + 1;
    format!("{}{}", &open[..at], &open[end..])
}

/// Drop the full-bleed background rect, and only that one.
///
/// Matched on geometry rather than on colour or order: an edge-label
/// backplate is the same colour, and its position depends on the diagram.
fn remove_canvas(body: &str, width: &str, height: &str) -> String {
    let canvas = format!("<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\"");
    let Some(at) = body.find(&canvas) else {
        return body.to_string();
    };
    let Some(end) = body[at..].find("/>") else {
        return body.to_string();
    };
    format!("{}{}", &body[..at], &body[at + end + 2..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="550.78" height="332.72" viewBox="0 0 550.78 332.72">"#,
        r##"<rect x="0" y="0" width="550.78" height="332.72" fill="#FFFFFF"/>"##,
        r##"<rect x="19" y="190" width="138" height="28" fill="#FFFFFF" stroke="#94A3B8"/>"##,
        r##"<path d="M 0 0" stroke="#64748B"/><text fill="#0F172A">Hi</text>"##,
        r##"<rect x="5" y="5" width="10" height="10" fill="#F8FAFC"/></svg>"##
    );

    #[test]
    fn the_canvas_goes_and_the_label_backplate_stays() {
        // Both are #FFFFFF, so keying on colour would strike the text out
        // of every edge label.
        let out = themeable(SVG, "");
        assert!(
            !out.contains(r#"<rect x="0" y="0" width="550.78""#),
            "{out}"
        );
        assert!(out.contains(r#"<rect x="19" y="190""#), "{out}");
    }

    #[test]
    fn the_root_keeps_its_viewbox_and_loses_its_dimensions() {
        let out = themeable(SVG, "");
        let open = &out[..out.find('>').unwrap()];
        assert!(!open.contains("width="), "{open}");
        assert!(!open.contains("height="), "{open}");
        assert!(open.contains(r#"viewBox="0 0 550.78 332.72""#), "{open}");
    }

    #[test]
    fn a_class_can_be_put_on_the_root() {
        let out = themeable(SVG, "markdoc-mermaid");
        assert!(out.starts_with("<svg "), "{out}");
        assert!(
            out[..out.find('>').unwrap()].contains(r#"class="markdoc-mermaid""#),
            "{out}"
        );
    }

    #[test]
    fn an_svg_without_dimensions_survives() {
        // Then there is no canvas rect to find either, and nothing to undo.
        let svg = r##"<svg viewBox="0 0 10 10"><text>x</text></svg>"##;
        let out = themeable(svg, "d");
        assert!(out.contains(r#"class="d""#), "{out}");
        assert!(out.contains("viewBox"), "{out}");
    }

    #[test]
    fn something_that_is_not_svg_is_returned_unharmed() {
        assert_eq!(themeable("not markup", "d"), "not markup");
    }
}

#[cfg(test)]
mod against_merman {
    use crate::{Anchor, Anchors, Options, render};

    struct None_;
    impl Anchors for None_ {
        fn get(&self, _: &str) -> Option<Anchor> {
            None
        }
    }

    fn open_tag() -> String {
        let out = render(
            "flowchart TD\n  a[One] --> b[Two]\n",
            &None_,
            &Options::default(),
        )
        .expect("renders");
        out.svg[..out.svg.find('>').expect("an open tag")].to_string()
    }

    #[test]
    fn the_class_is_merged_into_the_roots_own_rather_than_repeated() {
        // merman puts `class="flowchart"` on the root, and a parser given
        // two `class` attributes keeps whichever it likes.
        let open = open_tag();
        assert_eq!(
            open.matches("class=").count(),
            1,
            "exactly one class attribute: {open}"
        );
        assert!(
            open.contains("flowchart"),
            "merman's own class is kept: {open}"
        );
        assert!(
            open.contains("markdoc-mermaid"),
            "this crate's class is there too: {open}"
        );
    }

    #[test]
    fn nothing_is_left_pinning_the_drawing_to_one_size() {
        // Pinned twice over, so removing the attribute alone leaves it
        // fixed.
        let open = open_tag();
        assert!(!open.contains("width="), "no width attribute: {open}");
        assert!(!open.contains("max-width"), "no max-width style: {open}");
        assert!(
            open.contains("viewBox"),
            "the viewBox is what sizes it: {open}"
        );
    }
}

/// Draw each label where it is used, instead of referring to it.
///
/// `SvgPipeline::resvg_safe()` puts every label in `<defs>` and draws it
/// with `<use href="#…">`. A page that sanitises the SVG against an element
/// allowlist drops the `<use>` and loses every label silently, and those ids
/// are most of what two diagrams in one document would collide over. So the
/// reference is replaced by the thing referred to, and the `<defs>` entry
/// goes.
///
/// `<use x y>` translates what it draws, so those become a
/// `<g transform="translate(x,y)">`. A `<use>` this cannot read is left
/// alone *along with its definition*.
pub(crate) fn inline_label_references(svg: &str) -> String {
    const MARK: &str = "-foreignobject-label-";
    if !svg.contains(MARK) {
        return svg.to_string();
    }

    // Every definition, by the id a `<use>` names it with, and the span to
    // drop once something has been drawn in its place.
    let mut definitions: Vec<(&str, &str, std::ops::Range<usize>)> = Vec::new();
    let mut at = 0usize;
    while let Some(found) = svg[at..].find("<g id=\"") {
        let open = at + found;
        let id_start = open + 7;
        let Some(quote) = svg[id_start..].find('"') else {
            break;
        };
        let id = &svg[id_start..id_start + quote];
        if !id.contains(MARK) {
            at = id_start;
            continue;
        }
        let Some(open_end) = svg[id_start + quote..].find('>') else {
            break;
        };
        let body_start = id_start + quote + open_end + 1;
        let Some(len) = group_body_len(&svg[body_start..]) else {
            break;
        };
        let group = open..body_start + len + 4;
        // A `<defs>` holding nothing but this group goes with it.
        let defs = "<defs>";
        let drop = if svg[..group.start].ends_with(defs) && svg[group.end..].starts_with("</defs>")
        {
            group.start - defs.len()..group.end + "</defs>".len()
        } else {
            group.clone()
        };
        definitions.push((id, &svg[body_start..body_start + len], drop));
        at = group.end;
    }
    if definitions.is_empty() {
        return svg.to_string();
    }

    // What each `<use>` becomes, in the order they appear.
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut inlined: Vec<&str> = Vec::new();
    let mut at = 0usize;
    while let Some(found) = svg[at..].find("<use") {
        let open = at + found;
        let Some(close) = svg[open..].find("/>") else {
            break;
        };
        let element = &svg[open..open + close + 2];
        at = open + close + 2;
        let Some((_, after)) = element.split_once("href=\"#") else {
            continue;
        };
        let Some((id, _)) = after.split_once('"') else {
            continue;
        };
        let Some((_, body, _)) = definitions.iter().find(|(known, _, _)| *known == id) else {
            continue;
        };
        let (x, y) = (use_attr(element, "x"), use_attr(element, "y"));
        inlined.push(id);
        edits.push((
            open..at,
            if x == 0.0 && y == 0.0 {
                (*body).to_string()
            } else {
                // What `<use x y>` means.
                format!("<g transform=\"translate({x},{y})\">{body}</g>")
            },
        ));
    }

    // A definition goes only once nothing refers to it any more: dropping
    // one regardless would delete a label whose `<use>` this did not read.
    for (id, _, drop) in &definitions {
        if inlined.contains(id) {
            edits.push((drop.clone(), String::new()));
        }
    }
    edits.sort_by_key(|(range, _)| range.start);

    // One write: scanning and rewriting the whole SVG once per label is
    // quadratic.
    let mut out = String::with_capacity(svg.len());
    let mut copied = 0usize;
    for (range, replacement) in &edits {
        // An edit inside an edit already made is left as it is.
        if range.start < copied {
            continue;
        }
        out.push_str(&svg[copied..range.start]);
        out.push_str(replacement);
        copied = range.end;
    }
    out.push_str(&svg[copied..]);
    out
}

/// Put back the entity references merman replaces with placeholders of its
/// own while it parses.
///
/// It rewrites `&name;` as `ﬂ°name¶ß` and `&#39;` as `ﬂ°°39¶ß` before
/// parsing, and some of those reach the output -- a `click` tooltip among
/// them, which a screen reader would read out as written.
pub(crate) fn decode_entity_placeholders(svg: &str) -> String {
    const OPEN: &str = "\u{fb02}\u{b0}";
    const CLOSE: &str = "\u{b6}\u{df}";
    if !svg.contains(OPEN) {
        return svg.to_string();
    }

    // Not `&name;` back again: this output is read as XML too, and XML
    // defines five names, so an SVG carrying `&nbsp;` is not a document usvg
    // will parse. Each reference is resolved as a browser would and written
    // as the characters themselves.
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        let after = &rest[at + OPEN.len()..];
        // `ﬂ°°39¶ß` stands for `&#39;`, `ﬂ°amp¶ß` for `&amp;`.
        let (numeric, body) = match after.strip_prefix('\u{b0}') {
            Some(body) => (true, body),
            None => (false, after),
        };
        // No terminator within a reference's length is not a reference.
        let Some(end) = body.find(CLOSE).filter(|&end| end <= 32) else {
            out.push_str(OPEN);
            rest = after;
            continue;
        };
        let (name, tail) = (&body[..end], &body[end + CLOSE.len()..]);
        let reference = if numeric {
            format!("&#{name};")
        } else {
            format!("&{name};")
        };
        let decoded = merman::model::text::decode_html_entities_to_unicode(&reference);
        if decoded.as_ref() == reference.as_str() {
            // A name HTML does not define either: `&amp;`, so the reader
            // sees `&name;` as a browser shows it.
            push_as_xml(&mut out, '&');
            out.push_str(&reference[1..]);
        } else {
            for c in decoded.chars() {
                push_as_xml(&mut out, c);
            }
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// One character, written so that both an XML parser and an HTML one read it
/// back as itself.
///
/// The five that carry meaning in markup are escaped, numerically for the
/// apostrophe, which HTML 4 does not name. A character XML does not permit
/// becomes the replacement character.
fn push_as_xml(out: &mut String, c: char) {
    match c {
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '&' => out.push_str("&amp;"),
        '"' => out.push_str("&quot;"),
        '\'' => out.push_str("&#39;"),
        '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'.. => {
            out.push(c)
        }
        _ => out.push('\u{fffd}'),
    }
}

/// Round the corners of every node's box.
///
/// Written as SVG's `rx` and `ry` on the box rather than as a rule, because
/// usvg takes only *presentation* attributes from a stylesheet: a rule would
/// round a box in a browser and leave it square in print.
///
/// The boxes are the rects merman marks as a node's own container. One with
/// a radius of its own is left as it is, since that is a shape someone
/// chose. A radius of zero is no radius, and is replaced rather than kept.
pub(crate) fn round_boxes(svg: &str, radius: f32) -> String {
    const CONTAINER: &str = "label-container";
    let radius = radius.max(0.0);
    let mut out = String::with_capacity(svg.len() + 64);
    let mut rest = svg;
    while let Some(at) = rest.find("<rect") {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let Some(end) = from.find('>') else {
            break;
        };
        let (tag, after) = from.split_at(end);
        let shaped = |name| {
            attribute(tag, name)
                .and_then(|value| value.trim().trim_end_matches("px").parse::<f32>().ok())
                .is_some_and(|value| value > 0.0)
        };
        if tag.contains(CONTAINER) && !shaped("rx") && !shaped("ry") {
            // A zero radius already written has to go, or the box would
            // carry the attribute twice and a parser would pick one.
            let tag = remove_attribute(&remove_attribute(tag, "rx"), "ry");
            // Both spellings: merman closes a node's box itself, and leaves
            // a cynefin domain's open for a `</rect>`.
            let (head, close) = match tag.trim_end().strip_suffix('/') {
                Some(head) => (head.to_string(), " /"),
                None => (tag.clone(), ""),
            };
            out.push_str(head.trim_end());
            out.push_str(&format!(" rx=\"{radius}\" ry=\"{radius}\"{close}"));
        } else {
            out.push_str(tag);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// The class Mermaid marks a plain link's line with, and so the selector
/// its weight is declared under.
const NORMAL_LINK: &str = ".edge-thickness-normal";

/// Give a link the weight asked for, keeping a thick link thick.
///
/// Mermaid marks a line with the weight it is drawn at --
/// `edge-thickness-normal` for a plain link, `-thick` for `==>`,
/// `-invisible` for `~~~` -- and declares each weight in the diagram's own
/// stylesheet, so the weights are changed where merman wrote them. A kind
/// Mermaid draws heavier is a weight of its own rather than a multiple, and
/// keeps it unless `width` is heavier still. A rule reaches print, unlike a
/// corner radius ([`round_boxes`]), because `stroke-width` is a presentation
/// property.
///
/// Rewritten in place, not appended: a kind that gives its own lines a
/// weight says so in a later rule of the same strength, and a rule appended
/// here would come later still and win instead.
///
/// The head a link points at is [`scale_heads`]'s business.
pub(crate) fn weigh_lines(svg: &str, width: f32) -> String {
    const MARK: &str = ".edge-thickness-";
    let width = width.max(0.0);
    // A document that declares no weight for a normal link draws no link
    // this way, and is handed back as it came rather than rebuilt.
    let Some((open, close)) = stylesheet(svg).filter(|_| declares_a_link(svg)) else {
        return svg.to_string();
    };
    let mut out = String::with_capacity(svg.len() + 16);
    out.push_str(&svg[..open]);
    let mut rest = &svg[open..close];
    while let Some(brace) = rest.find('{') {
        // The selectors, with the brace and the end of the rule before them.
        let (selectors, body) = rest.split_at(brace + 1);
        out.push_str(selectors);
        let end = body.find('}').unwrap_or(body.len());
        let (declarations, after) = body.split_at(end);
        if selectors.contains(MARK) {
            // The weight the rule names, or the heavier of the two.
            let plain = selectors.contains(NORMAL_LINK);
            out.push_str(&weighed(declarations, width, plain));
        } else {
            out.push_str(declarations);
        }
        rest = after;
    }
    out.push_str(rest);
    out.push_str(&svg[close..]);
    out
}

/// Where the document's stylesheet starts and ends: the first byte after the
/// `<style>` start tag, and the first byte of `</style>`.
fn stylesheet(svg: &str) -> Option<(usize, usize)> {
    let open = svg.find("<style")?;
    let close = svg.find("</style>")?;
    let at = svg.get(open..close)?.find('>')?;
    Some((open + at + 1, close))
}

/// The `stroke-width` the first rule naming `selector` asks for.
fn declared_width(sheet: &str, selector: &str) -> Option<f32> {
    let mut rest = sheet;
    while let Some(brace) = rest.find('{') {
        let (selectors, body) = rest.split_at(brace);
        let body = &body[1..];
        let end = body.find('}').unwrap_or(body.len());
        if selectors.contains(selector)
            && let Some(width) = asked_width(&body[..end])
        {
            return Some(width);
        }
        rest = &body[(end + 1).min(body.len())..];
    }
    None
}

/// The `stroke-width` in a declaration block, in pixels.
fn asked_width(declarations: &str) -> Option<f32> {
    declarations
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .find(|(property, _)| property.trim() == "stroke-width")
        .and_then(|(_, value)| value.trim().trim_end_matches("px").trim().parse().ok())
}

/// A declaration block with every `stroke-width` in it reweighed.
///
/// `plain` is the rule for a normal link, drawn at exactly the weight asked
/// for; any other keeps Mermaid's own unless what was asked is heavier
/// still. A weight of zero is left as it is.
fn weighed(declarations: &str, width: f32, plain: bool) -> String {
    let mut out = String::with_capacity(declarations.len() + 8);
    for declaration in declarations.split_inclusive(';') {
        let scaled = declaration.split_once(':').and_then(|(property, value)| {
            if property.trim() != "stroke-width" {
                return None;
            }
            // Whatever followed the value -- the semicolon, where there is
            // one -- goes back after it.
            let end = value.find(';').unwrap_or(value.len());
            let (asked, tail) = value.split_at(end);
            let declared: f32 = asked.trim().trim_end_matches("px").trim().parse().ok()?;
            let drawn = if plain { width } else { declared.max(width) };
            (declared > 0.0).then(|| format!("{property}:{drawn}px{tail}"))
        });
        match scaled {
            Some(written) => out.push_str(&written),
            None => out.push_str(declaration),
        }
    }
    out
}

/// Draw each arrowhead whole, at a size that follows the line's weight.
///
/// A `<marker>` clips its content to its own viewport unless it says
/// `overflow="visible"`, and Mermaid's triangle fills its viewBox edge to
/// edge, so the outer half of its outline and its mitred tip are cut away.
/// The size follows the weight, scaled from the size merman wrote so that
/// every head keeps the proportion to its line Mermaid chose for a thick
/// one, and `refX` stays in the middle of the head.
///
/// Mermaid resets the outline with `stroke-dasharray: 1, 0`, one unit on and
/// nothing off, which a browser draws as a solid line. A PDF carries the
/// pattern as written and renderers disagree over it, so the dash goes along
/// with the clip.
///
/// Only a head a document draws is touched, and only this head:
/// `arrowMarkerPath` is the class Mermaid marks it with, so a kind drawing a
/// head of its own keeps it. A document that declares no weight for a normal
/// link keeps every head, as it keeps its lines -- and at a weight of 0
/// [`weigh_lines`] has made it one of those documents.
pub(crate) fn scale_heads(svg: &str, width: f32) -> String {
    /// The weight at which merman's own head size is kept.
    const PAIRED: f32 = 2.0;
    const HEAD: &str = "arrowMarkerPath";
    const OPEN: &str = "<marker";
    const CLOSE: &str = "</marker>";
    if !declares_a_link(svg) {
        return svg.to_string();
    }
    let scale = width.max(0.0) / PAIRED;
    let mut out = String::with_capacity(svg.len() + 64);
    let mut rest = svg;
    while let Some(at) = rest.find(OPEN) {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        // A marker ends at its close tag, or where the next marker starts,
        // so one written without a body cannot swallow the one after it.
        let bound = from[1..].find(OPEN).map_or(from.len(), |at| at + 1);
        let end = from[..bound]
            .find(CLOSE)
            .map_or(bound, |at| at + CLOSE.len());
        let (element, after) = from.split_at(end);
        let tag = element.find('>').map_or(element.len(), |at| at + 1);
        let (open, body) = element.split_at(tag);
        let drawn = attribute(open, "id").is_some_and(|id| svg.contains(&format!("url(#{id})")));
        if drawn && body.contains(HEAD) {
            out.push_str(&whole(open, scale));
            out.push_str(&undashed(body));
        } else {
            out.push_str(element);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Whether this document draws a link at a weight it declares for one.
fn declares_a_link(svg: &str) -> bool {
    stylesheet(svg)
        .and_then(|(open, close)| declared_width(&svg[open..close], NORMAL_LINK))
        .is_some_and(|asked| asked > 0.0)
}

/// A marker's open tag, drawn whole at `scale` of the size it was written.
fn whole(open: &str, scale: f32) -> String {
    let tag = with_attribute(open, "overflow", "visible");
    ["markerWidth", "markerHeight"]
        .iter()
        .fold(tag, |tag, name| scaled(&tag, name, scale))
}

/// An open tag with the number `name` holds taken `scale` times.
fn scaled(open: &str, name: &str, scale: f32) -> String {
    match attribute(open, name).and_then(|size| size.trim().parse::<f32>().ok()) {
        Some(size) => with_attribute(open, name, &format!("{}", size * scale)),
        None => open.to_string(),
    }
}

/// An open tag carrying `name`: in place where it is already written, and
/// before the tag's own close otherwise.
fn with_attribute(open: &str, name: &str, value: &str) -> String {
    let needle = format!(" {name}=\"");
    if let Some(at) = open.find(&needle) {
        let from = at + needle.len();
        let end = open[from..].find('"').map_or(open.len(), |at| from + at);
        return format!("{}{value}{}", &open[..from], &open[end..]);
    }
    let (head, close) = open.split_at(open.rfind('>').unwrap_or(open.len()));
    match head.trim_end().strip_suffix('/') {
        Some(head) => format!("{} {name}=\"{value}\" /{close}", head.trim_end()),
        None => format!("{} {name}=\"{value}\"{close}", head.trim_end()),
    }
}

/// A marker's body with any dash that draws solid taken out of its styles.
fn undashed(body: &str) -> String {
    const STYLE: &str = " style=\"";
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find(STYLE) {
        let (before, from) = rest.split_at(at + STYLE.len());
        out.push_str(before);
        let Some(end) = from.find('"') else {
            rest = from;
            break;
        };
        let (style, after) = from.split_at(end);
        out.push_str(&without_solid_dash(style));
        rest = after;
    }
    out.push_str(rest);
    out
}

/// A style with a `stroke-dasharray` that draws solid taken out of it, and
/// handed back as it came where there is none.
fn without_solid_dash(style: &str) -> String {
    let kept: Vec<&str> = style
        .split(';')
        .filter(|declaration| !draws_solid(declaration))
        .collect();
    if kept.len() == style.split(';').count() {
        return style.to_string();
    }
    kept.join(";")
}

/// Whether a declaration is a `stroke-dasharray` drawing a solid line: a
/// dash with every gap in it zero.
///
/// An odd list of lengths repeats to an even one, so a dash written on its
/// own has itself for a gap and is a dash in earnest.
fn draws_solid(declaration: &str) -> bool {
    let Some((property, value)) = declaration.split_once(':') else {
        return false;
    };
    if property.trim() != "stroke-dasharray" {
        return false;
    }
    let mut lengths = Vec::new();
    for length in value
        .split([',', ' '])
        .filter(|part| !part.trim().is_empty())
    {
        let Ok(length) = length.trim().trim_end_matches("px").parse::<f32>() else {
            return false;
        };
        lengths.push(length);
    }
    if lengths.is_empty() {
        return false;
    }
    if lengths.len() % 2 == 1 {
        lengths.extend_from_within(..);
    }
    lengths.iter().skip(1).step_by(2).all(|gap| *gap == 0.0)
}

#[cfg(test)]
mod rounding_tests {
    /// A radius of zero is a square box said another way.
    #[test]
    fn a_radius_of_zero_is_no_radius() {
        let svg = concat!(
            "<svg>",
            "<rect class=\"basic label-container\" rx=\"0\" ry=\"0\" width=\"66\" height=\"40\"/>",
            "<rect class=\"basic label-container\" rx=\"5\" ry=\"5\" width=\"66\" height=\"40\"/>",
            "</svg>"
        );
        let rounded = super::round_boxes(svg, 3.0);
        // Written once, in place of the zero rather than beside it.
        assert_eq!(rounded.matches("rx=").count(), 2, "{rounded}");
        assert_eq!(rounded.matches("ry=").count(), 2, "{rounded}");
        assert!(rounded.contains("rx=\"3\" ry=\"3\""), "{rounded}");
        assert!(rounded.contains("rx=\"5\" ry=\"5\""), "{rounded}");
        assert!(!rounded.contains("rx=\"0\""), "{rounded}");
    }

    /// Only a node's box, and only one without a radius already.
    #[test]
    fn a_box_with_a_shape_of_its_own_keeps_it() {
        let svg = concat!(
            "<svg>",
            "<rect class=\"basic label-container\" x=\"-90\" y=\"-27\" width=\"180\" height=\"54\" />",
            "<rect class=\"basic label-container\" rx=\"27\" ry=\"27\" width=\"90\" height=\"54\"/>",
            "<rect x=\"0\" y=\"0\" width=\"400\" height=\"200\" fill=\"white\"/>",
            "</svg>"
        );
        let rounded = super::round_boxes(svg, 3.0);
        assert!(
            rounded.contains(concat!(
                "<rect class=\"basic label-container\" x=\"-90\" y=\"-27\" ",
                "width=\"180\" height=\"54\" rx=\"3\" ry=\"3\" />"
            )),
            "{rounded}"
        );
        // The stadium's own radius, and the canvas with none.
        assert!(rounded.contains("rx=\"27\" ry=\"27\""), "{rounded}");
        assert!(
            rounded.contains("<rect x=\"0\" y=\"0\" width=\"400\" height=\"200\" fill=\"white\"/>"),
            "{rounded}"
        );
    }

    /// Both spellings of the end of a start tag, and nothing else moved.
    #[test]
    fn the_rest_of_the_document_is_untouched() {
        for tag in [
            "<rect class=\"label-container\" width=\"10\" height=\"10\"/>",
            "<rect class=\"label-container\" width=\"10\" height=\"10\" />",
            "<rect class=\"label-container\" width=\"10\" height=\"10\"></rect>",
        ] {
            let svg = format!("<svg><g>{tag}<text>x</text></g></svg>");
            let rounded = super::round_boxes(&svg, 2.5);
            assert_eq!(rounded.matches("rx=\"2.5\"").count(), 1, "{rounded}");
            assert!(rounded.contains("<text>x</text></g></svg>"), "{rounded}");
            // The attributes go inside the start tag, whichever way it ends.
            assert!(
                rounded.contains("height=\"10\" rx=\"2.5\" ry=\"2.5\""),
                "{rounded}"
            );
            assert_eq!(
                rounded.matches("</rect>").count(),
                svg.matches("</rect>").count()
            );
        }
        // Nothing to round is nothing changed.
        let plain = "<svg><circle r=\"5\"/></svg>";
        assert_eq!(super::round_boxes(plain, 3.0), plain);
    }
}

#[cfg(test)]
mod weighing_tests {
    /// A diagram's own stylesheet, cut down to the rules that declare a
    /// weight: the three Mermaid writes for a link, a kind's own line, and a
    /// box's border.
    const SHEET: &str = concat!(
        "<svg><style>",
        "#d1 .edge-thickness-normal{stroke-width:1px;}",
        "#d1 .edge-thickness-thick{stroke-width:3.5px;}",
        "#d1 .edge-thickness-invisible{stroke-width:0;fill:none;}",
        "#d1 .node rect{fill:#fff;stroke:#111;stroke-width:1px;}",
        "#d1 .edge-depth-0{stroke-width:14;}",
        "#d1 .messageLine0{stroke-width:1.5;stroke-dasharray:none;}",
        "</style>",
        "<path class=\"edge-thickness-normal flowchart-link\" stroke-width=\"9\"/>",
        "</svg>"
    );

    /// What was asked for, with the proportions between the link kinds kept.
    #[test]
    fn a_link_is_drawn_at_the_weight_asked_for() {
        let heavier = super::weigh_lines(SHEET, 2.0);
        assert!(
            heavier.contains("#d1 .edge-thickness-normal{stroke-width:2px;}"),
            "{heavier}"
        );
        // Mermaid's own weight for a thick link, which is a number of its
        // own rather than a multiple of the normal one.
        assert!(
            heavier.contains("#d1 .edge-thickness-thick{stroke-width:3.5px;}"),
            "{heavier}"
        );
        // Zero is no line at all, at any weight.
        assert!(
            heavier.contains("#d1 .edge-thickness-invisible{stroke-width:0;fill:none;}"),
            "{heavier}"
        );
    }

    /// Only the rules that declare a link's weight: not a box's border, and
    /// not a weight a kind gives its own lines.
    #[test]
    fn nothing_but_a_link_is_reweighed() {
        let heavier = super::weigh_lines(SHEET, 2.0);
        for kept in [
            "#d1 .node rect{fill:#fff;stroke:#111;stroke-width:1px;}",
            "#d1 .edge-depth-0{stroke-width:14;}",
            "#d1 .messageLine0{stroke-width:1.5;stroke-dasharray:none;}",
            // Outside the stylesheet nothing is read at all.
            "<path class=\"edge-thickness-normal flowchart-link\" stroke-width=\"9\"/>",
        ] {
            assert!(heavier.contains(kept), "{kept} in {heavier}");
        }
    }

    /// A weight equal to merman's own is no change, and a document with no
    /// link to weigh is handed back as it came.
    #[test]
    fn nothing_to_weigh_is_nothing_written() {
        assert_eq!(super::weigh_lines(SHEET, 1.0), SHEET);
        for untouched in [
            "<svg><style>#d1 .node rect{stroke-width:1px;}</style></svg>",
            "<svg><path stroke-width=\"1\"/></svg>",
            "<svg><style></style></svg>",
            "",
        ] {
            assert_eq!(super::weigh_lines(untouched, 2.0), untouched);
        }
    }

    /// A fraction of a pixel, and a weight of none.
    #[test]
    fn a_weight_can_be_lighter_than_the_one_merman_draws() {
        let lighter = super::weigh_lines(SHEET, 0.5);
        assert!(
            lighter.contains("#d1 .edge-thickness-normal{stroke-width:0.5px;}"),
            "{lighter}"
        );
        // A thick link is thick: a lighter normal line does not drag it down.
        assert!(
            lighter.contains("#d1 .edge-thickness-thick{stroke-width:3.5px;}"),
            "{lighter}"
        );
        let none = super::weigh_lines(SHEET, 0.0);
        assert!(
            none.contains("#d1 .edge-thickness-normal{stroke-width:0px;}"),
            "{none}"
        );
    }

    /// A weight heavier than Mermaid's thick link takes it with it, or a
    /// `==>` would come out lighter than the `-->` beside it.
    #[test]
    fn a_weight_past_mermaids_thick_link_carries_it() {
        let heavy = super::weigh_lines(SHEET, 4.0);
        assert!(
            heavy.contains("#d1 .edge-thickness-normal{stroke-width:4px;}"),
            "{heavy}"
        );
        assert!(
            heavy.contains("#d1 .edge-thickness-thick{stroke-width:4px;}"),
            "{heavy}"
        );
    }
}

#[cfg(test)]
mod head_tests {
    /// A diagram cut down to a head a link points at, a head nothing points
    /// at, and the rule that says this document's links carry a weight.
    const HEADS: &str = concat!(
        "<svg><style>#d1 .edge-thickness-normal{stroke-width:1px;}</style>",
        "<marker id=\"d1-pointEnd\" class=\"marker\" viewBox=\"0 0 10 10\" refX=\"5\"",
        " refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"8\" markerHeight=\"8\"",
        " orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\" class=\"arrowMarkerPath\"",
        " style=\"stroke-width:1;stroke-dasharray:1, 0\"/></marker>",
        "<marker id=\"d1-circleEnd\" class=\"marker\" viewBox=\"0 0 10 10\" refX=\"11\"",
        " refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"11\" markerHeight=\"11\"",
        " orient=\"auto\"><circle cx=\"5\" cy=\"5\" r=\"5\" class=\"arrowMarkerPath\"",
        " style=\"stroke-width:1;stroke-dasharray:1, 0\"/></marker>",
        "<path class=\"flowchart-link\" marker-end=\"url(#d1-pointEnd)\"/></svg>"
    );

    /// Whole, and at merman's own size at the pinned weight.
    #[test]
    fn a_head_is_drawn_whole_at_the_size_mermaid_wrote() {
        let printed = super::scale_heads(HEADS, 2.0);
        assert!(
            printed.contains(concat!(
                "<marker id=\"d1-pointEnd\" class=\"marker\" viewBox=\"0 0 10 10\" refX=\"5\"",
                " refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"8\" markerHeight=\"8\"",
                " orient=\"auto\" overflow=\"visible\"><path d=\"M 0 0 L 10 5 L 0 10 z\"",
                " class=\"arrowMarkerPath\" style=\"stroke-width:1\"/></marker>"
            )),
            "{printed}"
        );
    }

    /// Heavier, and the head grows with the line in the proportion Mermaid
    /// drew it at.
    #[test]
    fn a_head_follows_the_weight_of_its_line() {
        let printed = super::scale_heads(HEADS, 2.5);
        assert!(
            printed.contains(
                "markerWidth=\"10\" markerHeight=\"10\" orient=\"auto\" overflow=\"visible\""
            ),
            "{printed}"
        );
    }

    /// A weight of 0 leaves every head as merman drew it: the second pass
    /// reads what the first wrote, and at 0 no weight is declared.
    #[test]
    fn a_weight_of_zero_leaves_every_head_alone() {
        let drawn = super::scale_heads(&super::weigh_lines(HEADS, 0.0), 0.0);
        assert_eq!(drawn, HEADS.replace("stroke-width:1px", "stroke-width:0px"));
    }

    /// A head nothing points at is left as merman wrote it.
    #[test]
    fn a_head_no_link_points_at_is_left_alone() {
        let kept = concat!(
            "<marker id=\"d1-circleEnd\" class=\"marker\" viewBox=\"0 0 10 10\" refX=\"11\"",
            " refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"11\" markerHeight=\"11\"",
            " orient=\"auto\"><circle cx=\"5\" cy=\"5\" r=\"5\" class=\"arrowMarkerPath\"",
            " style=\"stroke-width:1;stroke-dasharray:1, 0\"/></marker>"
        );
        for width in [1.0, 2.0, 2.5, 4.0] {
            let printed = super::scale_heads(HEADS, width);
            assert!(printed.contains(kept), "at {width}: {printed}");
        }
    }

    /// A document whose lines keep their own weight keeps its heads too.
    #[test]
    fn a_kind_that_weighs_its_own_lines_keeps_its_head() {
        let own = HEADS.replace(".edge-thickness-normal", ".usecase-relationship");
        assert_eq!(super::scale_heads(&own, 2.5), own);
        for untouched in [
            "<svg></svg>",
            "<svg><style></style><marker id=\"a\"></svg>",
            "",
        ] {
            assert_eq!(super::scale_heads(untouched, 2.5), untouched);
        }
    }

    /// A dash is taken out only where it draws a solid line anyway.
    #[test]
    fn a_dash_in_earnest_stays_where_it_is() {
        for solid in [
            "stroke-dasharray:1, 0",
            "stroke-dasharray:4,0",
            " stroke-dasharray: 2, 0 ",
        ] {
            assert!(super::draws_solid(solid), "{solid}");
        }
        for drawn in [
            "stroke-dasharray:4, 2",
            "stroke-dasharray:3",
            "stroke-dasharray:1, 0, 2, 3",
            "stroke-dasharray:none",
            "stroke-width:1",
            "stroke-dasharray",
        ] {
            assert!(!super::draws_solid(drawn), "{drawn}");
        }
        // And a style carrying one is handed back as it came.
        let dashed = HEADS.replace("stroke-dasharray:1, 0", "stroke-dasharray:4, 2");
        assert!(
            super::scale_heads(&dashed, 2.5)
                .contains("style=\"stroke-width:1;stroke-dasharray:4, 2\""),
            "a dash was taken for a reset"
        );
    }
}

#[cfg(test)]
mod inlining_tests {
    use super::*;

    /// A document's worth of labels, inlined in one pass, on an input large
    /// enough that a quadratic one would show.
    #[test]
    fn every_label_of_a_large_diagram_is_inlined() {
        const LABELS: usize = 240;
        let mut svg = String::from("<svg>");
        for i in 0..LABELS {
            svg.push_str(&format!(
                "<defs><g id=\"d-foreignobject-label-{i}\"><svg><text>Box number {i} of the \
                 spine</text></svg></g></defs>"
            ));
        }
        for i in 0..LABELS {
            // Half at the origin, half offset, which take different paths.
            if i % 2 == 0 {
                svg.push_str(&format!(
                    "<g class=\"node\"><use href=\"#d-foreignobject-label-{i}\" x=\"0\" y=\"0\"/></g>"
                ));
            } else {
                svg.push_str(&format!(
                    "<g class=\"node\"><use href=\"#d-foreignobject-label-{i}\" x=\"{i}\" y=\"25\"/></g>"
                ));
            }
        }
        svg.push_str("</svg>");

        let started = std::time::Instant::now();
        let out = inline_label_references(&svg);
        println!(
            "{LABELS} labels, {} KB: {:?}",
            svg.len() / 1024,
            started.elapsed()
        );

        assert_eq!(out.matches("<use").count(), 0, "a reference is left");
        assert_eq!(
            out.matches("-foreignobject-label-").count(),
            0,
            "a definition is left to refer to"
        );
        for i in 0..LABELS {
            assert!(
                out.contains(&format!("Box number {i} of the spine")),
                "label {i} is not drawn"
            );
        }
        // And the offsets became transforms.
        assert_eq!(
            out.matches("transform=\"translate(").count(),
            LABELS / 2,
            "an offset reference lost its translation"
        );
    }
}

/// How much of `rest` belongs to the `<g>` already opened, counting nested
/// `<g>` so a label holding one does not end the scan early.
fn group_body_len(rest: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut at = 0usize;
    while at < rest.len() {
        let next_open = rest[at..].find("<g").map(|i| at + i);
        let next_close = rest[at..].find("</g>").map(|i| at + i);
        match (next_open, next_close) {
            (Some(open), Some(close)) if open < close => {
                depth += 1;
                at = open + 2;
            }
            (_, Some(close)) => {
                depth -= 1;
                if depth == 0 {
                    return Some(close);
                }
                at = close + 4;
            }
            _ => return None,
        }
    }
    None
}

/// One numeric attribute of a `<use>`, or `0` where it has none.
fn use_attr(element: &str, name: &str) -> f32 {
    element
        .split_once(&format!(" {name}=\""))
        .and_then(|(_, rest)| rest.split_once('"'))
        .and_then(|(value, _)| value.parse().ok())
        .unwrap_or(0.0)
}
