//! Make the renderer's SVG fit a page it knows nothing about.
//!
//! `mermaid-rs-renderer` emits a finished picture: a white canvas, fixed
//! pixel dimensions, and every colour written into the markup. That is the
//! right default for a standalone file and the wrong one for a document.
//!
//! Three things are undone here.
//!
//! **The canvas.** The first element is a `<rect>` the size of the whole
//! drawing, filled white. On a page with a dark theme that is a white slab;
//! in a PDF it is a white box over whatever the sheet looks like. It goes.
//!
//! Note that `#FFFFFF` is *not* only the canvas: the renderer also paints it
//! behind edge labels, so a line crossing underneath does not strike through
//! the text. Those have to stay. The canvas is identified by its geometry —
//! `x="0" y="0"` at exactly the root's width and height — and removed on its
//! own, before any colour is touched.
//!
//! **The dimensions.** `width` and `height` on the root pin the drawing to
//! one size. Removed, leaving `viewBox`, which is what lets CSS size it: the
//! same diagram then fits a phone, a text column and a printed page.
//!
//! **The colours.** Each becomes `var(--token, original)`. With no stylesheet
//! the fallback is what the renderer chose, so nothing looks worse than
//! before; with one, the diagram follows the page into dark mode. This is how
//! the hand-drawn tree this replaces already worked, and it is the part a
//! renderer cannot do for you, because it has never seen the page.

/// What each colour the renderer bakes in is *for*, and the token that
/// overrides it. Taken from the output rather than the documentation: these
/// are the literal values in a rendered flowchart as of 0.3.1.
const PALETTE: &[(&str, &str)] = &[
    // Node and edge-label text.
    ("#0F172A", "--diagram-text"),
    // Edge lines and arrowheads.
    ("#64748B", "--diagram-line"),
    // Node outlines and edge-label outlines.
    ("#94A3B8", "--diagram-border"),
    // Node body.
    ("#F8FAFC", "--diagram-node"),
    // Edge-label backplate. Only reached after the canvas rect is gone.
    ("#FFFFFF", "--diagram-surface"),
];

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
    if !class.is_empty() {
        head = match head.strip_suffix('>') {
            Some(body) => format!("{body} class=\"{class}\">"),
            None => head,
        };
    }

    let mut body = rest.to_string();
    if let (Some(w), Some(h)) = (width, height) {
        body = remove_canvas(&body, &w, &h);
    }
    for (colour, token) in PALETTE {
        body = replace_colour(&body, colour, token);
    }

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
/// Matched on geometry rather than on being first or on its colour: an
/// edge-label backplate is the same colour, and depends on the diagram for
/// its position, so colour and order are both unsafe to key on.
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

/// Swap a literal colour for a custom property that falls back to it.
///
/// Case-insensitive on the hex digits, because `#FFFFFF` and `#ffffff` are
/// the same colour and a renderer is free to change its mind about which to
/// write.
fn replace_colour(body: &str, colour: &str, token: &str) -> String {
    let replacement = format!("var({token}, {colour})");
    let mut out = String::with_capacity(body.len());
    let lower = body.to_ascii_lowercase();
    let needle = colour.to_ascii_lowercase();
    let mut from = 0usize;
    while let Some(at) = lower[from..].find(&needle) {
        let at = from + at;
        out.push_str(&body[from..at]);
        out.push_str(&replacement);
        from = at + colour.len();
    }
    out.push_str(&body[from..]);
    out
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
        // Both are #FFFFFF. Keying on colour would strike the text out of
        // every edge label; keying on "first" would break the day the
        // renderer emits <defs> ahead of it.
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
    fn every_colour_becomes_overridable_and_falls_back_to_itself() {
        // With no stylesheet the diagram looks exactly as the renderer drew
        // it, which is what makes this safe to apply unconditionally.
        let out = themeable(SVG, "");
        for (colour, token) in PALETTE {
            assert!(
                out.contains(&format!("var({token}, {colour})")),
                "{colour} not tokenised: {out}"
            );
        }
        assert!(!out.contains(r##"fill="#0F172A""##), "{out}");
    }

    #[test]
    fn lowercase_hex_is_recognised_too() {
        let svg = r##"<svg viewBox="0 0 1 1"><text fill="#0f172a">x</text></svg>"##;
        let out = themeable(svg, "");
        assert!(out.contains("var(--diagram-text, #0F172A)"), "{out}");
    }

    #[test]
    fn an_svg_without_dimensions_survives() {
        // Then there is no canvas rect to find either, and nothing to undo.
        let svg = r##"<svg viewBox="0 0 10 10"><text fill="#0F172A">x</text></svg>"##;
        let out = themeable(svg, "d");
        assert!(out.contains("var(--diagram-text"), "{out}");
        assert!(out.contains(r#"class="d""#), "{out}");
    }

    #[test]
    fn something_that_is_not_svg_is_returned_unharmed() {
        assert_eq!(themeable("not markup", "d"), "not markup");
    }
}
