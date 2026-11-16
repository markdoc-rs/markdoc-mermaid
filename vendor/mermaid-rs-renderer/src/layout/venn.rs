//! vendored: a Venn diagram's circles, and where a label sits in them.
//!
//! This is the one of the four with no node-and-edge reading at all. A `union`
//! is a label placed in the region where the sets it names overlap and only
//! those sets overlap, so the circles have to be arranged before the label has
//! anywhere to go.
//!
//! One, two and three sets get the arrangements that actually produce every
//! region: a circle, two overlapping circles, and the classic triangle of
//! three. Four or more are placed on a ring, which draws every set but cannot
//! realise every intersection -- no arrangement of circles can, which is why
//! four-set Venn diagrams are drawn with ellipses. A union naming sets whose
//! region does not exist in the arrangement is still labelled, at the best
//! position available, rather than dropped.

use std::f32::consts::PI;

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::text::measure_label;
use super::{DiagramData, Layout, VennCircleLayout, VennLabelLayout, VennLayout};

pub(super) fn compute_venn_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    const PAD: f32 = 20.0;
    const RADIUS: f32 = 150.0;
    // How far a label for a partial overlap is pushed out of the middle, as a
    // multiple of its sets' centroid offset. Chosen so that on three circles
    // a pairwise label clears the all-three region in the middle.
    const PARTIAL_PUSH: f32 = 1.6;

    let data = &graph.venn;
    let title = data
        .title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(|t| measure_label(t, theme, config));
    let title_height = title.as_ref().map_or(0.0, |t| t.height + PAD);

    let count = data.sets.len();
    // How far each circle's centre sits from the middle. Two circles overlap
    // by about a third of their width; three sit on a tighter ring so that all
    // seven regions of a Venn diagram exist.
    let offset = match count {
        0 | 1 => 0.0,
        2 => RADIUS * 0.62,
        3 => RADIUS * 0.60,
        _ => RADIUS * 0.95,
    };
    // Two circles sit side by side; three and more go round a ring, starting
    // at the top so a three-set diagram reads the way it is usually drawn.
    let angle_of = |idx: usize| match count {
        2 => {
            if idx == 0 {
                PI
            } else {
                0.0
            }
        }
        _ => -PI / 2.0 + idx as f32 * 2.0 * PI / count.max(1) as f32,
    };

    let span = offset + RADIUS;
    let centre_x = PAD + span;
    let centre_y = PAD + title_height + span;

    let centre_of = |idx: usize| {
        let angle = angle_of(idx);
        (
            centre_x + offset * angle.cos(),
            centre_y + offset * angle.sin(),
        )
    };

    let circles: Vec<VennCircleLayout> = data
        .sets
        .iter()
        .enumerate()
        .map(|(idx, set)| {
            let (cx, cy) = centre_of(idx);
            let angle = angle_of(idx);
            // The set's name goes outside its own circle, pushed along the
            // line from the middle so two names never land on each other.
            let (label_x, label_y) = if count <= 1 {
                (cx, cy - RADIUS - 6.0)
            } else {
                (
                    cx + (RADIUS + 14.0) * angle.cos(),
                    cy + (RADIUS + 14.0) * angle.sin(),
                )
            };
            VennCircleLayout {
                label: measure_label(&set.label, theme, config),
                cx,
                cy,
                r: RADIUS,
                label_x,
                label_y,
            }
        })
        .collect();

    let labels: Vec<VennLabelLayout> = data
        .unions
        .iter()
        .filter_map(|union| {
            // Every named set has to exist, or there is no region to speak of.
            let indices: Option<Vec<usize>> = union
                .sets
                .iter()
                .map(|name| data.sets.iter().position(|set| set.id == *name))
                .collect();
            let indices = indices?;
            if indices.is_empty() {
                return None;
            }
            let centroid = indices.iter().fold((0.0, 0.0), |acc, idx| {
                let (x, y) = centre_of(*idx);
                (acc.0 + x, acc.1 + y)
            });
            let n = indices.len() as f32;
            let (mut x, mut y) = (centroid.0 / n, centroid.1 / n);
            // A union of every set is the middle. One of only some is pushed
            // away from the middle, into the region those sets share and the
            // others do not.
            if indices.len() < count {
                x = centre_x + (x - centre_x) * PARTIAL_PUSH;
                y = centre_y + (y - centre_y) * PARTIAL_PUSH;
            }
            Some(VennLabelLayout {
                label: measure_label(&union.label, theme, config),
                x,
                y,
            })
        })
        .collect();

    let width = PAD * 2.0 + span * 2.0;
    Layout {
        kind: graph.kind,
        nodes: Default::default(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height: centre_y + span + PAD,
        diagram: DiagramData::Venn(Box::new(VennLayout {
            title,
            title_y: PAD,
            title_x: width / 2.0,
            circles,
            labels,
        })),
    }
}
