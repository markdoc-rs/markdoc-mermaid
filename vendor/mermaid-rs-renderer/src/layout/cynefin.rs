//! vendored: the Cynefin framework's five domains.
//!
//! An item's domain is the whole content of this diagram, and a domain's
//! place never moves: complex top left, complicated top right, chaotic bottom
//! left, clear bottom right, confusion in the middle. So there is nothing to
//! lay out in the usual sense -- the geometry is fixed and the items are
//! stacked into it.

use crate::config::LayoutConfig;
use crate::ir::{CynefinDomain, Graph};
use crate::theme::Theme;

use super::text::measure_label;
use super::{CynefinDomainLayout, CynefinLayout, CynefinLinkLayout, DiagramData, Layout};

/// How many items the centre ellipse shows before it counts the rest.
const CONFUSION_SHOWN: usize = 3;

pub(super) fn compute_cynefin_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    const FIELD: f32 = 620.0;
    const PAD: f32 = 16.0;
    const ITEM_GAP: f32 = 4.0;

    let data = &graph.cynefin;
    let title = data
        .title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(|t| measure_label(t, theme, config));
    let title_height = title.as_ref().map_or(0.0, |t| t.height + PAD);

    let field_x = PAD;
    let field_y = PAD + title_height;
    let half = FIELD / 2.0;

    // The centre ellipse, which the four quadrants have to leave room for.
    let centre = (field_x + half, field_y + half);
    let ellipse_rx = FIELD * 0.17;
    let ellipse_ry = FIELD * 0.12;

    let mut domains = Vec::with_capacity(5);
    for domain in CynefinDomain::ALL {
        let items = &data.items[domain.index()];
        let (x, y) = match domain {
            CynefinDomain::Complex => (field_x, field_y),
            CynefinDomain::Complicated => (field_x + half, field_y),
            CynefinDomain::Chaotic => (field_x, field_y + half),
            CynefinDomain::Clear => (field_x + half, field_y + half),
            // Placed from its centre, not its corner, so it reads as one shape
            // rather than a fifth quadrant.
            CynefinDomain::Confusion => (centre.0 - ellipse_rx, centre.1 - ellipse_ry),
            // (the match is exhaustive over `ALL`)
        };
        let (width, height) = if domain == CynefinDomain::Confusion {
            (ellipse_rx * 2.0, ellipse_ry * 2.0)
        } else {
            (half, half)
        };

        // The centre is small, so it shows a few items and says how many it
        // is holding back rather than overflowing its own ellipse.
        let shown = if domain == CynefinDomain::Confusion {
            items.len().min(CONFUSION_SHOWN)
        } else {
            items.len()
        };
        let overflow = items.len() - shown;
        let item_blocks: Vec<super::TextBlock> = items[..shown]
            .iter()
            .map(|item| measure_label(item, theme, config))
            .collect();
        let overflow_block =
            (overflow > 0).then(|| measure_label(&format!("+{overflow} more"), theme, config));

        domains.push(CynefinDomainLayout {
            domain,
            name: measure_label(domain.title(), theme, config),
            x,
            y,
            width,
            height,
            items: item_blocks,
            overflow: overflow_block,
            item_gap: ITEM_GAP,
        });
    }

    // A link joins two domains, drawn centre to centre, so it has to miss the
    // boxes' own text: the ends are pulled in towards each other.
    let domain_centre = |domain: CynefinDomain| match domain {
        CynefinDomain::Complex => (field_x + half * 0.5, field_y + half * 0.5),
        CynefinDomain::Complicated => (field_x + half * 1.5, field_y + half * 0.5),
        CynefinDomain::Chaotic => (field_x + half * 0.5, field_y + half * 1.5),
        CynefinDomain::Clear => (field_x + half * 1.5, field_y + half * 1.5),
        CynefinDomain::Confusion => centre,
    };
    let links: Vec<CynefinLinkLayout> = data
        .links
        .iter()
        .map(|link| {
            let (x1, y1) = domain_centre(link.from);
            let (x2, y2) = domain_centre(link.to);
            CynefinLinkLayout {
                x1,
                y1,
                x2,
                y2,
                label: link
                    .label
                    .as_deref()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| measure_label(l, theme, config)),
            }
        })
        .collect();

    Layout {
        kind: graph.kind,
        nodes: Default::default(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: field_x + FIELD + PAD,
        height: field_y + FIELD + PAD,
        diagram: DiagramData::Cynefin(Box::new(CynefinLayout {
            title,
            title_y: PAD,
            field_x,
            field_y,
            field_size: FIELD,
            centre_x: centre.0,
            centre_y: centre.1,
            ellipse_rx,
            ellipse_ry,
            domains,
            links,
        })),
    }
}
