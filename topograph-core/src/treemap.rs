//! Squarified cushion treemap geometry (Phase 8) and vertex emission for the
//! GPU renderer (Phase 9). All layout and lighting math lives here, per the
//! spec; the GUI crate only packs the emitted vertices into a
//! `QSGGeometryNode`.
//!
//! The cushion lighting equation (van Wijk's cushion treemaps) is evaluated
//! per vertex in Rust on an adaptively subdivided grid: every ancestor
//! rectangle contributes a product of two normalized parabolas, the surface
//! normal is dotted against a fixed light vector, and ambient plus clamped
//! diffuse yield the shade. Grid density is chosen from each tile's pixel
//! size, so total vertex count is bounded by the screen area, not the tree
//! size.

use crate::{FileTree, NodeFlags, NodeId};

/// One laid-out rectangle, in item-local pixel coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreemapRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub node_id: NodeId,
    pub depth: u32,
}

/// The emission format consumed by the renderer: position plus a straight
/// RGBA8 color, the exact attribute set of Qt's colored-point geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreemapVertex {
    pub x: f32,
    pub y: f32,
    pub color: [u8; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct TreemapOptions {
    /// Gap between a directory's rectangle and its children, in pixels.
    /// Settable at runtime; this is the "dynamic padding" that visualizes
    /// hierarchy depth.
    pub padding: f32,
    /// Tiles whose calculated area is smaller than this square (in pixels)
    /// are culled: neither laid out nor recursed into.
    pub min_tile: f32,
    /// Cushion ridge height; 0 disables shading (flat tiles).
    pub cushion_height: f32,
    /// Ridge falloff per level: the enclosing canvas (depth 0) carries the
    /// full height and each deeper level's ridge is `falloff` times
    /// shorter, so the outermost boundaries shade strongest and deeper
    /// levels fade (van Wijk's convention). 1.0 makes every level equal.
    pub ridge_falloff: f32,
    /// Ambient light contribution, 0..=1.
    pub ambient: f32,
    /// Diffuse light contribution, 0..=1.
    pub diffuse: f32,
    /// In-plane light direction as an angle in radians (screen coordinates,
    /// y down). The default points toward the top-left.
    pub light_angle: f32,
}

impl Default for TreemapOptions {
    fn default() -> Self {
        Self {
            padding: 3.0,
            min_tile: 3.0,
            cushion_height: 0.7,
            ridge_falloff: 0.6,
            ambient: 0.55,
            diffuse: 0.55,
            // 5*pi/4: (cos, sin) = (-0.707, -0.707), toward the top-left.
            light_angle: 5.0 * std::f32::consts::PI / 4.0,
        }
    }
}

impl TreemapOptions {
    /// Unit light vector from `light_angle` plus a fixed vertical component.
    fn light_vector(&self) -> (f64, f64, f64) {
        let a = self.light_angle as f64;
        let (sx, sy) = (a.cos(), a.sin());
        let sz = 0.75;
        let len = (sx * sx + sy * sy + sz * sz).sqrt();
        (sx / len, sy / len, sz / len)
    }
}

/// Kanagawa Dragon anchor colors cycled by directory depth; files render in
/// a dimmed foreground tone. Phase 13 formalizes category mapping.
const DIR_PALETTE: [[u8; 3]; 6] = [
    [0x8e, 0xa4, 0xa2], // aqua
    [0x8b, 0xa4, 0xb0], // blue
    [0x87, 0xa9, 0x87], // green
    [0xc4, 0xb2, 0x8a], // yellow
    [0xb6, 0x92, 0x7b], // orange
    [0xc4, 0x74, 0x6e], // red
];
const FILE_COLOR: [u8; 3] = [0x7d, 0x7a, 0x75];

/// Target cushion-grid cell edge in pixels. Tiles are subdivided into cells
/// of about this size (at least one cell, at most 28 per axis), which bounds
/// total vertex count by screen area rather than tree size.
const GRID_CELL_PX: f64 = 12.0;
const GRID_MAX_CELLS: f64 = 28.0;

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// One cushion ridge: the ancestor rectangle that produced it plus its
/// height contribution.
#[derive(Clone, Copy)]
struct Ridge {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    height: f64,
}

/// Walker state: `layout` collects rectangles only, `build_vertices` emits
/// the shaded triangle grid. Both run the same recursion so the geometry and
/// the shading can never diverge.
struct Walker<'a> {
    tree: &'a FileTree,
    options: &'a TreemapOptions,
    rects: Option<&'a mut Vec<TreemapRect>>,
    vertices: Option<&'a mut Vec<TreemapVertex>>,
}

