//! markdoc-mermaid: a Wardley map's geometry.
//!
//! A component's place on the map *is* what the diagram says, so this is the
//! one kind here whose positions come from the source rather than from a
//! layout algorithm. Two axes: visibility up the left, evolution across the
//! bottom, divided into named stages.
//!
//! Mermaid writes a position `[visibility, evolution]`, visibility first,
//! which its own documentation flags as "opposite of typical (x, y)
//! notation". Both run 0 to 1, and visibility counts upwards, so it is
//! flipped here: SVG's y grows downwards.

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::text::measure_label;
use super::{
    DiagramData, Layout, WardleyComponentLayout, WardleyEvolutionLayout, WardleyLayout,
    WardleyLinkLayout, WardleyNoteLayout,
};

/// The four stages of evolution, as Wardley named them.
pub(super) const DEFAULT_STAGES: [&str; 4] = ["Genesis", "Custom-Built", "Product", "Commodity"];

pub(super) fn compute_wardley_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    const PLOT_WIDTH: f32 = 760.0;
    const PLOT_HEIGHT: f32 = 480.0;
    const PAD: f32 = 16.0;
    // Room for the axis names and the stage labels under the plot.
    const LEFT_GUTTER: f32 = 34.0;
    const BOTTOM_GUTTER: f32 = 46.0;

    let data = &graph.wardley;
    let title = data
        .title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(|t| measure_label(t, theme, config));
    let title_height = title.as_ref().map_or(0.0, |t| t.height + PAD);

    let plot_x = PAD + LEFT_GUTTER;
    let plot_y = PAD + title_height;

    // `visibility` counts up from the bottom; SVG's y counts down.
    let at = |visibility: f32, evolution: f32| {
        (
            plot_x + evolution.clamp(0.0, 1.0) * PLOT_WIDTH,
            plot_y + (1.0 - visibility.clamp(0.0, 1.0)) * PLOT_HEIGHT,
        )
    };

    let stages: Vec<String> = if data.stages.is_empty() {
        DEFAULT_STAGES.iter().map(|s| s.to_string()).collect()
    } else {
        data.stages.clone()
    };
    let stage_width = PLOT_WIDTH / stages.len().max(1) as f32;
    let stage_labels: Vec<(super::TextBlock, f32)> = stages
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let block = measure_label(name, theme, config);
            // Centred under its own band.
            let centre = plot_x + (idx as f32 + 0.5) * stage_width;
            (block, centre)
        })
        .collect();
    // The dividing lines, between bands rather than at the edges.
    let stage_bounds: Vec<f32> = (1..stages.len())
        .map(|idx| plot_x + idx as f32 * stage_width)
        .collect();

    let components: Vec<WardleyComponentLayout> = data
        .components
        .iter()
        .map(|component| {
            let (x, y) = at(component.visibility, component.evolution);
            WardleyComponentLayout {
                label: measure_label(&component.name, theme, config),
                x,
                y,
                anchor: component.anchor,
            }
        })
        .collect();

    // A link joins two components by name; one naming a component the map does
    // not declare is dropped rather than drawn to nowhere.
    let find = |name: &str| {
        data.components
            .iter()
            .position(|c| c.name == name)
            .map(|idx| {
                at(
                    data.components[idx].visibility,
                    data.components[idx].evolution,
                )
            })
    };
    let links: Vec<WardleyLinkLayout> = data
        .links
        .iter()
        .filter_map(|(from, to)| {
            let (x1, y1) = find(from)?;
            let (x2, y2) = find(to)?;
            Some(WardleyLinkLayout { x1, y1, x2, y2 })
        })
        .collect();

    let evolutions: Vec<WardleyEvolutionLayout> = data
        .evolutions
        .iter()
        .filter_map(|(name, target)| {
            let component = data.components.iter().find(|c| c.name == *name)?;
            let (from_x, y) = at(component.visibility, component.evolution);
            let (to_x, _) = at(component.visibility, *target);
            Some(WardleyEvolutionLayout { from_x, to_x, y })
        })
        .collect();

    let notes: Vec<WardleyNoteLayout> = data
        .notes
        .iter()
        .map(|note| {
            let (x, y) = at(note.visibility, note.evolution);
            WardleyNoteLayout {
                label: measure_label(&note.text, theme, config),
                x,
                y,
            }
        })
        .collect();

    let visibility_axis = measure_label("Visibility", theme, config);
    let evolution_axis = measure_label("Evolution", theme, config);

    Layout {
        kind: graph.kind,
        nodes: Default::default(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: plot_x + PLOT_WIDTH + PAD,
        height: plot_y + PLOT_HEIGHT + BOTTOM_GUTTER + PAD,
        diagram: DiagramData::Wardley(Box::new(WardleyLayout {
            title,
            title_y: PAD,
            plot_x,
            plot_y,
            plot_width: PLOT_WIDTH,
            plot_height: PLOT_HEIGHT,
            stage_labels,
            stage_bounds,
            visibility_axis,
            evolution_axis,
            components,
            links,
            evolutions,
            notes,
        })),
    }
}
