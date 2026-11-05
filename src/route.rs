//! Edges a reader can follow, for a top-down flowchart.
//!
//! The renderer places the boxes well and routes the edges badly. On the
//! troubleshooting trees this crate exists for, its router took the three
//! answers to one question out of the *sides* of the question's box, ran
//! them round a loop the size of the diagram, and brought two of them into
//! their answers sideways along the top border, so the arrowheads sat on the
//! boxes' top corners and one label ended up under the wrong box. Nothing in
//! its configuration changes that: with and without its grid router, its
//! objective pass and its port bias, 5 to 9 edges of each tree still end
//! sideways.
//!
//! A decision tree read top down wants one shape for every forward edge, and
//! it is the one people draw by hand: out of the bottom of the question, down
//! to a channel in the gap below it, across, and down into the *top* of the
//! answer, with the answer's words on that last leg.
//!
//! ```text
//!            ┌──────────┐
//!            │ question │
//!            └──┬──┬──┬─┘
//!       ┌───────┘  │  └───────┐      <- the channel
//!   ┌───┴───┐  ┌───┴───┐  ┌───┴───┐  <- each answer's label on its own leg
//!   ▼       ▼  ▼       ▼  ▼       ▼
//! ```
//!
//! So this pass keeps the renderer's boxes and replaces the route of every
//! edge that runs down the page with that shape, provided the new route
//! crosses no other box and its label overlaps no box and no other label.
//! An edge it cannot route that way, and an edge that runs back up the page
//! or along a rank (a loop back to an earlier question), keeps the route the
//! renderer gave it: a worse route is still better than a wrong one.

use std::collections::BTreeMap;

use mermaid_rs_renderer::{Direction, Layout};

/// Room kept between a route and a box it passes, and between labels.
const CLEAR: f32 = 4.0;
/// How far below its box an edge turns into the channel.
const TURN: f32 = 14.0;
/// The label plate's padding, as the renderer draws it (4.5 and 2.2).
const PAD_X: f32 = 5.0;
const PAD_Y: f32 = 3.0;
/// The arrowhead's length, which a label on the last leg must stay clear of.
const ARROW: f32 = 8.0;
/// A straight piece of a drawn edge.
type Segment = ((f32, f32), (f32, f32));
/// A rerouted edge: its index, its points and its label's centre.
type Rerouted = (usize, Vec<(f32, f32)>, Option<(f32, f32)>);

/// The least distance between two edges running down side by side.
const LANE: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Rect {
    fn grow(self, by: f32) -> Rect {
        Rect {
            x0: self.x0 - by,
            y0: self.y0 - by,
            x1: self.x1 + by,
            y1: self.y1 + by,
        }
    }

    fn overlaps(&self, other: &Rect) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    fn cx(&self) -> f32 {
        (self.x0 + self.x1) / 2.0
    }

    /// Whether an axis-parallel segment passes through the rectangle.
    fn hit(&self, a: (f32, f32), b: (f32, f32)) -> bool {
        let seg = Rect {
            x0: a.0.min(b.0),
            y0: a.1.min(b.1),
            x1: a.0.max(b.0),
            y1: a.1.max(b.1),
        };
        // A segment has no width: give it a hair so a vertical one inside a
        // box still counts.
        seg.grow(0.01).overlaps(self)
    }
}