/// Lays out the subtree under `root` inside `width` x `height` and returns
/// the flat rectangle buffer. Children are sorted by size descending before
/// row packing; zero-size and sub-threshold nodes are culled; a directory's
/// children are laid inside its rectangle inset by `options.padding`.
pub fn layout(
    tree: &FileTree,
    root: NodeId,
    width: f32,
    height: f32,
    options: &TreemapOptions,
) -> Vec<TreemapRect> {
    let mut rects = Vec::new();
    walk_root(
        tree,
        root,
        width,
        height,
        options,
        Walker {
            tree,
            options,
            rects: Some(&mut rects),
            vertices: None,
        },
    );
    rects
}

/// Builds the renderable vertex buffer for the same layout: every laid
/// rectangle becomes a grid of cushion-shaded triangles.
pub fn build_vertices(
    tree: &FileTree,
    root: NodeId,
    width: f32,
    height: f32,
    options: &TreemapOptions,
) -> Vec<TreemapVertex> {
    build_vertices_counted(tree, root, width, height, options).0
}

/// `build_vertices` plus the laid-tile count, from a single walk: the UI
/// counter wants tiles (laid rectangles), not vertices (several triangles
/// each).
pub fn build_vertices_counted(
    tree: &FileTree,
    root: NodeId,
    width: f32,
    height: f32,
    options: &TreemapOptions,
) -> (Vec<TreemapVertex>, usize) {
    let mut vertices = Vec::new();
    let mut rects = Vec::new();
    walk_root(
        tree,
        root,
        width,
        height,
        options,
        Walker {
            tree,
            options,
            rects: Some(&mut rects),
            vertices: Some(&mut vertices),
        },
    );
    (vertices, rects.len())
}

fn walk_root(
    _tree: &FileTree,
    root: NodeId,
    width: f32,
    height: f32,
    options: &TreemapOptions,
    mut walker: Walker,
) {
    if width < 1.0 || height < 1.0 {
        return;
    }
    let canvas = Rect {
        x: 0.0,
        y: 0.0,
        w: width as f64,
        h: height as f64,
    };
    // The canvas is the outermost cushion ridge; the root's own rectangle is
    // never drawn, matching the tree view where the root is the container.
    let ridges = vec![Ridge {
        x0: canvas.x,
        x1: canvas.x + canvas.w,
        y0: canvas.y,
        y1: canvas.y + canvas.h,
        height: ridge_height(options, 0),
    }];
    walk_children(
        root,
        inset(canvas, options.padding as f64),
        &ridges,
        1,
        &mut walker,
    );
}

/// Ridge height for a rectangle at `depth`: the enclosing canvas is depth 0
/// with the full height, and each deeper level's ridge is `ridge_falloff`
/// times shorter, so enclosing boundaries shade strongest while a tile's own
/// ridges fade as the hierarchy deepens.
fn ridge_height(options: &TreemapOptions, depth: u32) -> f64 {
    options.cushion_height as f64 * (options.ridge_falloff as f64).powi(depth as i32)
}

fn walk_children(parent: NodeId, avail: Rect, ridges: &[Ridge], depth: u32, walker: &mut Walker) {
    for (child, rect) in place_children(walker.tree, parent, avail, walker.options) {
        let data = walker
            .tree
            .get_data(child)
            .expect("placed ids reference live nodes");
        let is_dir = data.flags.contains(NodeFlags::IS_DIRECTORY);
        let base = if is_dir {
            DIR_PALETTE[(depth - 1) as usize % DIR_PALETTE.len()]
        } else {
            FILE_COLOR
        };

        if let Some(rects) = walker.rects.as_mut() {
            rects.push(TreemapRect {
                x: rect.x as f32,
                y: rect.y as f32,
                w: rect.w as f32,
                h: rect.h as f32,
                node_id: child,
                depth,
            });
        }

        // The tile's own rectangle joins the ancestor ridge stack at its
        // depth; its children shade against both.
        let mut tile_ridges: Vec<Ridge> = ridges.to_vec();
        tile_ridges.push(Ridge {
            x0: rect.x,
            x1: rect.x + rect.w,
            y0: rect.y,
            y1: rect.y + rect.h,
            height: ridge_height(walker.options, depth),
        });

        if let Some(vertices) = walker.vertices.as_mut() {
            emit_tile(rect, base, &tile_ridges, walker.options, vertices);
        }

        if is_dir {
            walk_children(
                child,
                inset(rect, walker.options.padding as f64),
                &tile_ridges,
                depth + 1,
                walker,
            );
        }
    }
}

