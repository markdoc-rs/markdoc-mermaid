//! A caller's own text measurement.
//!
//! The renderer lays a label out by measuring it and something else draws it,
//! so a caller that can load the drawing font should measure with it. See
//! [`crate::Options::measure`], and [`crate::Layout::label_slack`] for where
//! nothing can.

use std::sync::Arc;

/// One run of text the renderer is about to lay out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Text<'a> {
    /// The text itself, whitespace already collapsed as SVG would.
    pub text: &'a str,
    /// The family asked for, as a CSS font stack: `None` where the diagram
    /// named none. [`crate::Layout::font_family`] is what puts one here.
    pub family: Option<&'a str>,
    /// The size in pixels.
    pub size: f64,
    /// The weight asked for, as CSS writes it (`bold`, `600`), if any.
    pub weight: Option<&'a str>,
    /// Whether the run is italic or oblique.
    pub italic: bool,
}

/// A function that gives the width of a [`Text`] in pixels.
///
/// Line height and wrapping stay the renderer's own, so a caller need only
/// shape one run with one font.
#[derive(Clone)]
pub struct Measure(Arc<dyn Fn(Text<'_>) -> f64 + Send + Sync>);

impl Measure {
    pub fn new(width_px: impl Fn(Text<'_>) -> f64 + Send + Sync + 'static) -> Self {
        Self(Arc::new(width_px))
    }

    /// The width of one run, in pixels.
    pub fn width(&self, text: Text<'_>) -> f64 {
        (self.0)(text)
    }
}

/// `Options` derives `Debug`, and a closure has none to give.
impl std::fmt::Debug for Measure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Measure(..)")
    }
}

/// The measurer merman is handed, wrapping the caller's.
pub(crate) fn policy(measure: &Measure) -> merman::svg::TextMeasurementPolicy {
    let measure = measure.clone();
    let backend = merman::svg::DeterministicTextMeasurer::default().with_width_callback(
        move |text: &str, style: &merman::svg::TextStyle| {
            let italic = style
                .font_style
                .as_deref()
                .is_some_and(|s| matches!(s.trim(), "italic" | "oblique"));
            measure.width(Text {
                text,
                family: style.font_family.as_deref(),
                size: style.font_size,
                weight: style.font_weight.as_deref(),
                italic,
            })
        },
    );
    // Literals, so neither can be the empty string these reject.
    let id = merman::svg::MeasurementProfileId::new("markdoc-mermaid-host")
        .expect("a literal profile name");
    let identity = merman::svg::TextMeasurementProfileIdentity::new(id, "1")
        .expect("a literal profile version");
    merman::svg::TextMeasurementPolicy::uniform(merman::svg::TextMeasurementProfile::new(
        identity, backend,
    ))
}

/// merman's own estimate for a line, scaled.
///
/// This is what [`crate::Layout::label_slack`] does: the allowance reaches
/// the measurement rather than the wrap width, so a short label gains room
/// too.
pub(crate) fn slack_policy(slack: f32) -> merman::svg::TextMeasurementPolicy {
    use merman::svg::TextMeasurer;

    let own = merman::svg::DeterministicTextMeasurer::default();
    let slack = f64::from(slack);
    let backend = merman::svg::DeterministicTextMeasurer::default().with_width_callback(
        move |text: &str, style: &merman::svg::TextStyle| {
            own.measure_html_line_width_px(text, style) * slack
        },
    );
    let id =
        merman::svg::MeasurementProfileId::new("markdoc-mermaid-slack").expect("a literal name");
    let identity = merman::svg::TextMeasurementProfileIdentity::new(id, "1")
        .expect("a literal profile version");
    merman::svg::TextMeasurementPolicy::uniform(merman::svg::TextMeasurementProfile::new(
        identity, backend,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_measure_is_debuggable_and_cloneable_so_options_stays_both() {
        let m = Measure::new(|t: Text<'_>| t.text.chars().count() as f64 * t.size * 0.5);
        assert_eq!(format!("{m:?}"), "Measure(..)");
        let c = m.clone();
        let t = Text {
            text: "abcd",
            family: None,
            size: 10.0,
            weight: None,
            italic: false,
        };
        assert_eq!(c.width(t), 20.0);
    }
}
