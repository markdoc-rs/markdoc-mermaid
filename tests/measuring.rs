//! A caller's own measurement, and the allowance for where there is none.
use markdoc_mermaid::{Anchor, Anchors, Layout, Measure, Options, Text, render};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct None_;
impl Anchors for None_ {
    fn get(&self, _: &str) -> Option<Anchor> {
        Option::None
    }
}
const SRC: &str = "flowchart TD\n  a[\"A label of some length\"] --> b[\"Another one\"]\n";

fn width_of(svg: &str) -> f64 {
    let at = svg.find("viewBox=\"").expect("a viewBox");
    let v = &svg[at + 9..];
    let v = &v[..v.find('"').expect("a closing quote")];
    v.split_whitespace()
        .nth(2)
        .expect("a width")
        .parse()
        .expect("a number")
}

#[test]
fn the_callers_own_measurement_is_what_the_labels_are_laid_out_by() {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (c, s) = (calls.clone(), seen.clone());
    // `Layout` is `#[non_exhaustive]`, so a caller sets its fields one by one.
    let mut layout = Layout::default();
    layout.font_family = Some("Noto Sans".into());
    let options = Options {
        // Twice as wide as a renderer's own estimate would be anywhere near.
        measure: Some(Measure::new(move |t: Text<'_>| {
            c.fetch_add(1, Ordering::Relaxed);
            s.lock().expect("the lock").push((
                t.text.to_string(),
                t.family.map(str::to_string),
                t.size,
            ));
            t.text.chars().count() as f64 * t.size
        })),
        layout,
        ..Options::default()
    };
    let wide = render(SRC, &None_, &options).expect("renders");
    let plain = render(SRC, &None_, &Options::default()).expect("renders");

    assert!(
        calls.load(Ordering::Relaxed) > 0,
        "the measurement hook was never called"
    );
    let seen = seen.lock().expect("the lock").clone();
    assert!(
        seen.iter()
            .any(|(t, _, _)| t.contains("A label of some length")),
        "the label text never reached the hook: {seen:?}"
    );
    assert!(
        seen.iter()
            .all(|(_, f, _)| f.as_deref() == Some("Noto Sans")),
        "the family asked for did not reach the hook: {seen:?}"
    );
    assert!(
        seen.iter().all(|(_, _, size)| *size > 0.0),
        "a run was measured at no size: {seen:?}"
    );
    assert!(
        width_of(&wide.svg) > width_of(&plain.svg),
        "measuring every label wider drew no wider: {} vs {}",
        width_of(&wide.svg),
        width_of(&plain.svg)
    );
    // Measured this wide the label wraps, and each line becomes its own
    // `<text>`.
    for word in ["label", "length", "Another"] {
        assert!(wide.svg.contains(word), "{word:?} is not drawn");
    }
}

/// An allowance has to reach the measurement, not the wrap width, or a label
/// shorter than that width gains nothing.
#[test]
fn slack_widens_what_is_drawn_around_a_short_label() {
    let slack = |s: f32| {
        let mut layout = Layout::default();
        layout.label_slack = s;
        render(
            // Short: nowhere near the 180 px wrap width.
            "flowchart TD\n  a[\"Yes\"] -- \"no\" --> b[\"Maybe\"]\n",
            &None_,
            &Options {
                layout,
                ..Options::default()
            },
        )
        .expect("renders")
        .svg
    };
    // Every rect's width, which is the boxes and the edge-label plates.
    let rects = |svg: &str| -> Vec<f64> {
        svg.split("<rect")
            .skip(1)
            .filter_map(|f| {
                let at = f.find(" width=\"")?;
                let rest = &f[at + 8..];
                rest[..rest.find('"')?].parse().ok()
            })
            .collect()
    };
    let plain = render(
        "flowchart TD\n  a[\"Yes\"] -- \"no\" --> b[\"Maybe\"]\n",
        &None_,
        &Options::default(),
    )
    .expect("renders")
    .svg;

    // 1.0 asks for nothing and must change nothing.
    assert_eq!(
        rects(&slack(1.0)),
        rects(&plain),
        "a slack of 1.0 moved the geometry"
    );
    assert_eq!(width_of(&slack(1.0)), width_of(&plain));

    // 1.3 widens them, short labels included.
    let roomy = slack(1.3);
    let (before, after) = (rects(&plain), rects(&roomy));
    assert_eq!(before.len(), after.len(), "a different set of shapes");
    assert!(
        before.iter().zip(&after).all(|(b, a)| a >= b),
        "something got narrower: {before:?} -> {after:?}"
    );
    assert!(
        before.iter().zip(&after).any(|(b, a)| a > b),
        "nothing was given any room: {before:?} -> {after:?}"
    );
    // The diagram's own width is not asserted: the extra room can go inside
    // the shapes without the picture growing.
}
