//! Spacing, colours and line weights, in this crate's own terms.
//!
//! Each field maps onto one of Mermaid's configuration keys, so it means
//! here what it means in a Mermaid `%%{init}%%` block or site config.

use serde_json::{Map, Value, json};

/// Which of Mermaid's colour schemes a diagram that keeps its colours is
/// drawn in.
///
/// This is Mermaid's `theme`. Where the page owns the colours
/// ([`crate::Options::themeable`]) it is ignored and `Default` is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Palette {
    /// Mermaid's own default: lavender boxes, purple borders, grey plates.
    #[default]
    Default,
    /// Greys: `#eee` boxes, `#999` borders, `#666` lines. What a printed
    /// page usually wants.
    Neutral,
    /// Mermaid's `base`, the scheme meant to be adjusted by theme variables.
    Base,
    /// Slate: a near-white box with a grey-blue border, grey-blue lines and
    /// arrowheads, and near-black text. Mermaid's `base` with four theme
    /// variables set. Quieter than [`Palette::Default`] without going grey.
    Slate,
    /// Mermaid's `forest`: greens.
    Forest,
    /// Mermaid's `dark`, for a diagram drawn and kept on a dark background.
    /// Not what a dark *page* wants -- that is `themeable`.
    Dark,
    /// No scheme named at all, which merman answers with one of its own:
    /// white boxes with `#28253D` borders. Not available where the page owns
    /// the colours, which uses [`Palette::Default`].
    Unnamed,
}

impl Palette {
    /// What merman is told, or `None` to tell it nothing.
    fn as_str(self) -> Option<&'static str> {
        match self {
            Palette::Default => Some("default"),
            Palette::Neutral => Some("neutral"),
            Palette::Base | Palette::Slate => Some("base"),
            Palette::Forest => Some("forest"),
            Palette::Dark => Some("dark"),
            Palette::Unnamed => None,
        }
    }

    /// The theme variables the scheme needs on top of its name.
    fn variables(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Palette::Slate => &[
                ("primaryColor", "#F8FAFC"),
                ("primaryBorderColor", "#94A3B8"),
                ("lineColor", "#64748B"),
                ("primaryTextColor", "#0F172A"),
            ],
            _ => &[],
        }
    }
}

/// How much room the diagram is given, and how it is drawn.
///
/// Every field has a default that suits labels read out of a document, so
/// `Layout::default()` is the starting point and each field an adjustment.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Layout {
    /// Space between one rank of nodes and the next, along the flow.
    /// Mermaid's `flowchart.rankSpacing`. `None` leaves Mermaid's own.
    pub rank_spacing: Option<f32>,
    /// Space between nodes within a rank. Mermaid's
    /// `flowchart.nodeSpacing`. `None` leaves Mermaid's own.
    pub node_spacing: Option<f32>,
    /// The width a label is wrapped at, in pixels. Mermaid's
    /// `flowchart.wrappingWidth`. `None` leaves Mermaid's own, which is
    /// narrower than a heading wants.
    pub wrapping_width: Option<f32>,
    /// `look: classic`, rather than Mermaid 12's own `neo` look with its
    /// drop shadows and `Recursive Variable` font.
    ///
    /// The look and the colours are separate: this decides the shapes, and
    /// [`Self::palette`] the scheme, in any combination.
    pub classic_look: bool,
    /// The family the labels are *drawn* in, as Mermaid's `fontFamily`.
    ///
    /// It does not change the layout: the renderer measures with a metric
    /// table of its own. Name a family you serve *and* measure with
    /// [`crate::Options::measure`], so both ends agree, or buy room with
    /// [`Self::label_slack`]. `None` leaves Mermaid's own stack.
    pub font_family: Option<String>,
    /// Which of Mermaid's colour schemes a diagram that keeps its colours is
    /// drawn in. See [`Palette`]: it has no effect where the page owns the
    /// colours.
    pub palette: Palette,
    /// An allowance on every measured label, as a multiplier.
    ///
    /// 1.0 asks for none. 1.2 lays the diagram out as though every label
    /// were a fifth wider than it measures. It buys room for a page that
    /// cannot know the reader's font; prefer [`crate::Options::measure`]
    /// where the font can be loaded. A kind whose layout does not measure
    /// text is unaffected.
    pub label_slack: f32,
    /// How far the corners of a node's box are rounded, in user units.
    ///
    /// `None` leaves every box as merman draws it, which under
    /// [`Self::classic_look`] is square. A box whose shape is already round
    /// -- a stadium, a state, a kanban card -- keeps its own radius.
    pub corner_radius: Option<f32>,
    /// The weight of a link's line, in pixels.
    ///
    /// `None` leaves every line as merman draws it. A link Mermaid draws
    /// heavier keeps its own weight unless this is heavier still, and an
    /// invisible link stays invisible. A head a link points at is drawn
    /// whole and sized with the weight. A kind that gives its own lines a
    /// weight keeps it.
    pub line_width: Option<f32>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            rank_spacing: None,
            node_spacing: None,
            wrapping_width: Some(180.0),
            classic_look: true,
            font_family: None,
            palette: Palette::default(),
            label_slack: 1.0,
            corner_radius: None,
            line_width: None,
        }
    }
}