/// Reroute the forward edges of a top-down flowchart. Anything else, and any
/// edge the new shape does not fit, is left exactly as the renderer laid it
/// out.
pub(crate) fn tidy(layout: &mut Layout, direction: Direction) {
    if direction != Direction::TopDown {
        return;
    }
    let boxes: BTreeMap<String, Rect> = layout
        .nodes
        .values()
        .filter(|n| !n.hidden)
        .map(|n| {
            (
                n.id.clone(),
                Rect {
                    x0: n.x,
                    y0: n.y,
                    x1: n.x + n.width,
                    y1: n.y + n.height,
                },
            )
        })
        .collect();

    // The forward edges, and where on its box each one leaves and arrives:
    // spread along the bottom of the source in the order of the targets, and
    // along the top of the target in the order of the sources, so no two
    // routes start or end on one point and none of them cross at a box.
    let forward: Vec<usize> = layout
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            e.from != e.to
                && match (boxes.get(&e.from), boxes.get(&e.to)) {
                    (Some(s), Some(t)) => t.y0 > s.y1 + 2.0 * TURN,
                    _ => false,
                }
        })
        .map(|(i, _)| i)
        .collect();
    let mut out_ports: BTreeMap<usize, f32> = BTreeMap::new();
    let mut in_ports: BTreeMap<usize, f32> = BTreeMap::new();
    for (id, rect) in &boxes {
        let mut leaving: Vec<usize> = forward
            .iter()
            .copied()
            .filter(|&i| &layout.edges[i].from == id)
            .collect();
        leaving.sort_by(|&a, &b| {
            boxes[&layout.edges[a].to]
                .cx()
                .total_cmp(&boxes[&layout.edges[b].to].cx())
        });
        spread(rect, &leaving, &mut out_ports);
        let mut arriving: Vec<usize> = forward
            .iter()
            .copied()
            .filter(|&i| &layout.edges[i].to == id)
            .collect();
        arriving.sort_by(|&a, &b| {
            boxes[&layout.edges[a].from]
                .cx()
                .total_cmp(&boxes[&layout.edges[b].from].cx())
        });
        spread(rect, &arriving, &mut in_ports);
    }

    // The short edges first, so the answer right below a question gets the
    // place right above its box, and a long edge works round them.
    let mut order = forward.clone();
    order.sort_by(|&a, &b| {
        let span = |i: usize| boxes[&layout.edges[i].to].y0 - boxes[&layout.edges[i].from].y1;
        span(a).total_cmp(&span(b))
    });

    // An edge that cannot be rerouted keeps the renderer's route and label,
    // which every other edge must then keep clear of: so on a failure the
    // whole placement starts again with that edge left as it was.
    let mut kept: Vec<usize> = (0..layout.edges.len())
        .filter(|i| !forward.contains(i))
        .collect();
    let done = loop {
        let mut placed: Vec<Rect> = kept
            .iter()
            .filter_map(|&i| {
                let e = &layout.edges[i];
                Some(plate(e.label.as_ref()?, e.label_anchor?))
            })
            .collect();
        let mut lines: Vec<Segment> = kept
            .iter()
            .flat_map(|&i| {
                let p = &layout.edges[i].points;
                p.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>()
            })
            .collect();
        let mut done: Vec<Rerouted> = Vec::new();
        // Where the edges already routed run down: a new lane keeps apart
        // from them, or two edges merge into one line nobody can follow.
        let mut lanes: Vec<Rect> = Vec::new();
        let mut failed = None;
        for &i in order.iter().filter(|i| !kept.contains(i)) {
            let edge = &layout.edges[i];
            let s = boxes[&edge.from];
            let t = boxes[&edge.to];
            let others: Vec<Rect> = boxes
                .iter()
                .filter(|(id, _)| **id != edge.from && **id != edge.to)
                .map(|(_, r)| r.grow(CLEAR))
                .collect();
            let candidates = routes(
                s,
                t,
                out_ports[&i],
                in_ports[&i],
                &others,
                &lanes,
                layout.width,
            );
            // The first route its label fits on; one without a label takes
            // the first route there is.
            let found = candidates
                .into_iter()
                .find_map(|points| match edge.label.as_ref() {
                    None => Some((points, None)),
                    Some(block) => place(&points, block, &boxes, &placed, &lines)
                        .map(|anchor| (points, Some((anchor, plate(block, anchor))))),
                });
            let Some((points, label)) = found else {
                failed = Some(i);
                break;
            };
            let anchor = label.map(|(anchor, p)| {
                placed.push(p);
                anchor
            });
            lines.extend(points.windows(2).map(|w| (w[0], w[1])));
            lanes.extend(
                points
                    .windows(2)
                    .filter(|w| (w[0].0 - w[1].0).abs() < 0.5)
                    .map(|w| Rect {
                        x0: w[0].0 - LANE,
                        y0: w[0].1.min(w[1].1),
                        x1: w[0].0 + LANE,
                        y1: w[0].1.max(w[1].1),
                    }),
            );
            done.push((i, points, anchor));
        }
        match failed {
            Some(i) => kept.push(i),
            None => break done,
        }
    };
    for (i, points, anchor) in done {
        let edge = &mut layout.edges[i];
        edge.points = points;
        if anchor.is_some() {
            edge.label_anchor = anchor;
        }
    }
}

