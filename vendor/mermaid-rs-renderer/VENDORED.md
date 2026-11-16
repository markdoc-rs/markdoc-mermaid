# mermaid-rs-renderer, vendored

This directory is [mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer)
0.3.1, the Mermaid renderer `markdoc-mermaid` draws with, carried here with the fixes
below. It was a crates.io dependency until markdoc-mermaid 0.3.0.

- Source: the crates.io package `mermaid-rs-renderer-0.3.1` (checksum
  `6cb5b469222ce6b9896edf0a969cf68464f5a028ef394fd6643f00bd943df117`), which is the
  repository at commit `2f993bd79a55235eb59a34d807852276ba25bea7` — its tag `v0.3.1`, and
  the `sha1` in the package's own `.cargo_vcs_info.json`. Kept: `src/`, `tests/`,
  `benches/`, `examples/`, `LICENSE`, `README.md`, `CHANGELOG.md`, `clippy.toml`,
  `.gitignore`, `flake.nix`, `Cargo.toml` (the package's normalised manifest, changed for
  publishing as below) and `Cargo.toml.orig` (the repository's). Left out: `src/main.rs`,
  `src/cli.rs` and `tests/cli_suite.rs` (change 1), the package's `Cargo.lock`, and the
  registry's bookkeeping files. Added: `docs/comparison_sources/` (change 2).
  mermaid-rs-renderer is MIT, as markdoc-mermaid is; `LICENSE` is kept unchanged, and the
  copyright in it is upstream's.
- Published to crates.io as `markdoc-mermaid-renderer`, because the published
  `markdoc-mermaid` gets its dependencies from there (packaging drops `path`, and
  `[patch]` does not travel). Its `Cargo.toml` differs from the package's in `name`,
  `description`, `readme` (this file) and `repository`, in
  `exclude = ["Cargo.toml.orig"]` (a name `cargo package` reserves), and in the features
  and dependencies dropped by change 1. The library is still `mermaid_rs_renderer`.
- `markdoc-mermaid`'s `Cargo.toml` takes it as `mermaid-rs-renderer = { package =
  "markdoc-mermaid-renderer", version = "0.3.1", path = "vendor/mermaid-rs-renderer" }`.
  It is published before `markdoc-mermaid`:
  `cargo publish --manifest-path vendor/mermaid-rs-renderer/Cargo.toml`. A change here is
  published as a new version of `markdoc-mermaid-renderer`, which `markdoc-mermaid` must
  then require.
- Its own test suite runs on its own, and is not part of markdoc-mermaid's `cargo test`:
  `cargo test --manifest-path vendor/mermaid-rs-renderer/Cargo.toml`.
  `.github/scripts/check.sh` runs it as its own step.

## A note on `cargo fmt`

The published source is not `rustfmt`-clean against current rustfmt, in files nothing here
has touched. `check.sh` therefore does not run `cargo fmt --check` over this directory, and
markdoc-mermaid's own `cargo fmt --check` does not reach it: a path dependency is not a
workspace member. The changes below were written to match the surrounding style.

## Changes

Each is a change to upstream's behaviour or its packaging, and each is marked in the source
with a `markdoc-mermaid:` comment saying why, so it is not mistaken for upstream's own code.

### Packaging

1. **No CLI and no PNG.** `src/main.rs`, `src/cli.rs` and `tests/cli_suite.rs` are not
   vendored, the `cli` and `png` features and the `clap`, `resvg` and `usvg` dependencies
   are gone from `Cargo.toml`, and the `#[cfg(feature = "png")]` items in `src/render.rs`
   and `src/lib.rs` (`write_output_png`, `parse_hex_color`, `primary_font`, two tests) are
   removed with them. markdoc-mermaid took the crate with `default-features = false` for
   exactly this; the code is now absent rather than switched off, so no feature can turn it
   back on. A diagram in a document is SVG, which keeps `<text>` as selectable text in
   print rather than a picture of text.

2. **`docs/comparison_sources/` restored**, so the test suite compiles. Upstream's
   `exclude` list drops `docs/`, but five unit tests in `src/layout/mod.rs` reach for those
   files with `include_str!`, so `cargo test` on the published package fails to build with
   13 errors before running anything. The five files are fetched from the repository at the
   pinned commit: `er_blog.mmd`, `flowchart_cycles.mmd`, `flowchart_dense.mmd`,
   `flowchart_opaque.mmd`, `flowchart_subgraph_dir.mmd`.

### Fixes