/// Collects a parent's layable children (size > 0), sorts them by size
/// descending, culls below-threshold tiles, and runs the squarified row
/// packing over `avail`, returning each node with its rectangle.
fn place_children(
    tree: &FileTree,
    parent: NodeId,
    avail: Rect,
    options: &TreemapOptions,
) -> Vec<(NodeId, Rect)> {
    let items: Vec<(NodeId, f64)> = tree
        .get_children(parent)
        .filter_map(|child| {
            let size = tree.get_data(child).map(|d| d.size)?;
            (size > 0).then_some((child, size as f64))
        })
        .collect();
    if items.is_empty() || avail.w < 1.0 || avail.h < 1.0 {
        return Vec::new();
    }

    let total: f64 = items.iter().map(|(_, s)| s).sum();
    let scale = avail.w * avail.h / total;
    // Cull below-threshold tiles before sorting and packing: culling is
    // order-independent and scale is fixed, so a wide directory full of
    // sub-threshold children never pays for their sort. Culled mass is not
    // redistributed (the sliver stays background rather than being hidden
    // inside a neighbor). Areas from here on are in pixels squared: each
    // node's share of the available area.
    let min_area = options.min_tile as f64 * options.min_tile as f64;
    let mut items: Vec<(NodeId, f64)> = items
        .into_iter()
        .map(|(n, s)| (n, s * scale))
        .filter(|&(_, a)| a >= min_area)
        .collect();
    if items.is_empty() {
        return Vec::new();
    }

    items.sort_by(|a, b| b.1.total_cmp(&a.1));

    let mut placed: Vec<(NodeId, Rect)> = Vec::with_capacity(items.len());
    squarify(avail, &items, &mut placed);
    placed
}

/// Bruls/Huizing/van Wijk squarified packing: rows of items are laid along
/// the shorter side of `rect` while the worst aspect ratio in the row keeps
/// improving, then the remaining rectangle recurses. `items` must already be
/// sorted by area descending, with areas in pixels squared.
fn squarify(rect: Rect, items: &[(NodeId, f64)], out: &mut Vec<(NodeId, Rect)>) {
    if items.is_empty() || rect.w < 1.0 || rect.h < 1.0 {
        return;
    }

    let side = rect.w.min(rect.h); // the row runs along the shorter side
    let mut split = 0;
    let mut best = f64::INFINITY;
    for i in 0..items.len() {
        let worst = worst_ratio(&items[..=i], side);
        if worst <= best {
            best = worst;
            split = i + 1;
        } else {
            break;
        }
    }
    let (row, rest) = items.split_at(split);

    let row_area: f64 = row.iter().map(|(_, s)| s).sum();
    let thickness = row_area / side;

    let mut cursor = 0.0; // position along the row's run
    for &(node, area) in row {
        let run = area / row_area * side; // extent along the shorter side
        let r = if rect.w >= rect.h {
            // Vertical strip at the left edge; items stack along y.
            Rect {
                x: rect.x,
                y: rect.y + cursor,
                w: thickness,
                h: run,
            }
        } else {
            // Horizontal strip at the top; items run along x.
            Rect {
                x: rect.x + cursor,
                y: rect.y,
                w: run,
                h: thickness,
            }
        };
        out.push((node, r));
        cursor += run;
    }

    let remaining = if rect.w >= rect.h {
        Rect {
            x: rect.x + thickness,
            y: rect.y,
            w: rect.w - thickness,
            h: rect.h,
        }
    } else {
        Rect {
            x: rect.x,
            y: rect.y + thickness,
            w: rect.w,
            h: rect.h - thickness,
        }
    };
    squarify(remaining, rest, out);
}

/// Worst (largest) aspect ratio among a candidate row laid against a side of
/// length `side`: max over items of max(w^2*s/S^2, S^2/(w^2*s)), square
/// rooted back into an aspect ratio.
fn worst_ratio(row: &[(NodeId, f64)], side: f64) -> f64 {
    let total: f64 = row.iter().map(|(_, s)| s).sum();
    let mut worst = 0.0f64;
    for &(_, s) in row {
        let short = side * side * s / (total * total);
        worst = worst.max(short.max(1.0 / short).sqrt());
    }
    worst
}