impl Layout {
    /// The site config merman reads, as Mermaid's own keys.
    ///
    /// `themeable` asks for `var(--diagram-…)` tokens in place of merman's
    /// own colours, for the page's stylesheet to fill in. `kind` is merman's
    /// `diagram_type`: which keys take a token depends on the family.
    pub(crate) fn to_site_config(&self, themeable: bool, kind: &str, seed: u32) -> Value {
        let mut flowchart = Map::new();
        if let Some(rank) = self.rank_spacing {
            flowchart.insert("rankSpacing".into(), json!(rank.max(0.0)));
        }
        if let Some(node) = self.node_spacing {
            flowchart.insert("nodeSpacing".into(), json!(node.max(0.0)));
        }
        if let Some(wrap) = self.wrapping_width {
            flowchart.insert("wrappingWidth".into(), json!(wrap.max(0.0)));
        }

        let mut root = Map::new();
        if !flowchart.is_empty() {
            root.insert("flowchart".into(), Value::Object(flowchart));
        }
        if self.classic_look {
            root.insert("look".into(), json!("classic"));
        }
        // `default` whatever was asked for: the tokenising pass knows the
        // literals that scheme emits and no others.
        let palette = if themeable {
            Palette::Default
        } else {
            self.palette
        };
        if let Some(theme) = palette.as_str() {
            root.insert("theme".into(), json!(theme));
        }
        if !palette.variables().is_empty() {
            root.insert(
                "themeVariables".into(),
                Value::Object(
                    palette
                        .variables()
                        .iter()
                        .map(|(key, value)| ((*key).to_string(), json!(value)))
                        .collect(),
                ),
            );
        }
        if let Some(family) = &self.font_family {
            root.insert("fontFamily".into(), json!(family));
        }
        // merman seeds a cynefin diagram's wobble from the diagram id
        // unless the config names a seed. A seed in the author's own front
        // matter still wins over this one.
        if kind == "cynefin" {
            root.insert("cynefin".into(), json!({ "seed": seed }));
        }
        if themeable {
            let mut vars = json!({
                    // Only the keys merman will take a `var()` for: it reads
                    // `edgeLabelBackground` and `primaryColor` as real
                    // colours and rejects the source for those two.
                    "textColor": "var(--diagram-text)",
                    "nodeTextColor": "var(--diagram-text)",
                    "titleColor": "var(--diagram-text)",
                    "lineColor": "var(--diagram-line)",
                    "nodeBorder": "var(--diagram-border)",
                    "mainBkg": "var(--diagram-node)",
                    // Per-kind text, for the kinds that paint their own and
                    // ignore `textColor` -- and only where the surface behind
                    // the text is tokenised too, or the pair loses contrast.
                    "classText": "var(--diagram-text)",
                    "actorTextColor": "var(--diagram-text)",
                    "signalTextColor": "var(--diagram-text)",
                    "loopTextColor": "var(--diagram-text)",
                    "stateLabelColor": "var(--diagram-text)",
                    // The state's box, which goes with its label: `mainBkg`
                    // does not reach `stateBkg` in the `default` theme.
                    "stateBkg": "var(--diagram-node)",
                    // Pinned, not left unset: the default theme derives it
                    // from `actorTextColor` above, and a note's own pale
                    // background is not tokenised.
                    "noteTextColor": "black",
                    "pieTitleTextColor": "var(--diagram-text)",
                    "pieLegendTextColor": "var(--diagram-text)",
            });
            // Only where the family will take them; a block diagram will not.
            if kind != "block" {
                let map = vars.as_object_mut().expect("an object");
                map.insert("clusterBorder".into(), json!("var(--diagram-border)"));
                map.insert("clusterBkg".into(), json!("var(--diagram-surface)"));
            }
            root.insert("themeVariables".into(), vars);
        }
        Value::Object(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_asks_for_the_plain_look_and_a_wider_wrap() {
        let config = Layout::default().to_site_config(false, "flowchart-v2", 0);
        assert_eq!(config["look"], json!("classic"));
        assert_eq!(config["theme"], json!("default"));
        assert_eq!(config["flowchart"]["wrappingWidth"], json!(180.0));
        // Spacing is Mermaid's own unless asked for.
        assert!(config["flowchart"].get("rankSpacing").is_none());
        assert!(config["flowchart"].get("nodeSpacing").is_none());
    }

    #[test]
    fn only_a_cynefin_is_given_a_seed() {
        let cynefin = Layout::default().to_site_config(false, "cynefin", 7);
        assert_eq!(cynefin["cynefin"]["seed"], json!(7));
        let flowchart = Layout::default().to_site_config(false, "flowchart-v2", 7);
        assert!(flowchart.get("cynefin").is_none(), "{flowchart}");
    }

    #[test]
    fn a_spacing_asked_for_reaches_mermaids_own_key() {
        let layout = Layout {
            rank_spacing: Some(70.0),
            node_spacing: Some(40.0),
            ..Layout::default()
        };
        let config = layout.to_site_config(false, "flowchart-v2", 0);
        assert_eq!(config["flowchart"]["rankSpacing"], json!(70.0));
        assert_eq!(config["flowchart"]["nodeSpacing"], json!(40.0));
    }

    #[test]
    fn themeable_asks_mermaid_for_the_pages_colours_rather_than_its_own() {
        let plain = Layout::default().to_site_config(false, "flowchart-v2", 0);
        assert!(plain.get("themeVariables").is_none());
        let themed = Layout::default().to_site_config(true, "flowchart-v2", 0);
        assert_eq!(
            themed["themeVariables"]["lineColor"],
            json!("var(--diagram-line)")
        );
        assert_eq!(
            themed["themeVariables"]["mainBkg"],
            json!("var(--diagram-node)")
        );
    }

    #[test]
    fn a_palette_reaches_the_renderer_only_where_the_colours_are_kept() {
        let layout = Layout {
            palette: Palette::Neutral,
            ..Layout::default()
        };
        // Kept: the scheme is asked for.
        assert_eq!(
            layout.to_site_config(false, "flowchart-v2", 0)["theme"],
            json!("neutral")
        );
        // Owned by the page: `default` regardless.
        assert_eq!(
            layout.to_site_config(true, "flowchart-v2", 0)["theme"],
            json!("default")
        );
        for themeable in [true, false] {
            assert_eq!(
                Layout::default().to_site_config(themeable, "flowchart-v2", 0)["theme"],
                json!("default")
            );
        }
    }

    #[test]
    fn the_scheme_is_named_whatever_the_look() {
        let plain = Layout {
            classic_look: false,
            palette: Palette::Neutral,
            ..Layout::default()
        };
        let config = plain.to_site_config(false, "flowchart-v2", 0);
        assert_eq!(config["theme"], json!("neutral"));
        assert!(
            config.get("look").is_none(),
            "the look was asked for anyway"
        );
        // A themed diagram always names a scheme.
        assert_eq!(
            plain.to_site_config(true, "flowchart-v2", 0)["theme"],
            json!("default")
        );
    }

    #[test]
    fn a_named_family_reaches_mermaids_own_key() {
        let layout = Layout {
            font_family: Some("Noto Sans".into()),
            wrapping_width: Some(100.0),
            ..Layout::default()
        };
        let config = layout.to_site_config(false, "flowchart-v2", 0);
        assert_eq!(config["fontFamily"], json!("Noto Sans"));
        // The slack belongs to the measurement, not to the wrap width.
        assert_eq!(config["flowchart"]["wrappingWidth"], json!(100.0));
        let plain = Layout::default().to_site_config(false, "flowchart-v2", 0);
        assert_eq!(plain["flowchart"]["wrappingWidth"], json!(180.0));
        assert!(plain.get("fontFamily").is_none());
    }

    #[test]
    fn a_negative_spacing_is_clamped_rather_than_passed_on() {
        let layout = Layout {
            node_spacing: Some(-5.0),
            ..Layout::default()
        };
        assert_eq!(
            layout.to_site_config(false, "flowchart-v2", 0)["flowchart"]["nodeSpacing"],
            json!(0.0)
        );
    }
}