/// Ports along one side of `rect`, evenly spaced, one per edge.
fn spread(rect: &Rect, edges: &[usize], ports: &mut BTreeMap<usize, f32>) {
    let n = edges.len() as f32;
    let w = rect.x1 - rect.x0;
    for (k, &i) in edges.iter().enumerate() {
        let x = if edges.len() == 1 {
            rect.cx()
        } else {
            // The middle half of the side: well clear of the rounded corners.
            rect.x0 + w * (0.25 + 0.5 * (k as f32 + 0.5) / n)
        };
        ports.insert(i, x);
    }
}

/// Down, across, down: every shape that crosses no other box, the plainest
/// first. Bounded by `width`, the drawing's own: a lane outside it would be
/// cut off by the view box.
fn routes(
    s: Rect,
    t: Rect,
    sx: f32,
    tx: f32,
    others: &[Rect],
    taken: &[Rect],
    width: f32,
) -> Vec<Vec<(f32, f32)>> {
    let start = (sx, s.y1);
    let end = (tx, t.y0);
    // Crosses no box, and no long drop runs alongside another edge's.
    let clear = |pts: &[(f32, f32)]| {
        pts.windows(2).all(|w| {
            let long_drop = (w[0].0 - w[1].0).abs() < 0.5 && (w[1].1 - w[0].1).abs() > 2.0 * TURN;
            others.iter().all(|r| !r.hit(w[0], w[1]))
                && (!long_drop || taken.iter().all(|r| !r.hit(w[0], w[1])))
        })
    };
    let mut out: Vec<Vec<(f32, f32)>> = Vec::new();
    if (sx - tx).abs() < 0.5 {
        let pts = vec![start, (sx, t.y0)];
        if clear(&pts) {
            out.push(pts);
        }
    }
    // Two boxes nearly in line: one straight drop where both sides have
    // room for it, rather than a jog of a few units.
    if (sx - tx).abs() < 24.0 {
        let inside = |x: f32, r: &Rect| x >= r.x0 + 16.0 && x <= r.x1 - 16.0;
        for x in [tx, sx] {
            let pts = vec![(x, s.y1), (x, t.y0)];
            if inside(x, &s) && inside(x, &t) && clear(&pts) {
                out.push(pts);
            }
        }
    }
    // The channel just below the source first, so the long leg -- the one
    // that carries the label -- is the one into the answer; then the one
    // just above the target, whose long leg is the one out of the question.
    let below = s.y1 + TURN;
    let above = t.y0 - TURN;
    for y in [below, above, (s.y1 + t.y0) / 2.0] {
        let pts = vec![start, (sx, y), (tx, y), end];
        if clear(&pts) {
            out.push(pts);
        }
    }
    // Past a rank in between: down, across to a free lane, down the lane,
    // across, and down into the target.
    let mut lanes: Vec<f32> = Vec::new();
    let mut xs: Vec<(f32, f32)> = others
        .iter()
        .chain(taken)
        .filter(|r| r.y1 > below && r.y0 < above)
        .map(|r| (r.x0, r.x1))
        .collect();
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));
    for w in xs.windows(2) {
        if w[1].0 > w[0].1 {
            lanes.push((w[0].1 + w[1].0) / 2.0);
        }
    }
    // And either side of everything in the way.
    if let (Some(left), Some(right)) = (
        xs.iter().map(|x| x.0).reduce(f32::min),
        xs.iter().map(|x| x.1).reduce(f32::max),
    ) {
        lanes.push(left - 2.0 * CLEAR);
        lanes.push(right + 2.0 * CLEAR);
    }
    lanes.retain(|&x| x > CLEAR && x < width - CLEAR);
    let mid = (sx + tx) / 2.0;
    lanes.sort_by(|a, b| (a - mid).abs().total_cmp(&(b - mid).abs()));
    for lane in lanes {
        let pts = vec![
            start,
            (sx, below),
            (lane, below),
            (lane, above),
            (tx, above),
            end,
        ];
        if clear(&pts) {
            out.push(pts);
        }
    }
    out
}

