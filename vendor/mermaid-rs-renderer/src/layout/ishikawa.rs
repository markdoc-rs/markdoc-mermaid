//! markdoc-mermaid: an Ishikawa, or fishbone, diagram.
//!
//! A spine running left to right into the problem at the head, and a bone per
//! category angled back off it, alternating above and below so that the
//! categories do not crowd each other. The causes sit along their own bone.
//!
//! A tree would carry the same information, which is why this was the last of
//! the four to be drawn: the spine and the angled bones are the diagram, not
//! the hierarchy, and an author asking for a fishbone is asking for them.

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::text::measure_label;
use super::{DiagramData, IshikawaBoneLayout, IshikawaCauseLayout, IshikawaLayout, Layout};

pub(super) fn compute_ishikawa_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    const PAD: f32 = 20.0;
    // How far back along the spine a bone reaches, and how far out from it.
    const BONE_RUN: f32 = 110.0;
    const BONE_RISE: f32 = 150.0;
    // Room between two bones' attachment points on the spine.
    const BONE_SPACING: f32 = 150.0;
    const CAUSE_GAP: f32 = 4.0;
    const SUB_INDENT: f32 = 12.0;

    let data = &graph.ishikawa;
    let problem = measure_label(data.problem.as_deref().unwrap_or("Problem"), theme, config);

    // Each bone carries its causes, so its own length follows how many it has;
    // the tallest side decides how far the spine sits from the top.
    let bones_above = data.categories.len().div_ceil(2);
    let bones_below = data.categories.len() / 2;
    let cause_height = theme.font_size + CAUSE_GAP;
    let side_depth = |bones: usize| -> f32 {
        if bones == 0 {
            return 0.0;
        }
        let deepest = data
            .categories
            .iter()
            .map(|category| category.causes.len())
            .max()
            .unwrap_or(0);
        BONE_RISE + deepest as f32 * cause_height + PAD
    };
    let above_depth = side_depth(bones_above).max(60.0);
    let below_depth = side_depth(bones_below).max(60.0);

    let spine_y = PAD + above_depth;
    let spine_x = PAD;
    // Every bone needs its own stretch of spine, plus a run of spine before
    // the first one and the head at the end.
    let spine_length =
        BONE_RUN + BONE_SPACING * data.categories.len().max(1) as f32 + problem.width * 0.25;
    let head_x = spine_x + spine_length;

    let mut bones = Vec::with_capacity(data.categories.len());
    for (idx, category) in data.categories.iter().enumerate() {
        let above = idx % 2 == 0;
        // Attached right to left in the order they were written, so the first
        // category is nearest the head: the way a fishbone is read.
        let position = data.categories.len() - idx;
        let attach_x = head_x - position as f32 * BONE_SPACING;
        let rise = if above { -BONE_RISE } else { BONE_RISE };
        let tip_x = attach_x - BONE_RUN;
        let tip_y = spine_y + rise;

        let name = measure_label(&category.name, theme, config);
        // The causes step from the tip back towards the spine, each one out to
        // the side of the bone so the bone itself stays visible.
        let causes: Vec<IshikawaCauseLayout> = category
            .causes
            .iter()
            .enumerate()
            .map(|(cause_idx, cause)| {
                let step = cause_idx as f32 + 1.0;
                let along = step * cause_height;
                let ratio = (along / BONE_RISE).min(1.0);
                IshikawaCauseLayout {
                    label: measure_label(&cause.text, theme, config),
                    x: tip_x + BONE_RUN * ratio + 8.0 + cause.depth as f32 * SUB_INDENT,
                    y: tip_y - rise * ratio,
                    sub: cause.depth > 0,
                }
            })
            .collect();

        bones.push(IshikawaBoneLayout {
            name,
            attach_x,
            attach_y: spine_y,
            tip_x,
            tip_y,
            above,
            causes,
        });
    }

    let width = head_x + problem.width + PAD * 2.0;
    let height = spine_y + below_depth + PAD;

    Layout {
        kind: graph.kind,
        nodes: Default::default(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::Ishikawa(Box::new(IshikawaLayout {
            problem,
            problem_x: head_x,
            problem_y: spine_y,
            spine_x,
            spine_y,
            spine_end_x: head_x,
            bones,
        })),
    }
}