fn inset(rect: Rect, pad: f64) -> Rect {
    Rect {
        x: rect.x + pad,
        y: rect.y + pad,
        w: (rect.w - 2.0 * pad).max(0.0),
        h: (rect.h - 2.0 * pad).max(0.0),
    }
}

/// Normalized parabola over [a, b]: 0 at the edges, 1 in the middle.
fn bump(u: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return 0.0;
    }
    4.0 * (u - a) * (b - u) / ((b - a) * (b - a))
}

/// Derivative of `bump` with respect to u.
fn bump_deriv(u: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return 0.0;
    }
    4.0 * (a + b - 2.0 * u) / ((b - a) * (b - a))
}

/// Cushion surface slopes at (x, y): every ridge contributes
/// `h * bump'(x) * bump(y)` to dz/dx and `h * bump(x) * bump'(y)` to dz/dy.
fn cushion_slopes(x: f64, y: f64, ridges: &[Ridge]) -> (f64, f64) {
    let mut dz_dx = 0.0;
    let mut dz_dy = 0.0;
    for r in ridges {
        let bx = bump(x, r.x0, r.x1);
        let by = bump(y, r.y0, r.y1);
        dz_dx += r.height * bump_deriv(x, r.x0, r.x1) * by;
        dz_dy += r.height * bx * bump_deriv(y, r.y0, r.y1);
    }
    (dz_dx, dz_dy)
}

/// Shades one tile: subdivides into a grid sized by the tile's pixel
/// dimensions, evaluates the cushion lighting at each grid vertex once, and
/// emits two triangles per cell referencing those colors.
fn emit_tile(
    rect: Rect,
    base: [u8; 3],
    ridges: &[Ridge],
    options: &TreemapOptions,
    out: &mut Vec<TreemapVertex>,
) {
    let gx = (rect.w / GRID_CELL_PX).ceil().clamp(1.0, GRID_MAX_CELLS) as usize;
    let gy = (rect.h / GRID_CELL_PX).ceil().clamp(1.0, GRID_MAX_CELLS) as usize;

    let light = options.light_vector();

    let point = |ix: usize, iy: usize| -> (f64, f64) {
        (
            rect.x + rect.w * ix as f64 / gx as f64,
            rect.y + rect.h * iy as f64 / gy as f64,
        )
    };
    let mut colors: Vec<[u8; 4]> = Vec::with_capacity((gx + 1) * (gy + 1));
    for iy in 0..=gy {
        for ix in 0..=gx {
            let (x, y) = point(ix, iy);
            let (dz_dx, dz_dy) = cushion_slopes(x, y, ridges);
            colors.push(shade_point(base, dz_dx, dz_dy, light, options));
        }
    }
    let color_at = |ix: usize, iy: usize| -> [u8; 4] { colors[iy * (gx + 1) + ix] };
    let push = |(cx, cy): (usize, usize), out: &mut Vec<TreemapVertex>| {
        let (x, y) = point(cx, cy);
        out.push(TreemapVertex {
            x: x as f32,
            y: y as f32,
            color: color_at(cx, cy),
        });
    };

    for iy in 0..gy {
        for ix in 0..gx {
            let corners = [(ix, iy), (ix + 1, iy), (ix, iy + 1), (ix + 1, iy + 1)];
            // Two triangles: (00, 10, 01) and (10, 11, 01).
            push(corners[0], out);
            push(corners[1], out);
            push(corners[2], out);
            push(corners[1], out);
            push(corners[3], out);
            push(corners[2], out);
        }
    }
}