/// The plate drawn behind a label centred on `anchor`.
fn plate(block: &mermaid_rs_renderer::layout::TextBlock, anchor: (f32, f32)) -> Rect {
    Rect {
        x0: anchor.0 - block.width / 2.0 - PAD_X,
        y0: anchor.1 - block.height / 2.0 - PAD_Y,
        x1: anchor.0 + block.width / 2.0 + PAD_X,
        y1: anchor.1 + block.height / 2.0 + PAD_Y,
    }
}

/// Where on the route its label goes: on the last vertical leg (the one into
/// the answer) if it fits there, else on the first, else across the middle
/// -- the first place that covers no box and no other label.
fn place(
    points: &[(f32, f32)],
    block: &mermaid_rs_renderer::layout::TextBlock,
    boxes: &BTreeMap<String, Rect>,
    placed: &[Rect],
    lines: &[Segment],
) -> Option<(f32, f32)> {
    let h = block.height + 2.0 * PAD_Y;
    let mut candidates: Vec<(f32, f32)> = Vec::new();
    let n = points.len();
    // Every label of one question's answers level with the others: the last
    // leg is measured from where a channel turn would be even when the
    // route is one straight drop.
    let floor = points[0].1 + TURN;
    // The vertical legs: the last one (into the answer) first, then the
    // others, longest first.
    let mut legs: Vec<usize> = (0..n - 1)
        .filter(|&k| (points[k].0 - points[k + 1].0).abs() < 0.5)
        .collect();
    legs.sort_by(|&a, &b| {
        let last = |k: usize| k == n - 2;
        let len = |k: usize| points[k + 1].1 - points[k].1;
        last(b).cmp(&last(a)).then(len(b).total_cmp(&len(a)))
    });
    for k in legs {
        let (a, b) = (points[k], points[k + 1]);
        // The last leg ends in the arrowhead, which the plate must not hide.
        let (top, bottom) = if k == n - 2 {
            (a.1.max(floor), b.1 - ARROW)
        } else {
            (a.1, b.1)
        };
        let room = bottom - top;
        if room < h + 2.0 * CLEAR {
            continue;
        }
        // The middle of a short leg; on a long one, just above the box it
        // leads to, then further up a step at a time.
        if room < 4.0 * h {
            candidates.push((a.0, top + room / 2.0));
        }
        let mut y = bottom - CLEAR - h / 2.0;
        while y - h / 2.0 >= top + CLEAR {
            candidates.push((a.0, y));
            y -= h / 2.0;
        }
    }
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        if (a.1 - b.1).abs() < 0.5 && (a.0 - b.0).abs() > block.width + 2.0 * PAD_X {
            candidates.push(((a.0 + b.0) / 2.0, a.1));
        }
    }
    let free = |c: (f32, f32), over_lines: bool| {
        let p = plate(block, c);
        boxes.values().all(|r| !r.grow(CLEAR).overlaps(&p))
            && placed.iter().all(|q| !q.grow(CLEAR).overlaps(&p))
            && (over_lines || lines.iter().all(|&(a, b)| !p.hit(a, b)))
    };
    // A plate hides what runs under it, so a place clear of every other
    // edge first; failing that, one that crosses an edge, as Mermaid's own
    // labels do.
    candidates
        .iter()
        .copied()
        .find(|&c| free(c, false))
        .or_else(|| candidates.iter().copied().find(|&c| free(c, true)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    #[test]
    fn a_route_goes_down_across_and_down_into_the_top() {
        let s = r(100.0, 0.0, 300.0, 50.0);
        let t = r(0.0, 150.0, 150.0, 200.0);
        let pts = routes(s, t, 200.0, 75.0, &[], &[], 400.0).remove(0);
        assert_eq!(pts.first(), Some(&(200.0, 50.0)));
        assert_eq!(pts.last(), Some(&(75.0, 150.0)));
        let n = pts.len();
        assert_eq!(pts[n - 2].0, pts[n - 1].0, "the last leg is vertical");
        assert!(pts[n - 2].1 < pts[n - 1].1, "and it runs down into the box");
    }

    #[test]
    fn a_box_in_the_way_moves_the_channel() {
        let s = r(100.0, 0.0, 300.0, 50.0);
        let t = r(0.0, 300.0, 150.0, 350.0);
        // Sits right under the turn below the source, across the way left.
        let blocker = r(40.0, 60.0, 180.0, 120.0).grow(CLEAR);
        let all = routes(s, t, 200.0, 75.0, &[blocker], &[], 400.0);
        assert!(!all.is_empty());
        for pts in all {
            for w in pts.windows(2) {
                assert!(!blocker.hit(w[0], w[1]), "{pts:?}");
            }
        }
    }

    #[test]
    fn no_clear_route_keeps_the_renderers() {
        let s = r(0.0, 0.0, 200.0, 50.0);
        let t = r(0.0, 300.0, 200.0, 350.0);
        // A wall across the whole gap.
        let wall = r(-10.0, 100.0, 410.0, 250.0);
        // A lane round it would leave the drawing.
        assert!(routes(s, t, 100.0, 50.0, &[wall], &[], 400.0).is_empty());
    }

    /// A decision tree of the shape that came out worst: one question with
    /// three answers, a chain, a two-way split, an answer reached from two
    /// questions, and a loop back up.
    const TREE: &str = "flowchart TD
  start[Not working] --> q1[What colour is the button when you press it?]
  q1 -- \"It turns green\" --> a[Do the lights keep wavering yellow?]
  q1 -- \"It turns red\" --> b[Can you charge the battery?]
  q1 -- \"It does not turn on\" --> c[Is the battery connected?]
  a -- \"Wavering yellow\" --> d[Did the computer start up?]
  d -- \"Button on\" --> e[Can you ping the computer?]
  d -- \"Button off\" --> f[Can you turn on the computer by hand?]
  e -- \"Ping succeeded\" --> g[Does it report power board errors?]
  f -- \"Computer turns on\" --> g
  g -- \"Power issue\" --> h[Can you ping the power board?]
  h -- \"No reply\" --> d
";

    fn laid_out(tidy_edges: bool) -> mermaid_rs_renderer::Layout {
        let parsed = mermaid_rs_renderer::parse_mermaid_strict(TREE).unwrap();
        let config = crate::Layout::default().to_config();
        let mut layout = mermaid_rs_renderer::compute_layout(
            &parsed.graph,
            &mermaid_rs_renderer::Theme::modern(),
            &config,
        );
        if tidy_edges {
            tidy(&mut layout, parsed.graph.direction);
        }
        layout
    }

    fn rect_of(layout: &mermaid_rs_renderer::Layout, id: &str) -> Rect {
        let n = &layout.nodes[id];
        r(n.x, n.y, n.x + n.width, n.y + n.height)
    }

    #[test]
    fn every_edge_down_the_page_ends_in_the_top_of_its_box() {
        let layout = laid_out(true);
        let mut forward = 0;
        for e in &layout.edges {
            let (s, t) = (rect_of(&layout, &e.from), rect_of(&layout, &e.to));
            if t.y0 <= s.y1 {
                continue; // the loop back up keeps the renderer's route
            }
            forward += 1;
            let n = e.points.len();
            let (a, b) = (e.points[n - 2], e.points[n - 1]);
            assert!(
                (a.0 - b.0).abs() < 0.5 && b.1 > a.1,
                "{} -> {}: {:?}",
                e.from,
                e.to,
                e.points
            );
            assert!(
                (b.1 - t.y0).abs() < 0.5,
                "{} -> {} stops short",
                e.from,
                e.to
            );
            assert!(
                b.0 > t.x0 + 8.0 && b.0 < t.x1 - 8.0,
                "{} -> {} at a corner",
                e.from,
                e.to
            );
            let (p0, p1) = (e.points[0], e.points[1]);
            assert!(
                (p0.1 - s.y1).abs() < 0.5 && p1.1 > p0.1,
                "{} -> {} leaves sideways",
                e.from,
                e.to
            );
        }
        assert_eq!(forward, 10);
    }

    #[test]
    fn no_label_sits_on_a_box_or_on_another_label() {
        let layout = laid_out(true);
        let plates: Vec<(String, Rect)> = layout
            .edges
            .iter()
            .filter_map(|e| Some((e.to.clone(), plate(e.label.as_ref()?, e.label_anchor?))))
            .collect();
        for (to, p) in &plates {
            for n in layout.nodes.values() {
                let b = r(n.x, n.y, n.x + n.width, n.y + n.height);
                assert!(!b.overlaps(p), "the label into {to} covers {}", n.id);
            }
        }
        for (i, (a, p)) in plates.iter().enumerate() {
            for (b, q) in &plates[i + 1..] {
                assert!(!p.overlaps(q), "the labels into {a} and {b} overlap");
            }
        }
    }

    #[test]
    fn a_label_sits_on_its_own_edge() {
        // Next to the line it names, not beside another one: a label that
        // drifts is how "It turns red" came to sit under the wrong box.
        let layout = laid_out(true);
        for e in &layout.edges {
            let (Some(block), Some(anchor)) = (e.label.as_ref(), e.label_anchor) else {
                continue;
            };
            if rect_of(&layout, &e.to).y0 <= rect_of(&layout, &e.from).y1 {
                continue; // the loop back up is the renderer's, label and all
            }
            let p = plate(block, anchor);
            assert!(
                e.points.windows(2).any(|w| p.hit(w[0], w[1])),
                "{} -> {}: label at {anchor:?}, route {:?}",
                e.from,
                e.to,
                e.points
            );
        }
    }

    #[test]
    fn off_leaves_the_renderers_routes() {
        let before = laid_out(false);
        let parsed = mermaid_rs_renderer::parse_mermaid_strict(TREE).unwrap();
        let mut lr = before.clone();
        // Not a top-down chart: nothing to do.
        tidy(&mut lr, mermaid_rs_renderer::Direction::LeftRight);
        assert_eq!(
            parsed.graph.direction,
            mermaid_rs_renderer::Direction::TopDown
        );
        for (x, y) in before.edges.iter().zip(&lr.edges) {
            assert_eq!(x.points, y.points);
        }
    }

    #[test]
    fn ports_spread_over_the_middle_of_a_side() {
        let rect = r(0.0, 0.0, 200.0, 50.0);
        let mut ports = BTreeMap::new();
        spread(&rect, &[7, 8, 9], &mut ports);
        let xs: Vec<f32> = [7, 8, 9].iter().map(|i| ports[i]).collect();
        assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
        assert!(xs[0] >= 50.0 && xs[2] <= 150.0, "{xs:?}");
        let mut one = BTreeMap::new();
        spread(&rect, &[1], &mut one);
        assert_eq!(one[&1], 100.0);
    }
}