3. **A lone `-` is not a link** (`ARROW_CORE` in `src/parser.rs`). Upstream's arrow pattern
   had the core `[-.=ox]*[-=]+[-.=ox]*`, whose `[-=]+` matches a single character, so one
   hyphen read as an edge. Two consequences, both silent:

   - `detect_diagram_kind` tries flowchart edge syntax last, so every diagram header the
     renderer has no parser for — all of them `*-beta` — came back a flowchart of two boxes
     named after the halves of its own header. `cynefin-beta` drew "cynefin" → "beta".
     `eventmodeling` was the only one that errored, because it has no hyphen.
   - `ARROW_RE` and `split_edge_chain` cut hyphenated ids in half: `alpha-one --> beta-two`
     parsed as the four nodes `alpha`, `ne`, `beta`, `two`. Upstream masks *bracketed*
     labels against this already (`A[wi-fi]`), but an id is not bracketed, and a
     documentation anchor — markdoc-mermaid's whole input — is usually a hyphenated slug.

   Every mermaid link carries at least two of `-` or `=`, so the core now requires two:
   `[-.=ox]*[-=][-.=ox]*[-=][-.=ox]*`. `--`, `---`, `-->`, `-.->`, `==>`, `--x`, `--o` and
   the labelled forms all clear it. Counting is what does this rather than a length, because
   `alpha-one` offers `-o`, two characters that look like the tail of `--o`. The labelled
   patterns' `dash1`/`dash2` halves are left as upstream wrote them: `-. text .->` really
   does carry a single dash before its dot.

4. **An unrecognised header is named in the error** (`unknown_diagram_header`). With fix 3
   an unknown header reaches `bail!`, and "unknown or missing Mermaid diagram header" is a
   poor answer when there plainly is one and this renderer simply does not draw it. A first
   line that is a bare word is reported as `unsupported Mermaid diagram type: <header>`.

5. **Radar axis and curve labels** (`radar_label`). Mermaid names a radar axis and curve
   `id["Label"]`. Upstream ran `strip_quotes` over the whole token, which leaves the
   brackets and the id in place, so `axis a["Alpha"]` drew an axis labelled `a["Alpha"]`.

6. **Radar's title** (`graph.radar_title`, `DiagramData::Radar`). The parser skipped
   `title` without storing it, and `render_radar` emitted its title `<text>` element with
   the text hardcoded empty — the slot was there and nothing ever filled it. The title now
   travels from the parser to the renderer through a `DiagramData::Radar` variant, added
   for it.

7. **A block diagram's node line respects its labels**
   (`split_whitespace_outside_brackets`). A block lists its nodes space-separated, and
   upstream split the line with `split_whitespace`, which cuts a quoted label in half:
   `a["Heading for step"]` came out as four tokens and so four nodes, three of them
   fragments of the label, with `a` keeping its id as its text. One-word labels hid it.
   Bracket depth is tracked directly rather than taken from `mask_bracket_content`, which
   blanks a bracketed run to spaces — the very character this splits on.

8. **A radar diagram's legend stays inside the layout**
   (`compute_radar_layout`). `legend_base_x` is a fixed 593.75 of a 680-wide box, so a
   legend entry wider than 86 — a series label of about a dozen characters, which
   `benches/fixtures/expanded/radar_tech.mmd` has — reached past the right edge and broke
   the `rectangle exceeds layout bounds` invariant that `validate_layout_invariants` holds
   every other diagram to, which `tests/invariant_suite.rs` checks over every fixture in
   the repository. Clamping the node to the box is the whole fix, because `render_radar`
   lays its legend out again from its own constants: these nodes are what names them in the
   SVG and what the invariants read, so nothing visible moves. The violation is upstream's
   and predates this vendoring — it is reachable from the published crate, though not
   observable there, because its own `exclude` list stops the suite that catches it from
   building at all (change 2).

9. **Three clippy lints and a manifest warning.** `tests/aspect_fold_suite.rs` tripped
   `field_reassign_with_default` twice and `manual_range_contains` once under current
   clippy; `check.sh` runs the vendored crate's clippy with `-D warnings`, and upstream's
   `src/` is clean. `homepage` was dropped from `Cargo.toml` as redundant with
   `repository`.

### Known, not fixed

**Mermaid's `#...;` entity escapes are not decoded.** Nothing in `src/` reads `#quot;`,
`#35;` or any other of them; the only entity handling is `escape_xml` on the way out. So a
label carrying one shows it literally. markdoc-mermaid's `expand::quote` emits `#quot;` for
a `"` in a heading, because a bare `"` would close a bracketed label early, which means a
heading with a quotation mark in it is drawn with `#quot;` where the mark should be. It
predates this vendoring and is left alone here: a decoder has to know not to touch a label
that contains a literal `#`, and that is a change to every diagram kind at once rather than
one of the contained fixes above.

### Not a fix

10. **One test is `#[ignore]`d**: `layout::tests::cycle_fixture_subgraph_entry_aligns_with_spine`.
   It asserts that one edge of `flowchart_cycles.mmd` collapses to a single straight
   segment, and the node positions it rests on come from measured text, so the answer
   depends on which font the host resolved. It passes where DejaVu is the match (a Debian
   image with `fonts-dejavu-core`, 6 fonts) and fails on a host with a wider set (Fedora,
   514 fonts), where the edge takes a 16px dogleg. That is the environment, not the layout,
   and it is upstream's behaviour either way — this renderer's geometry is font-dependent
   throughout. Run it with `cargo test -- --ignored` on a DejaVu host.