/// The cushion lighting equation: surface normal from the accumulated
/// slopes, dot product against the fixed light vector, ambient plus clamped
/// diffuse, applied per channel to the base color.
fn shade_point(
    base: [u8; 3],
    dz_dx: f64,
    dz_dy: f64,
    light: (f64, f64, f64),
    options: &TreemapOptions,
) -> [u8; 4] {
    let nx = -dz_dx;
    let ny = -dz_dy;
    let len = (nx * nx + ny * ny + 1.0).sqrt();
    let dot = ((nx * light.0 + ny * light.1 + light.2) / len).clamp(0.0, 1.0);
    let lit = (options.ambient as f64 + options.diffuse as f64 * dot).clamp(0.0, 1.2);
    [
        (base[0] as f64 * lit).clamp(0.0, 255.0) as u8,
        (base[1] as f64 * lit).clamp(0.0, 255.0) as u8,
        (base[2] as f64 * lit).clamp(0.0, 255.0) as u8,
        255,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeData;
    use std::time::Instant;

    fn dir(tree: &mut FileTree, parent: NodeId, name: &str) -> NodeId {
        tree.add_child(
            parent,
            NodeData::new(name, 0, 0, 0, NodeFlags::IS_DIRECTORY),
        )
    }

    fn file(tree: &mut FileTree, parent: NodeId, name: &str, size: u64) -> NodeId {
        tree.add_child(
            parent,
            NodeData::new(name, size, size, 0, NodeFlags::empty()),
        )
    }

    fn aspect(r: TreemapRect) -> f64 {
        (r.w as f64 / r.h as f64).max(r.h as f64 / r.w as f64)
    }

    #[test]
    fn single_file_fills_the_canvas() {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        file(&mut tree, root, "only", 100);
        tree.aggregate_sizes();

        let opts = TreemapOptions::default();
        let rects = layout(&tree, root, 400.0, 300.0, &opts);
        assert_eq!(rects.len(), 1);
        // Inside the canvas minus the root padding.
        assert_eq!(rects[0].w as f64, 400.0 - 2.0 * opts.padding as f64);
        assert_eq!(rects[0].h as f64, 300.0 - 2.0 * opts.padding as f64);
        assert_eq!(rects[0].x as f64, opts.padding as f64);
        assert_eq!(rects[0].depth, 1);
    }

    #[test]
    fn aspect_ratios_stay_bounded_on_skewed_input() {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        let sizes = [60, 25, 10, 3, 1, 1];
        for (i, s) in sizes.iter().enumerate() {
            file(&mut tree, root, &format!("f{i}"), *s * 1000);
        }
        tree.aggregate_sizes();

        let rects = layout(&tree, root, 800.0, 600.0, &TreemapOptions::default());
        assert_eq!(rects.len(), sizes.len());
        for r in &rects {
            assert!(
                aspect(*r) <= 5.0,
                "aspect {} for rect {r:?} exceeds the squarified bound",
                aspect(*r)
            );
        }
    }

    #[test]
    fn children_partition_the_available_area() {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        for (i, s) in [7, 5, 4, 2].iter().enumerate() {
            file(&mut tree, root, &format!("f{i}"), *s * 512);
        }
        tree.aggregate_sizes();

        let opts = TreemapOptions::default();
        let rects = layout(&tree, root, 500.0, 400.0, &opts);
        let area: f64 = rects.iter().map(|r| r.w as f64 * r.h as f64).sum();
        let avail = (500.0 - 2.0 * opts.padding as f64) * (400.0 - 2.0 * opts.padding as f64);
        assert!(
            // The rects store f32 dimensions, so the sum carries sub-pixel
            // rounding; anything under half a pixel squared is an exact tiling.
            (area - avail).abs() < 0.5,
            "area {area} vs available {avail}"
        );

        // Siblings cover the extent without overlap: the bounding box of the
        // tiles equals their total area.
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for r in &rects {
            x0 = x0.min(r.x as f64);
            y0 = y0.min(r.y as f64);
            x1 = x1.max(r.x as f64 + r.w as f64);
            y1 = y1.max(r.y as f64 + r.h as f64);
        }
        assert!((x1 - x0) * (y1 - y0) - area < 1.0);
    }

    #[test]
    fn zero_size_and_subthreshold_children_are_culled() {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        let big = file(&mut tree, root, "big", 10_000_000);
        let zero = file(&mut tree, root, "zero", 0);
        let speck = file(&mut tree, root, "speck", 1);
        tree.aggregate_sizes();

        let rects = layout(&tree, root, 500.0, 400.0, &TreemapOptions::default());
        let ids: Vec<NodeId> = rects.iter().map(|r| r.node_id).collect();
        assert!(ids.contains(&big));
        assert!(!ids.contains(&zero), "zero-size files lay no tile");
        assert!(!ids.contains(&speck), "sub-threshold tiles are culled");
    }

    #[test]
    fn directories_recurse_inside_their_padded_rectangle() {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        let a = dir(&mut tree, root, "a");
        let a1 = file(&mut tree, a, "a1", 400);
        let b = file(&mut tree, root, "b", 100);
        tree.aggregate_sizes();

        let opts = TreemapOptions::default();
        let rects = layout(&tree, root, 600.0, 400.0, &opts);
        let dir_rect = rects.iter().find(|r| r.node_id == a).unwrap();
        let child_rect = rects.iter().find(|r| r.node_id == a1).unwrap();
        let file_rect = rects.iter().find(|r| r.node_id == b).unwrap();

        assert_eq!(dir_rect.depth, 1);
        assert_eq!(child_rect.depth, 2);
        // The child sits inside the directory's rectangle inset by padding.
        let pad = opts.padding as f64;
        assert!(child_rect.x as f64 >= dir_rect.x as f64 + pad - 1e-6);
        assert!(child_rect.y as f64 >= dir_rect.y as f64 + pad - 1e-6);
        assert!(
            child_rect.x as f64 + child_rect.w as f64
                <= dir_rect.x as f64 + dir_rect.w as f64 - pad + 1e-6
        );
        assert!(
            child_rect.y as f64 + child_rect.h as f64
                <= dir_rect.y as f64 + dir_rect.h as f64 - pad + 1e-6
        );
        assert_eq!(file_rect.depth, 1);
    }

    #[test]
    fn vertex_buffer_matches_the_layout_and_is_opaque() {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        let a = dir(&mut tree, root, "a");
        for i in 0..5 {
            file(&mut tree, a, &format!("a{i}"), 100 * (i + 1) as u64);
        }
        file(&mut tree, root, "b", 300);
        tree.aggregate_sizes();

        let opts = TreemapOptions::default();
        let rects = layout(&tree, root, 700.0, 500.0, &opts);
        let verts = build_vertices(&tree, root, 700.0, 500.0, &opts);

        assert!(!verts.is_empty());
        assert_eq!(verts.len() % 6, 0, "vertices come in triangle pairs");
        for v in &verts {
            assert_eq!(v.color[3], 255, "tiles render opaque");
            assert!(v.x >= 0.0 && v.x <= 700.0 && v.y >= 0.0 && v.y <= 500.0);
        }
        // Every laid rect emits at least one cell; both passes share the
        // walker, so they must agree on which tiles exist.
        assert!(verts.len() >= rects.len() * 6);
    }

    #[test]
    fn cushion_shading_varies_across_a_tile_and_zero_height_flattens() {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        file(&mut tree, root, "only", 50_000);
        tree.aggregate_sizes();

        let ridged = TreemapOptions {
            padding: 0.0,
            ..TreemapOptions::default()
        };
        let verts = build_vertices(&tree, root, 300.0, 300.0, &ridged);
        let luminance = |c: [u8; 4]| c[0] as f64 + c[1] as f64 + c[2] as f64;
        let max_l = verts
            .iter()
            .map(|v| luminance(v.color))
            .fold(0.0f64, f64::max);
        let min_l = verts
            .iter()
            .map(|v| luminance(v.color))
            .fold(f64::MAX, f64::min);
        assert!(
            max_l - min_l > 1.0,
            "cushion ridges must shade (max {max_l}, min {min_l})"
        );

        let flat_opts = TreemapOptions {
            padding: 0.0,
            cushion_height: 0.0,
            ..TreemapOptions::default()
        };
        let flat = build_vertices(&tree, root, 300.0, 300.0, &flat_opts);
        let flat_max = flat
            .iter()
            .map(|v| luminance(v.color))
            .fold(0.0f64, f64::max);
        let flat_min = flat
            .iter()
            .map(|v| luminance(v.color))
            .fold(f64::MAX, f64::min);
        assert!(
            (flat_max - flat_min).abs() < 1e-9,
            "height 0 disables shading"
        );
    }

    #[test]
    fn layout_of_a_million_node_tree_is_logged_not_gated() {
        // Same shape as the arena benchmark: 100 nested directories with
        // 10k files each. Duration is printed for the release-mode
        // observation recorded in the roadmap; debug and CI machines vary,
        // so the bound is not asserted here.
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("r", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        let mut parent = root;
        for _ in 0..100 {
            let d = dir(&mut tree, parent, "d");
            for i in 0..10_000 {
                file(&mut tree, d, &format!("f{i}"), 1024);
            }
            parent = d;
        }
        tree.aggregate_sizes();

        let opts = TreemapOptions::default();
        let start = Instant::now();
        let rects = layout(&tree, root, 1024.0, 768.0, &opts);
        let layout_time = start.elapsed();
        let start = Instant::now();
        let verts = build_vertices(&tree, root, 1024.0, 768.0, &opts);
        let vertex_time = start.elapsed();
        println!(
            "1M-node treemap: {} rects in {:?}, {} vertices in {:?}",
            rects.len(),
            layout_time,
            verts.len(),
            vertex_time
        );
        assert!(!rects.is_empty());
    }
}
