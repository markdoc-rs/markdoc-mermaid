//! Spacing control, in this crate's own terms.
//!
//! The renderer's defaults are tuned for a diagram drawn on its own. A
//! diagram built from *document headings* is a different shape: the labels
//! are whole sentences rather than the three or four words someone types by
//! hand into a `.mmd` file, and they are long in a way the defaults do not
//! expect.
//!
//! That matters more than it sounds. With the renderer's own
//! `edge_gap_floor_ratio` of 0.55, four of the twenty-nine edge labels across
//! this project's three troubleshooting diagrams are drawn *underneath* a
//! node box and cannot be read at all — not crowded, not clipped: gone. On a
//! decision tree the edge labels are the branch conditions, so losing one
//! leaves a reader unable to tell which way to go.
//!
//! [`Layout::label_clearance`] defaults to `0.80`, the centre of the
//! `0.75..=0.85` plateau where all three of those diagrams come out clean,
//! and it costs about 1% in height. Above `0.90` the count goes back up and
//! moves around unpredictably, because nothing in the renderer *constrains*
//! a label to stay clear — wider gaps only make a collision less likely.
//!
//! That is the honest limit of this knob, and the reason it is a knob and
//! not a hard-coded constant.

use mermaid_rs_renderer::LayoutConfig;

/// How much room the diagram is given.
///
/// Every field has a default that suits labels read out of a document, so
/// `Layout::default()` is the right starting point and each field is an
/// escape hatch from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Clearance kept around an edge label, as a multiple of the gap between
    /// neighbouring nodes.
    ///
    /// Raise it when a label is drawn over a box; lower it for a tighter
    /// picture. See the module docs for why `0.80` and what it does not
    /// promise.
    pub label_clearance: f32,

    /// Gap between one rank of nodes and the next — vertical in a `TD`
    /// flowchart, horizontal in `LR`.
    ///
    /// `None` leaves the renderer to size it from the nodes it measured,
    /// which is usually right.
    pub rank_spacing: Option<f32>,

    /// Gap between nodes sharing a rank.
    ///
    /// `None` leaves it to the renderer.
    pub node_spacing: Option<f32>,

    /// Reroute the edges of a top-down flowchart: out of the bottom of a
    /// box, across in the gap below it, and into the top of the next, with
    /// the label on that last leg. See `route`, which says why the
    /// renderer's own routes are not kept. Off leaves its routes as drawn.
    pub tidy_edges: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            label_clearance: 0.80,
            rank_spacing: None,
            node_spacing: None,
            tidy_edges: true,
        }
    }
}

impl Layout {
    /// Translate into the renderer's configuration.
    ///
    /// Setting either spacing switches the diagram to explicit spacing and
    /// turns the renderer's adaptive pass off, because that pass treats the
    /// configured number as a *ceiling* rather than a value: it measures the
    /// nodes, takes half their mean extent, and then keeps whichever is
    /// smaller. Left on, asking for 90 and asking for 110 give the same
    /// picture, which is not what a caller setting a number means. The
    /// field left as `None` then falls back to the renderer's own default
    /// rather than to a measured one, since there is nothing to measure
    /// before layout has run.
    pub(crate) fn to_config(&self) -> LayoutConfig {
        let mut config = LayoutConfig::default();
        config.flowchart.objective.edge_gap_floor_ratio = self.label_clearance.max(0.0);
        if let Some(rank) = self.rank_spacing {
            config.rank_spacing = rank.max(0.0);
            config.flowchart.auto_spacing.enabled = false;
        }
        if let Some(node) = self.node_spacing {
            config.node_spacing = node.max(0.0);
            config.flowchart.auto_spacing.enabled = false;
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_reserves_more_room_than_the_renderer_would() {
        // The whole point of the default. If the renderer ever raises its
        // own floor to meet this, the override becomes a no-op and should be
        // reconsidered rather than silently kept.
        let theirs = LayoutConfig::default()
            .flowchart
            .objective
            .edge_gap_floor_ratio;
        let ours = Layout::default().to_config();
        assert!(
            ours.flowchart.objective.edge_gap_floor_ratio > theirs,
            "ours {} is no longer above the renderer's {theirs}",
            ours.flowchart.objective.edge_gap_floor_ratio
        );
    }

    #[test]
    fn leaving_spacing_alone_leaves_the_renderer_to_measure() {
        let config = Layout::default().to_config();
        assert!(
            config.flowchart.auto_spacing.enabled,
            "adaptive spacing should stay on when no spacing was asked for"
        );
    }

    #[test]
    fn asking_for_a_spacing_gets_that_spacing_and_not_a_capped_one() {
        // A number large enough that the adaptive pass would have clamped it
        // down. The point of the test is that it survives.
        let config = Layout {
            rank_spacing: Some(140.0),
            ..Layout::default()
        }
        .to_config();
        assert_eq!(config.rank_spacing, 140.0);
        assert!(!config.flowchart.auto_spacing.enabled);
    }

    #[test]
    fn a_negative_spacing_is_clamped_rather_than_handed_to_the_renderer() {
        let config = Layout {
            label_clearance: -3.0,
            node_spacing: Some(-20.0),
            ..Layout::default()
        }
        .to_config();
        assert_eq!(config.flowchart.objective.edge_gap_floor_ratio, 0.0);
        assert_eq!(config.node_spacing, 0.0);
    }
}
