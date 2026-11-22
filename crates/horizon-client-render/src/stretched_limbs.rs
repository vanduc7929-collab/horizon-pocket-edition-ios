//! Stretched limbs geometry, skin-band UV tiling, cubic Hermite RMF path sweeping,
//! armor/held-item hiding, and expanded frustum culling (up to 64+ blocks).
//!
//! 1:1 Rust port of Grand Line 0.4.0 (`devilfruits`):
//! - `core.limb.{LimbUv, LimbBands, LimbFill, LimbPath}`
//! - `client.limb.{LimbSpec, LimbFrame, TexelBoxMesh, LimbRenderer, Limbs}`
//! - `mixin.client.{LimbArmorLayerMixin, LimbItemLayerMixin, LimbCullMixin}`
#![forbid(unsafe_code)]

use glam::{DVec3, Mat3, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::f32::consts::TAU;

/// Standard limb UV band height in classic Minecraft skin texels (`LimbUv.BAND`).
pub const LIMB_BAND_TEXELS: u32 = 4;
/// Standard unstretched limb length in classic Minecraft skin texels (`LimbUv.LENGTH`).
pub const LIMB_REST_LENGTH_PX: u32 = 12;
/// Maximum stretched limb length in model pixels (`LimbBands.MAX_LENGTH` = 1024 px = 64 blocks).
pub const MAX_LIMB_LENGTH_PX: f32 = 1024.0;
/// Maximum stretched limb reach in world blocks (`1024 / 16 = 64.0`).
pub const MAX_LIMB_REACH_BLOCKS: f64 = 64.0;
/// Squared camera distance below which an entity with active stretched limbs stays drawn
/// (`Limbs.keepsDrawn`: `128^2 = 16384.0`).
pub const KEEPS_DRAWN_DIST_SQ: f64 = 16384.0;
/// Swarm limb budget multiplier (`Limbs.SWARM_FACTOR` = 3x).
pub const SWARM_FACTOR: usize = 3;
/// Default normal limb budget (`CoreClientConfig.MAX_LIMBS` = 12).
pub const DEFAULT_MAX_LIMBS: usize = 12;
/// Maximum spline control points in [`LimbPath`] (`LimbPath.MAX_POINTS`).
pub const MAX_PATH_POINTS: usize = 16;
/// Maximum swept cross-section rings in [`TexelBoxMesh`] (`TexelBoxMesh.MAX_RINGS`).
pub const MAX_MESH_RINGS: usize = 2048;
/// Default swirl overlay tint (`LimbSpec.swirlColor = -1056964609 = 0xC0FFFFFF`).
pub const DEFAULT_SWIRL_COLOR: u32 = 0xC0FF_FFFF;

// ============================================================================
// LimbUv (1:1 port of `com.devilfruits.core.limb.LimbUv`)
// ============================================================================

/// Which limb is stretched (`LimbUv.Limb`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LimbKind {
    #[default]
    Arm,
    Leg,
}

/// Right or left limb (`LimbUv.Side`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LimbSide {
    #[default]
    Right,
    Left,
}

/// Player skin arm width model (`LimbUv.Model`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkinModel {
    #[default]
    Classic,
    Slim,
}

/// Base skin vs. outer sleeve/pants overlay (`LimbUv.Layer`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkinLayer {
    #[default]
    Skin,
    Overlay,
}

/// Four radial sides around a limb cuboid (`LimbUv.Face`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(usize)]
pub enum LimbFace {
    West = 0,
    North = 1,
    East = 2,
    South = 3,
}

/// Resolved UV and cuboid dimension layout for one limb layer (`LimbUv.Layout` 1:1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LimbUvLayout {
    pub tex_width: u32,
    pub tex_height: u32,
    pub scale: f32,
    pub u: u32,
    pub v: u32,
    pub width: u32,
    pub depth: u32,
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub pivot_z: f32,
    pub grow: f32,
    pub mirror: bool,
}

/// Namespace for skin UV resolution helpers (`LimbUv` 1:1).
pub struct LimbUv;

impl LimbUv {
    /// Returns `true` for legacy 64x32 (2:1 aspect) player skins (`LimbUv.legacy`).
    #[inline]
    pub const fn is_legacy(tex_width: u32, tex_height: u32) -> bool {
        tex_width == tex_height.saturating_mul(2)
    }

    /// Resolves exact classic/slim/legacy/HD skin UV layout (`LimbUv.resolve` 1:1).
    ///
    /// Returns `None` for `SkinLayer::Overlay` on a legacy 2:1 skin.
    pub fn resolve(
        model: SkinModel,
        side: LimbSide,
        limb: LimbKind,
        tex_width: u32,
        tex_height: u32,
        layer: SkinLayer,
    ) -> Option<LimbUvLayout> {
        let legacy = Self::is_legacy(tex_width, tex_height);
        if legacy && layer == SkinLayer::Overlay {
            return None;
        }

        let right = side == LimbSide::Right;
        let overlay = layer == SkinLayer::Overlay;
        let slim = model == SkinModel::Slim;
        let scale = tex_width as f32 / 64.0;
        let mut mirror = false;

        let (u, v) = match limb {
            LimbKind::Arm => {
                if !right && !legacy {
                    (if overlay { 48 } else { 32 }, 48)
                } else {
                    mirror = !right;
                    (40, if overlay { 32 } else { 16 })
                }
            }
            LimbKind::Leg => {
                if !right && !legacy {
                    (if overlay { 0 } else { 16 }, 48)
                } else {
                    mirror = !right;
                    (0, if overlay { 32 } else { 16 })
                }
            }
        };

        let width = if limb == LimbKind::Arm && slim { 3 } else { 4 };
        let (min_x, min_y, pivot_x, pivot_y) = match limb {
            LimbKind::Arm => (
                if right {
                    if slim { -2.0 } else { -3.0 }
                } else {
                    -1.0
                },
                -2.0,
                if right { -5.0 } else { 5.0 },
                if slim { 2.5 } else { 2.0 },
            ),
            LimbKind::Leg => (-2.0, 0.0, if right { -1.9 } else { 1.9 }, 12.0),
        };

        Some(LimbUvLayout {
            tex_width,
            tex_height,
            scale,
            u,
            v,
            width,
            depth: 4,
            min_x,
            min_y,
            min_z: -2.0,
            pivot_x,
            pivot_y,
            pivot_z: 0.0,
            grow: if overlay { 0.25 } else { 0.0 },
            mirror,
        })
    }
}

impl LimbUvLayout {
    /// Converts a classic-grid U texel coordinate to normalized `[0, 1]` texture U.
    #[inline]
    pub fn u_coord(&self, texel: f32) -> f32 {
        texel * self.scale / self.tex_width as f32
    }

    /// Converts a classic-grid V texel coordinate to normalized `[0, 1]` texture V.
    #[inline]
    pub fn v_coord(&self, texel: f32) -> f32 {
        texel * self.scale / self.tex_height as f32
    }

    #[inline]
    pub fn px(&self, texel: f32) -> f32 {
        texel * self.scale
    }

    #[inline]
    pub fn band_rows(&self) -> f32 {
        4.0 * self.scale
    }

    #[inline]
    pub fn side_top(&self) -> f32 {
        (self.v + self.depth) as f32
    }

    #[inline]
    pub fn centre_x(&self) -> f32 {
        self.min_x + self.width as f32 * 0.5
    }

    #[inline]
    pub fn centre_z(&self) -> f32 {
        self.min_z + self.depth as f32 * 0.5
    }

    /// Returns the classic-grid U texel coordinate for radial `face` (`0..4`) and `corner` (`0..2`).
    #[inline]
    pub fn side_u(&self, face: usize, corner: usize) -> f32 {
        let u = self.u;
        let w = self.width;
        let d = self.depth;
        let val = match face {
            0 => {
                if corner == 0 {
                    u + d
                } else {
                    u
                }
            }
            1 => {
                if corner == 0 {
                    u + d + w
                } else {
                    u + d
                }
            }
            2 => {
                if corner == 0 {
                    u + 2 * d + w
                } else {
                    u + d + w
                }
            }
            _ => {
                if corner == 0 {
                    u + 2 * d + 2 * w
                } else {
                    u + 2 * d + w
                }
            }
        };
        val as f32
    }

    /// Signed X corner sign (`-1` or `+1`) for radial `face` and `corner`.
    #[inline]
    pub const fn corner_x(face: usize, corner: usize) -> i32 {
        match face {
            0 => -1,
            1 => {
                if corner == 0 {
                    1
                } else {
                    -1
                }
            }
            2 => 1,
            _ => {
                if corner == 0 {
                    -1
                } else {
                    1
                }
            }
        }
    }

    /// Signed Z corner sign (`-1` or `+1`) for radial `face` and `corner`.
    #[inline]
    pub const fn corner_z(face: usize, corner: usize) -> i32 {
        match face {
            0 => {
                if corner == 0 {
                    -1
                } else {
                    1
                }
            }
            1 => -1,
            2 => {
                if corner == 0 {
                    1
                } else {
                    -1
                }
            }
            _ => 1,
        }
    }

    /// Returns the 4 cap vertices `[x_sign, z_sign, u_texel, v_texel]` for shoulder/hip (`top = true`)
    /// or fist/foot (`top = false`) end caps (`LimbUv.Layout.cap` 1:1).
    pub fn cap(&self, top: bool) -> [[f32; 4]; 4] {
        let a = (self.u + self.depth) as f32;
        let b = (self.u + self.depth + self.width) as f32;
        let c = (self.u + self.depth + 2 * self.width) as f32;
        let v0 = self.v as f32;
        let v1 = (self.v + self.depth) as f32;
        if top {
            [
                [1.0, 1.0, b, v0],
                [-1.0, 1.0, a, v0],
                [-1.0, -1.0, a, v1],
                [1.0, -1.0, b, v1],
            ]
        } else {
            [
                [1.0, -1.0, c, v1],
                [-1.0, -1.0, b, v1],
                [-1.0, 1.0, b, v0],
                [1.0, 1.0, c, v0],
            ]
        }
    }
}

// ============================================================================
// LimbBands (1:1 port of `com.devilfruits.core.limb.LimbBands`)
// ============================================================================

/// One arc-length segment mapping path distance `[s0, s1]` (model pixels) to skin V-row `[r0, r1]`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BandSegment {
    pub s0: f32,
    pub s1: f32,
    pub r0: f32,
    pub r1: f32,
}

/// Splits an arbitrary stretched limb length into shoulder, repeating middle, wrist, and fist/foot
/// UV bands (`LimbBands.java` 1:1).
pub struct LimbBands;

impl LimbBands {
    /// Builds segments with default `fill = 4` (`LimbBands.build(length, scale, fist)`).
    pub fn build_default(length: f32, scale: f32, fist: f32) -> Vec<BandSegment> {
        Self::build(length, scale, fist, 4)
    }

    /// Builds UV band segments for a stretched limb (`LimbBands.build(length, scale, fist, fill)` 1:1).
    pub fn build(length: f32, scale: f32, fist: f32, fill: u32) -> Vec<BandSegment> {
        let length = length.clamp(0.5, MAX_LIMB_LENGTH_PX);
        let scale = scale.max(1.0e-3);
        let fist = fist.max(0.5);
        let fill = fill.clamp(4, 7);
        let shoulder = 4.0_f32;
        let hand = 4.0 * fist;

        let mut out = Vec::with_capacity((length as usize).min(MAX_LIMB_LENGTH_PX as usize) + 8);
        if length < shoulder + hand {
            let k = length / (shoulder + hand);
            out.push(BandSegment {
                s0: 0.0,
                s1: shoulder * k,
                r0: 0.0,
                r1: 4.0,
            });
            out.push(BandSegment {
                s0: shoulder * k,
                s1: (shoulder + fist) * k,
                r0: 8.0,
                r1: 9.0,
            });
            out.push(BandSegment {
                s0: (shoulder + fist) * k,
                s1: length,
                r0: 9.0,
                r1: 12.0,
            });
            return out;
        }

        out.push(BandSegment {
            s0: 0.0,
            s1: shoulder,
            r0: 0.0,
            r1: 4.0,
        });
        let middle = length - shoulder - hand;
        let mut s = shoulder;
        let lead = (fill - 4) as f32;
        let fill_f = fill as f32;

        if middle < lead {
            let c = cut(middle, scale);
            if c > 0.0 {
                out.push(BandSegment {
                    s0: s,
                    s1: s + c,
                    r0: 4.0,
                    r1: 4.0 + c,
                });
                s += c;
            }
        } else {
            if lead > 0.0 {
                out.push(BandSegment {
                    s0: s,
                    s1: s + lead,
                    r0: 4.0,
                    r1: fill_f,
                });
                s += lead;
            }

            let tile = (8 - fill) as f32;
            let rest = middle - lead;
            if tile * scale <= 1.001 {
                let c = cut(rest, scale);
                if c > 0.0 {
                    let row = fill_f + 0.5 / scale;
                    out.push(BandSegment {
                        s0: s,
                        s1: s + c,
                        r0: row,
                        r1: row,
                    });
                    s += c;
                }
            } else {
                let tiles = (rest / tile + 1.0e-4) as usize;
                for _ in 0..tiles {
                    out.push(BandSegment {
                        s0: s,
                        s1: s + tile,
                        r0: fill_f,
                        r1: 8.0,
                    });
                    s += tile;
                }
                let c = cut(rest - tiles as f32 * tile, scale);
                if c > 0.0 {
                    out.push(BandSegment {
                        s0: s,
                        s1: s + c,
                        r0: fill_f,
                        r1: fill_f + c,
                    });
                    s += c;
                }
            }
        }

        out.push(BandSegment {
            s0: s,
            s1: s + fist,
            r0: 8.0,
            r1: 9.0,
        });
        out.push(BandSegment {
            s0: s + fist,
            s1: s + hand,
            r0: 9.0,
            r1: 12.0,
        });
        out
    }

    #[inline]
    pub fn total_length(segments: &[BandSegment]) -> f32 {
        segments.last().map_or(0.0, |seg| seg.s1)
    }
}

#[inline]
fn cut(length: f32, scale: f32) -> f32 {
    ((f64::from(length) * f64::from(scale) + 0.001).floor() as f32) / scale
}

// ============================================================================
// LimbFill (1:1 port of `com.devilfruits.core.limb.LimbFill`)
// ============================================================================

/// Detects sleeve-cuff boundaries in player skin pixels so the repeating middle
/// band starts at the first uniform bare-skin/sleeve row (`LimbFill.java` 1:1).
pub struct LimbFill;

impl LimbFill {
    pub const SAME: f32 = 48.0;
    pub const HARD: f32 = 120.0;

    /// Finds the first fill row (`4..=7`) from packed `0xAARRGGBB` / `0xAABBGGRR` pixels
    /// (alpha in bits `24..32`), matching `LimbFill.firstRow` 1:1.
    pub fn first_row(px: &[u32], w: u32, h: u32, layout: &LimbUvLayout) -> u32 {
        let mut edges = 0u32;
        let mut hard = 0u32;

        for row in 4..=7 {
            let next = if row == 7 { 4 } else { row + 1 };
            let d = Self::difference(px, w, h, layout, row, next);
            if d >= Self::SAME {
                edges += 1;
            }
            if d >= Self::HARD {
                hard += 1;
            }
        }

        if edges != 0 && hard < 3 && (hard != 0 || edges < 3) {
            let mut first = 7u32;
            let mut row = 6i32;
            while row >= 4 && Self::difference(px, w, h, layout, row as u32, 7) < Self::SAME {
                first = row as u32;
                row -= 1;
            }
            first
        } else {
            4
        }
    }

    /// Convenience variant taking raw RGBA8 bytes (`[r, g, b, a, ...]`).
    pub fn first_row_rgba8(rgba: &[u8], w: u32, h: u32, layout: &LimbUvLayout) -> u32 {
        let packed: Vec<u32> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| {
                u32::from(c[3]) << 24
                    | u32::from(c[2]) << 16
                    | u32::from(c[1]) << 8
                    | u32::from(c[0])
            })
            .collect();
        Self::first_row(&packed, w, h, layout)
    }

    /// Computes average L1 channel difference between two skin rows (`LimbFill.difference` 1:1).
    pub fn difference(
        px: &[u32],
        w: u32,
        h: u32,
        layout: &LimbUvLayout,
        row_a: u32,
        row_b: u32,
    ) -> f32 {
        let s = layout.scale.round().max(1.0) as usize;
        let w_u = w as usize;
        let h_u = h as usize;
        let x0 = layout.px(layout.u as f32).round().max(0.0) as usize;
        let x1 = w_u.min(
            layout
                .px((layout.u + 2 * layout.depth + 2 * layout.width) as f32)
                .round()
                .max(0.0) as usize,
        );
        let ya = layout.px(layout.side_top() + row_a as f32).round().max(0.0) as usize;
        let yb = layout.px(layout.side_top() + row_b as f32).round().max(0.0) as usize;

        let mut sum = 0u64;
        let mut n = 0u64;
        for dy in 0..s {
            if ya + dy >= h_u || yb + dy >= h_u {
                break;
            }
            for x in x0..x1 {
                let ia = (ya + dy) * w_u + x;
                let ib = (yb + dy) * w_u + x;
                if ia < px.len() && ib < px.len() {
                    sum += u64::from(texel_diff(px[ia], px[ib]));
                    n += 1;
                }
            }
        }
        if n == 0 {
            0.0
        } else {
            sum as f32 / n as f32
        }
    }
}

#[inline]
fn texel_diff(a: u32, b: u32) -> u32 {
    let oa = (a >> 24) >= 128;
    let ob = (b >> 24) >= 128;
    if oa != ob {
        765
    } else if !oa {
        0
    } else {
        let dr = (a & 0xFF).abs_diff(b & 0xFF);
        let dg = ((a >> 8) & 0xFF).abs_diff((b >> 8) & 0xFF);
        let db = ((a >> 16) & 0xFF).abs_diff((b >> 16) & 0xFF);
        dr + dg + db
    }
}

// ============================================================================
// LimbPath (1:1 port of `com.devilfruits.core.limb.LimbPath`)
// ============================================================================

const ARC_STEPS: usize = 16;
const MAX_ACC: usize = (MAX_PATH_POINTS - 1) * ARC_STEPS + 1;

/// Arc-length-parameterized cubic Hermite path with Double-Reflection RMF transport
/// (`LimbPath.java` 1:1).
#[derive(Clone, Debug)]
pub struct LimbPath {
    points: [Vec3; MAX_PATH_POINTS],
    out_tan: [Vec3; MAX_PATH_POINTS],
    in_tan: [Vec3; MAX_PATH_POINTS],
    acc: [f32; MAX_ACC],
    count: usize,
    start_dir: Option<Vec3>,
    end_dir: Option<Vec3>,
    length: f32,
}

impl Default for LimbPath {
    fn default() -> Self {
        Self::new()
    }
}

impl LimbPath {
    pub fn new() -> Self {
        Self {
            points: [Vec3::ZERO; MAX_PATH_POINTS],
            out_tan: [Vec3::ZERO; MAX_PATH_POINTS],
            in_tan: [Vec3::ZERO; MAX_PATH_POINTS],
            acc: [0.0; MAX_ACC],
            count: 0,
            start_dir: None,
            end_dir: None,
            length: 0.0,
        }
    }

    pub fn reset(&mut self) -> &mut Self {
        self.count = 0;
        self.start_dir = None;
        self.end_dir = None;
        self.length = 0.0;
        self
    }

    pub fn add(&mut self, p: Vec3) -> &mut Self {
        if self.count < MAX_PATH_POINTS {
            self.points[self.count] = p;
            self.count += 1;
        }
        self
    }

    pub fn set(&mut self, index: usize, p: Vec3) -> &mut Self {
        if index < MAX_PATH_POINTS {
            self.points[index] = p;
        }
        self
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn point(&self, index: usize) -> Vec3 {
        self.points[index]
    }

    pub fn start_direction(&mut self, dir: Vec3) -> &mut Self {
        let len = dir.length();
        if len > 1.0e-6 {
            self.start_dir = Some(dir / len);
        }
        self
    }

    pub fn end_direction(&mut self, dir: Vec3) -> &mut Self {
        let len = dir.length();
        if len > 1.0e-6 {
            self.end_dir = Some(dir / len);
        }
        self
    }

    /// Computes Hermite tangents and the piecewise arc-length lookup table (`LimbPath.build` 1:1).
    pub fn build(&mut self) -> &mut Self {
        self.length = 0.0;
        self.acc[0] = 0.0;
        if self.count < 2 {
            return self;
        }

        for i in 0..self.count - 1 {
            let chord = self.points[i].distance(self.points[i + 1]);
            self.out_tan[i] = self.compute_tangent(i, chord);
            self.in_tan[i + 1] = self.compute_tangent(i + 1, chord);
        }

        let mut k = 0usize;
        for seg in 0..self.count - 1 {
            let (mut prev, _) = self.eval_segment(seg, 0.0, false);
            for j in 1..=ARC_STEPS {
                let (cur, _) = self.eval_segment(seg, j as f32 / ARC_STEPS as f32, false);
                self.length += prev.distance(cur);
                k += 1;
                self.acc[k] = self.length;
                prev = cur;
            }
        }
        self
    }

    #[inline]
    pub fn length(&self) -> f32 {
        self.length
    }

    /// Samples `(position, unit_tangent)` at arc-length `s` (`LimbPath.sample` 1:1).
    pub fn sample(&self, s: f32) -> (Vec3, Vec3) {
        if self.count < 2 {
            return (self.points[0], Vec3::Y);
        }
        let last = (self.count - 1) * ARC_STEPS;
        let s = s.clamp(0.0, self.length);
        let mut lo = 0usize;
        let mut hi = last;
        while hi - lo > 1 {
            let mid = (lo + hi) >> 1;
            if self.acc[mid] <= s {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let span = self.acc[hi] - self.acc[lo];
        let f = if span > 1.0e-6 {
            (s - self.acc[lo]) / span
        } else {
            0.0
        };
        let seg = (lo / ARC_STEPS).min(self.count - 2);
        let t = ((lo - seg * ARC_STEPS) as f32 + f) / ARC_STEPS as f32;
        self.eval_segment(seg, t, true)
    }

    fn eval_segment(&self, seg: usize, t: f32, want_tangent: bool) -> (Vec3, Vec3) {
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        let p0 = self.points[seg];
        let p1 = self.points[seg + 1];
        let m0 = self.out_tan[seg];
        let m1 = self.in_tan[seg + 1];

        let pos = p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11;
        if !want_tangent {
            return (pos, Vec3::Y);
        }

        let d00 = 6.0 * t2 - 6.0 * t;
        let d10 = 3.0 * t2 - 4.0 * t + 1.0;
        let d01 = -6.0 * t2 + 6.0 * t;
        let d11 = 3.0 * t2 - 2.0 * t;
        let raw_tan = p0 * d00 + m0 * d10 + p1 * d01 + m1 * d11;
        let l = raw_tan.length();
        let tangent = if l > 1.0e-6 {
            raw_tan / l
        } else {
            let chord = p1 - p0;
            if chord.length_squared() > 1.0e-12 {
                chord.normalize()
            } else {
                Vec3::Y
            }
        };
        (pos, tangent)
    }

    fn compute_tangent(&self, i: usize, chord: f32) -> Vec3 {
        let dir = if i == 0
            && let Some(d) = self.start_dir
        {
            d
        } else if i == self.count - 1
            && let Some(d) = self.end_dir
        {
            d
        } else {
            let a = i.saturating_sub(1);
            let b = (i + 1).min(self.count - 1);
            let d = self.points[b] - self.points[a];
            let l = d.length();
            if l > 1.0e-6 { d / l } else { d }
        };
        dir * chord
    }

    /// Double-Reflection Rotation Minimizing Frame (RMF) transport (`LimbPath.transport` 1:1).
    pub fn transport(p0: Vec3, t0: Vec3, r0: Vec3, p1: Vec3, t1: Vec3) -> Vec3 {
        let v1 = p1 - p0;
        let c1 = v1.length_squared();
        let mut r = r0;
        let mut t = t0;
        if c1 > 1.0e-10 {
            let kr = (2.0 / c1) * v1.dot(r);
            let kt = (2.0 / c1) * v1.dot(t);
            r -= v1 * kr;
            t -= v1 * kt;
        }
        let v2 = t1 - t;
        let c2 = v2.length_squared();
        if c2 > 1.0e-10 {
            let k = (2.0 / c2) * v2.dot(r);
            r -= v2 * k;
        }
        let proj = r - t1 * r.dot(t1);
        if proj.length_squared() > 1.0e-12 {
            proj.normalize()
        } else {
            r0
        }
    }

    /// Computes a conservative world-space AABB around the built path (for frustum culling).
    pub fn world_bounds(&self, to_world: Mat4, cam: DVec3, pad_blocks: f64) -> EntityAabb {
        let mut min = DVec3::splat(f64::INFINITY);
        let mut max = DVec3::splat(f64::NEG_INFINITY);
        let samples = ((self.count.saturating_sub(1)) * 4).max(2);
        for i in 0..=samples {
            let s = if self.length > 0.0 {
                self.length * (i as f32 / samples as f32)
            } else {
                0.0
            };
            let (p, _) = self.sample(s);
            let w = to_world.transform_point3(p);
            let wd = DVec3::new(w.x as f64 + cam.x, w.y as f64 + cam.y, w.z as f64 + cam.z);
            min = min.min(wd);
            max = max.max(wd);
        }
        EntityAabb {
            min: min - DVec3::splat(pad_blocks),
            max: max + DVec3::splat(pad_blocks),
        }
    }
}

// ============================================================================
// LimbSpec, LimbFrame, TexelBoxMesh & LimbRenderer
// (1:1 port of `LimbSpec.java`, `LimbFrame.java`, `TexelBoxMesh.java`, `LimbRenderer.java`)
// ============================================================================

/// Configuration for one stretched limb instance (`LimbSpec.java` 1:1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LimbSpec {
    pub limb: LimbKind,
    pub side: LimbSide,
    /// Stretch tension `[0, 1]` that thins the middle section toward `thinnest`.
    pub tension: f32,
    /// Minimum thickness multiplier at full tension (default `0.75`).
    pub thinnest: f32,
    /// Fist/foot end enlargement multiplier (default `1.0`).
    pub fist: f32,
    /// Helical twist in full turns along the limb length.
    pub twist_turns: f32,
    /// Continuous spin angle in radians added to the twist.
    pub spin: f32,
    /// Whether to emit the skin overlay (sleeve/pants) pass.
    pub overlay: bool,
    /// Whether this stretched limb hides the player's vanilla limb (and armor/held item).
    pub replaces: bool,
    /// Packed `0xAARRGGBB` tint color (default opaque white `0xFFFF_FFFF`).
    pub color: u32,
    /// Whether this limb uses the 3x swarm budget (e.g. Gomu Gomu no Gatling).
    pub swarm: bool,
    /// Optional swirl texture UV V-scroll offset.
    pub swirl_scroll: f32,
    /// Packed `0xAARRGGBB` swirl overlay tint (`DEFAULT_SWIRL_COLOR`).
    pub swirl_color: u32,
    /// Enable swirl overlay pass.
    pub has_swirl: bool,
}

impl Default for LimbSpec {
    fn default() -> Self {
        Self {
            limb: LimbKind::Arm,
            side: LimbSide::Right,
            tension: 0.0,
            thinnest: 0.75,
            fist: 1.0,
            twist_turns: 0.0,
            spin: 0.0,
            overlay: true,
            replaces: true,
            color: 0xFFFF_FFFF,
            swarm: false,
            swirl_scroll: 0.0,
            swirl_color: DEFAULT_SWIRL_COLOR,
            has_swirl: false,
        }
    }
}

impl LimbSpec {
    pub fn arm(mut self, side: LimbSide) -> Self {
        self.limb = LimbKind::Arm;
        self.side = side;
        self
    }

    pub fn leg(mut self, side: LimbSide) -> Self {
        self.limb = LimbKind::Leg;
        self.side = side;
        self
    }
}

/// Optional per-ring deformation callback (`TexelBoxMesh.RingShaper` 1:1).
pub trait RingShaper {
    fn shape(
        &self,
        s: f32,
        total_length: f32,
        pos: &mut Vec3,
        across: &mut Vec3,
        depth: &mut Vec3,
        scale: &mut f32,
    );
}

/// Optional per-vertex deformation & lighting callback (`TexelBoxMesh.VertexShaper` 1:1).
pub trait VertexShaper {
    fn shape(&self, face: usize, s: f32, pos: &mut Vec3);

    fn light(&self, _normal: &mut Vec3) -> f32 {
        1.0
    }
}

/// One emitted vertex of a swept stretched limb mesh (GPU-ready, 40 bytes).
#[repr(C)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct LimbVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
    pub color: u32,
    pub light: u32,
}

/// Coordinate frame between player model-pixel space and camera-relative world space
/// (`LimbFrame.java` + `LimbRenderer.root` 1:1).
#[derive(Clone, Debug)]
pub struct LimbFrame {
    pub layout: LimbUvLayout,
    pub first_person: bool,
    pub root_posed: bool,
    pub root_free: bool,
    pub cam: DVec3,
    pub to_world: Mat4,
    pub to_px: Mat4,
    pub pivot: Vec3,
    pub posed: Quat,
}

impl LimbFrame {
    pub fn new(layout: LimbUvLayout, to_world: Mat4, cam: DVec3, pivot: Vec3, posed: Quat) -> Self {
        let to_px = to_world.inverse();
        Self {
            layout,
            first_person: false,
            root_posed: false,
            root_free: false,
            cam,
            to_world,
            to_px,
            pivot,
            posed,
        }
    }

    pub fn set_root_posed(&mut self) {
        self.root_posed = true;
    }

    pub fn set_root_at(&mut self, world_pos: DVec3) {
        self.pivot = self.px(world_pos);
        self.root_free = true;
    }

    #[inline]
    pub fn px(&self, world_pos: DVec3) -> Vec3 {
        let rel = world_pos - self.cam;
        self.to_px
            .transform_point3(Vec3::new(rel.x as f32, rel.y as f32, rel.z as f32))
    }

    #[inline]
    pub fn px_direction(&self, dir: DVec3) -> Vec3 {
        self.to_px
            .transform_vector3(Vec3::new(dir.x as f32, dir.y as f32, dir.z as f32))
            .normalize_or_zero()
    }

    #[inline]
    pub fn px_length(&self, blocks: f64) -> f32 {
        blocks as f32 * self.to_px.transform_vector3(Vec3::Y).length()
    }

    #[inline]
    pub fn pivot_world(&self) -> DVec3 {
        let w = self.to_world.transform_point3(self.pivot);
        DVec3::new(w.x as f64 + self.cam.x, w.y as f64 + self.cam.y, w.z as f64 + self.cam.z)
    }

    #[inline]
    pub fn posed_point(&self, offset: Vec3) -> Vec3 {
        let local = Vec3::new(
            self.layout.centre_x() + offset.x,
            self.layout.min_y + offset.y,
            self.layout.centre_z() + offset.z,
        );
        self.posed * local + self.pivot
    }

    /// Aligns the shoulder/hip root quaternion toward `path[1]`, sets `path[0]` and `start_direction`,
    /// builds `path`, and returns initial radial reference vector `n0` (`LimbRenderer.drawPlayer` + `root` 1:1).
    pub fn align_root_and_build(&self, path: &mut LimbPath) -> Vec3 {
        let mut root_rot = self.posed;
        if !self.root_posed && path.count() >= 2 {
            let target = path.point(1);
            let posed_axis = (self.posed * Vec3::Y).normalize_or_zero();
            let mut from = self.pivot;
            let passes = if self.root_free { 1 } else { 2 };
            for _ in 0..passes {
                let dir = target - from;
                if dir.length_squared() < 1.0e-4 {
                    break;
                }
                let turn = Quat::from_rotation_arc(posed_axis, dir.normalize());
                root_rot = turn * self.posed;
                let attach = Vec3::new(
                    self.layout.centre_x(),
                    self.layout.min_y,
                    self.layout.centre_z(),
                );
                from = root_rot * attach + self.pivot;
            }
        }

        let start = if self.root_free {
            self.pivot
        } else {
            let attach = Vec3::new(
                self.layout.centre_x(),
                self.layout.min_y,
                self.layout.centre_z(),
            );
            root_rot * attach + self.pivot
        };
        let start_dir = root_rot * Vec3::Y;
        path.set(0, start).start_direction(start_dir).build();
        root_rot * Vec3::X
    }
}

/// Tubular cuboid-section mesh generator along a [`LimbPath`] (`TexelBoxMesh.java` 1:1).
#[derive(Clone, Debug)]
pub struct TexelBoxMesh {
    ring_s: Vec<f32>,
    centre: Vec<Vec3>,
    across: Vec<Vec3>,
    depth: Vec<Vec3>,
    along: Vec<Vec3>,
    scale: Vec<f32>,
    light: Vec<u32>,
    seg_first: Vec<usize>,
    seg_last: Vec<usize>,
    segments: Vec<BandSegment>,
    length: f32,
    pub v_shift: f32,
}

impl Default for TexelBoxMesh {
    fn default() -> Self {
        Self::new()
    }
}

impl TexelBoxMesh {
    pub fn new() -> Self {
        Self {
            ring_s: Vec::new(),
            centre: Vec::new(),
            across: Vec::new(),
            depth: Vec::new(),
            along: Vec::new(),
            scale: Vec::new(),
            light: Vec::new(),
            seg_first: Vec::new(),
            seg_last: Vec::new(),
            segments: Vec::new(),
            length: 0.0,
            v_shift: 0.0,
        }
    }

    #[inline]
    pub fn rings(&self) -> usize {
        self.ring_s.len()
    }

    /// Sweeps cross-section rings along `path` across `segments` (`TexelBoxMesh.sweep` 1:1).
    pub fn sweep(
        &mut self,
        path: &LimbPath,
        segments: &[BandSegment],
        mut step: f32,
        n0: Vec3,
        spec: &LimbSpec,
        lights: [u32; 3],
        ring_shaper: Option<&dyn RingShaper>,
    ) {
        self.segments.clear();
        self.segments.extend_from_slice(segments);
        self.ring_s.clear();
        self.centre.clear();
        self.across.clear();
        self.depth.clear();
        self.along.clear();
        self.scale.clear();
        self.light.clear();
        self.seg_first.clear();
        self.seg_last.clear();

        self.length = LimbBands::total_length(segments);
        let hand_start = (self.length - 4.0 * spec.fist).max(0.0);
        step = step.max(0.1);

        let count = segments.len();
        let mut need = 1usize;
        for seg in segments {
            need += (((seg.s1 - seg.s0) / step).ceil() as usize).max(1);
        }
        if need > MAX_MESH_RINGS {
            let denom = MAX_MESH_RINGS.saturating_sub(count + 1).max(1) as f32;
            step *= need as f32 / denom + 0.01;
        }

        let mut prev_pos = Vec3::ZERO;
        let mut prev_tan = Vec3::Y;
        let mut frame_n = Vec3::X;

        for (i, seg) in segments.iter().enumerate() {
            let sub = (((seg.s1 - seg.s0) / step - 0.001).ceil() as usize).max(1);
            if i == 0 {
                self.push_ring(
                    path,
                    seg.s0,
                    n0,
                    spec,
                    hand_start,
                    lights,
                    ring_shaper,
                    &mut prev_pos,
                    &mut prev_tan,
                    &mut frame_n,
                );
            }
            let first = self.ring_s.len().saturating_sub(1);
            self.seg_first.push(first);
            for j in 1..=sub {
                if self.ring_s.len() >= MAX_MESH_RINGS {
                    break;
                }
                let s = seg.s0 + (seg.s1 - seg.s0) * (j as f32 / sub as f32);
                self.push_ring(
                    path,
                    s,
                    n0,
                    spec,
                    hand_start,
                    lights,
                    ring_shaper,
                    &mut prev_pos,
                    &mut prev_tan,
                    &mut frame_n,
                );
            }
            let last = self.ring_s.len().saturating_sub(1);
            self.seg_last.push(last);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_ring(
        &mut self,
        path: &LimbPath,
        s: f32,
        n0: Vec3,
        spec: &LimbSpec,
        hand_start: f32,
        lights: [u32; 3],
        ring_shaper: Option<&dyn RingShaper>,
        prev_pos: &mut Vec3,
        prev_tan: &mut Vec3,
        frame_n: &mut Vec3,
    ) {
        let (mut pos, tan) = path.sample(s);
        if self.ring_s.is_empty() {
            let mut fn0 = n0 - tan * n0.dot(tan);
            if fn0.length_squared() < 1.0e-8 {
                let fallback = if tan.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
                fn0 = fallback - tan * fallback.dot(tan);
            }
            *frame_n = fn0.normalize_or_zero();
        } else {
            *frame_n = LimbPath::transport(*prev_pos, *prev_tan, *frame_n, pos, tan);
        }

        *prev_pos = pos;
        *prev_tan = tan;

        let f = if self.length > 0.0 {
            s / self.length
        } else {
            0.0
        };
        let roll = f * (spec.twist_turns * TAU + spec.spin);
        let base_b = frame_n.cross(tan);
        let c = roll.cos();
        let sn = roll.sin();
        let mut n = *frame_n * c + base_b * sn;
        let mut b = base_b * c - *frame_n * sn;
        let mut k = thickness(s, spec, hand_start);
        if let Some(shaper) = ring_shaper {
            shaper.shape(s, self.length, &mut pos, &mut n, &mut b, &mut k);
        }

        let lit = if f < 0.5 {
            mix_light(lights[0], lights[1], f * 2.0)
        } else {
            mix_light(lights[1], lights[2], f * 2.0 - 1.0)
        };

        self.ring_s.push(s);
        self.centre.push(pos);
        self.across.push(n);
        self.depth.push(b);
        self.along.push(tan);
        self.scale.push(k);
        self.light.push(lit);
    }

    /// Emits quad vertices (`4` per face: 4 radial sides per ring pair + shoulder/hip & fist/foot caps)
    /// into `out` (`TexelBoxMesh.write` 1:1).
    #[allow(clippy::too_many_arguments)]
    pub fn write_quads(
        &self,
        pose: Mat4,
        normal_mat: Mat3,
        uv: &LimbUvLayout,
        color: u32,
        per_texel: bool,
        shaper: Option<&dyn VertexShaper>,
        out: &mut Vec<LimbVertex>,
    ) {
        if self.ring_s.len() < 2 {
            return;
        }
        let hw = uv.width as f32 * 0.5 + uv.grow;
        let hd = uv.depth as f32 * 0.5 + uv.grow;
        let mx = if uv.mirror { -1.0_f32 } else { 1.0_f32 };
        let top = uv.side_top();

        for (seg_idx, seg) in self.segments.iter().enumerate() {
            let rate = if seg.s1 > seg.s0 {
                (seg.r1 - seg.r0) / (seg.s1 - seg.s0)
            } else {
                0.0
            };
            let first = self.seg_first[seg_idx];
            let last = self.seg_last[seg_idx];

            for a in first..last {
                let b_idx = a + 1;
                let va = uv.v_coord(top + seg.r0 + (self.ring_s[a] - seg.s0) * rate + self.v_shift);
                let vb =
                    uv.v_coord(top + seg.r0 + (self.ring_s[b_idx] - seg.s0) * rate + self.v_shift);

                for face in 0..4usize {
                    let u0 = uv.u_coord(uv.side_u(face, 0));
                    let u1 = uv.u_coord(uv.side_u(face, 1));
                    let x0 = LimbUvLayout::corner_x(face, 0) as f32;
                    let z0 = LimbUvLayout::corner_z(face, 0) as f32;
                    let x1 = LimbUvLayout::corner_x(face, 1) as f32;
                    let z1 = LimbUvLayout::corner_z(face, 1) as f32;
                    let columns = if !per_texel {
                        1
                    } else if face != 0 && face != 2 {
                        uv.width.max(1) as usize
                    } else {
                        uv.depth.max(1) as usize
                    };

                    for c in 0..columns {
                        let t0 = c as f32 / columns as f32;
                        let t1 = (c + 1) as f32 / columns as f32;
                        let xa = lerp_f32(x0, x1, t0) * mx * hw;
                        let za = lerp_f32(z0, z1, t0) * hd;
                        let ua = lerp_f32(u0, u1, t0);
                        let xb = lerp_f32(x0, x1, t1) * mx * hw;
                        let zb = lerp_f32(z0, z1, t1) * hd;
                        let ub = lerp_f32(u0, u1, t1);

                        let na = self.face_normal(face, a, mx);
                        self.emit_vertex(
                            pose, normal_mat, a, xa, za, ua, va, color, uv.grow, face, na, shaper,
                            out,
                        );
                        self.emit_vertex(
                            pose, normal_mat, a, xb, zb, ub, va, color, uv.grow, face, na, shaper,
                            out,
                        );
                        let nb = self.face_normal(face, b_idx, mx);
                        self.emit_vertex(
                            pose, normal_mat, b_idx, xb, zb, ub, vb, color, uv.grow, face, nb,
                            shaper, out,
                        );
                        self.emit_vertex(
                            pose, normal_mat, b_idx, xa, za, ua, vb, color, uv.grow, face, nb,
                            shaper, out,
                        );
                    }
                }
            }
        }

        self.write_cap(
            pose, normal_mat, uv, true, 0, hw, hd, mx, color, shaper, out,
        );
        self.write_cap(
            pose,
            normal_mat,
            uv,
            false,
            self.ring_s.len() - 1,
            hw,
            hd,
            mx,
            color,
            shaper,
            out,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn write_cap(
        &self,
        pose: Mat4,
        normal_mat: Mat3,
        uv: &LimbUvLayout,
        top_cap: bool,
        r: usize,
        hw: f32,
        hd: f32,
        mx: f32,
        color: u32,
        shaper: Option<&dyn VertexShaper>,
        out: &mut Vec<LimbVertex>,
    ) {
        let cap = uv.cap(top_cap);
        let sign = if top_cap { -1.0_f32 } else { 1.0_f32 };
        let normal = self.along[r] * sign;
        let face_id = if top_cap { 4 } else { 5 };
        for [cx, cz, cu, cv] in cap {
            self.emit_vertex(
                pose,
                normal_mat,
                r,
                cx * mx * hw,
                cz * hd,
                uv.u_coord(cu),
                uv.v_coord(cv),
                color,
                uv.grow,
                face_id,
                normal,
                shaper,
                out,
            );
        }
    }

    #[inline]
    fn face_normal(&self, face: usize, r: usize, mx: f32) -> Vec3 {
        let axis = if face != 0 && face != 2 {
            self.depth[r]
        } else {
            self.across[r]
        };
        let mut sign = if face != 0 && face != 1 { 1.0 } else { -1.0 };
        if face == 0 || face == 2 {
            sign *= mx;
        }
        axis * sign
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_vertex(
        &self,
        pose: Mat4,
        normal_mat: Mat3,
        r: usize,
        x: f32,
        z: f32,
        u: f32,
        v: f32,
        color: u32,
        grow: f32,
        face: usize,
        mut normal: Vec3,
        shaper: Option<&dyn VertexShaper>,
        out: &mut Vec<LimbVertex>,
    ) {
        let k = self.scale[r];
        let ext = if r == 0 {
            -grow
        } else if r + 1 == self.ring_s.len() {
            grow
        } else {
            0.0
        };
        let mut pos =
            self.centre[r] + (self.across[r] * x + self.depth[r] * z) * k + self.along[r] * ext;
        let mut c = color;
        if let Some(sh) = shaper {
            sh.shape(face, self.ring_s[r], &mut pos);
            let bright = sh.light(&mut normal);
            if bright != 1.0 {
                let a = color & 0xFF00_0000;
                let red = ((((color >> 16) & 0xFF) as f32 * bright).clamp(0.0, 255.0)) as u32;
                let grn = ((((color >> 8) & 0xFF) as f32 * bright).clamp(0.0, 255.0)) as u32;
                let blu = (((color & 0xFF) as f32 * bright).clamp(0.0, 255.0)) as u32;
                c = a | (red << 16) | (grn << 8) | blu;
            }
        }
        let world_pos = pose.transform_point3(pos);
        let mut wn = normal_mat * normal;
        let nlen = wn.length();
        if nlen > 1.0e-6 {
            wn /= nlen;
        }
        out.push(LimbVertex {
            pos: world_pos.to_array(),
            uv: [u, v],
            normal: wn.to_array(),
            color: c,
            light: self.light[r],
        });
    }
}

/// Complete mesh output for a stretched limb (`LimbRenderer.render` 1:1):
/// skin layer, optional overlay (sleeve/pants), and optional swirl layer.
#[derive(Clone, Debug, Default)]
pub struct StretchedLimbMesh {
    pub skin_quads: Vec<LimbVertex>,
    pub overlay_quads: Vec<LimbVertex>,
    pub swirl_quads: Vec<LimbVertex>,
}

impl StretchedLimbMesh {
    /// Generates all enabled layers for a stretched limb (`LimbRenderer.render` 1:1).
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        pose: Mat4,
        skin: &LimbUvLayout,
        overlay: Option<&LimbUvLayout>,
        path: &LimbPath,
        n0: Vec3,
        spec: &LimbSpec,
        skin_fill_row: u32,
        overlay_fill_row: u32,
        lights: [u32; 3],
        quality: CinematicQuality,
        ring_shaper: Option<&dyn RingShaper>,
        vertex_shaper: Option<&dyn VertexShaper>,
    ) -> Self {
        let fine = spec.twist_turns != 0.0
            || spec.spin != 0.0
            || ((path.count() > 2 || ring_shaper.is_some())
                && quality == CinematicQuality::Ultra);
        let step = if fine { 2.0 } else { 4.0 };
        let normal_mat = normal_mat3_safe(pose);

        let mut mesh = TexelBoxMesh::new();
        let mut out = Self::default();

        // 1. Base skin pass
        let skin_segs = LimbBands::build(path.length(), skin.scale, spec.fist, skin_fill_row);
        mesh.sweep(path, &skin_segs, step, n0, spec, lights, ring_shaper);
        mesh.write_quads(
            pose,
            normal_mat,
            skin,
            spec.color,
            false,
            vertex_shaper,
            &mut out.skin_quads,
        );

        // 2. Sleeve / pants overlay pass
        if let Some(ov) = overlay
            && spec.overlay
        {
            let ov_segs = LimbBands::build(path.length(), ov.scale, spec.fist, overlay_fill_row);
            mesh.sweep(path, &ov_segs, step, n0, spec, lights, ring_shaper);
            mesh.write_quads(
                pose,
                normal_mat,
                ov,
                spec.color,
                false,
                vertex_shaper,
                &mut out.overlay_quads,
            );
        }

        // 3. Optional swirl effect pass (e.g. Gomu spin / Haki air swirl)
        if spec.has_swirl {
            let tile = LimbUvLayout {
                tex_width: 16,
                tex_height: 16,
                scale: 1.0,
                u: 0,
                v: 0,
                width: skin.width,
                depth: skin.depth,
                min_x: skin.min_x,
                min_y: skin.min_y,
                min_z: skin.min_z,
                pivot_x: skin.pivot_x,
                pivot_y: skin.pivot_y,
                pivot_z: skin.pivot_z,
                grow: 0.32,
                mirror: false,
            };
            let swirl_seg = [BandSegment {
                s0: 0.0,
                s1: path.length(),
                r0: 0.0,
                r1: path.length(),
            }];
            mesh.sweep(path, &swirl_seg, step, n0, spec, lights, ring_shaper);
            mesh.v_shift = -spec.swirl_scroll;
            mesh.write_quads(
                pose,
                normal_mat,
                &tile,
                spec.swirl_color,
                false,
                None,
                &mut out.swirl_quads,
            );
            mesh.v_shift = 0.0;
        }

        out
    }

    /// Expands quad vertices (`4` per quad) into a triangle list (`6` per quad).
    pub fn quads_to_triangles(quads: &[LimbVertex]) -> Vec<LimbVertex> {
        let mut tris = Vec::with_capacity((quads.len() / 4) * 6);
        for q in quads.as_chunks::<4>().0 {
            tris.push(q[0]);
            tris.push(q[1]);
            tris.push(q[2]);
            tris.push(q[0]);
            tris.push(q[2]);
            tris.push(q[3]);
        }
        tris
    }
}

#[inline]
fn thickness(s: f32, spec: &LimbSpec, hand_start: f32) -> f32 {
    if s >= hand_start {
        let swell = ((s - hand_start) / spec.fist.max(0.001)).clamp(0.0, 1.0);
        lerp_f32(1.0, spec.fist, swell)
    } else {
        let mid0 = 4.0_f32;
        if spec.tension <= 0.0 || hand_start <= mid0 {
            1.0
        } else {
            let ramp = ((hand_start - mid0) * 0.5).min(4.0);
            let w = smoothstep((s - mid0) / ramp) * (1.0 - smoothstep((s - (hand_start - ramp)) / ramp));
            let thin = lerp_f32(1.0, spec.thinnest, spec.tension.clamp(0.0, 1.0));
            lerp_f32(1.0, thin, w)
        }
    }
}

#[inline]
fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

#[inline]
fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn mix_light(a: u32, b: u32, t: f32) -> u32 {
    let lo = lerp_f32((a & 0xFFFF) as f32, (b & 0xFFFF) as f32, t).round() as u32;
    let hi = lerp_f32((a >> 16) as f32, (b >> 16) as f32, t).round() as u32;
    (lo & 0xFFFF) | ((hi & 0xFFFF) << 16)
}

fn normal_mat3_safe(m: Mat4) -> Mat3 {
    let m3 = Mat3::from_mat4(m);
    let det = m3.determinant();
    if det.abs() > 1.0e-8 && det.is_finite() {
        m3.inverse().transpose()
    } else {
        m3
    }
}

// ============================================================================
// Limbs Registry, Armor/Held-Item Hiding & Frustum Culling Expansion
// (1:1 port of `Limbs.java`, `LimbArmorLayerMixin`, `LimbItemLayerMixin`, `LimbCullMixin`)
// ============================================================================

/// Cinematic VFX quality tier (`CoreClientConfig.CinematicQuality`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CinematicQuality {
    #[default]
    Ultra,
    High,
    Medium,
    Low,
}

/// Client limb configuration (`CoreClientConfig` limbs section).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimbConfig {
    pub max_limbs: usize,
    pub quality: CinematicQuality,
    pub show_armor_on_stretch: bool,
}

impl Default for LimbConfig {
    fn default() -> Self {
        Self {
            max_limbs: DEFAULT_MAX_LIMBS,
            quality: CinematicQuality::Ultra,
            show_armor_on_stretch: false,
        }
    }
}

mod serde_dvec3 {
    use glam::DVec3;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &DVec3, s: S) -> Result<S::Ok, S::Error> {
        v.to_array().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DVec3, D::Error> {
        let a = <[f64; 3]>::deserialize(d)?;
        Ok(DVec3::from_array(a))
    }
}

/// Axis-aligned world bounding box used for entity frustum culling expansion.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntityAabb {
    #[serde(with = "serde_dvec3")]
    pub min: DVec3,
    #[serde(with = "serde_dvec3")]
    pub max: DVec3,
}

impl EntityAabb {
    pub const fn new(min: DVec3, max: DVec3) -> Self {
        Self { min, max }
    }

    /// Creates a vanilla player bounding box centered at feet `pos` (`0.6 x 1.8 x 0.6`).
    pub fn player_at(pos: DVec3) -> Self {
        Self {
            min: DVec3::new(pos.x - 0.3, pos.y, pos.z - 0.3),
            max: DVec3::new(pos.x + 0.3, pos.y + 1.8, pos.z + 0.3),
        }
    }

    /// Expands this AABB symmetrically on all 3 axes by `margin` blocks.
    #[inline]
    pub fn inflate(self, margin: f64) -> Self {
        let m = margin.max(0.0);
        Self {
            min: self.min - DVec3::splat(m),
            max: self.max + DVec3::splat(m),
        }
    }

    /// Returns the union of two AABBs.
    #[inline]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[inline]
    pub fn contains_point(self, p: DVec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    #[inline]
    pub fn size(self) -> DVec3 {
        self.max - self.min
    }
}

/// Visibility overrides for a player's vanilla skin limbs, armor pieces, and held items
/// (`LimbClient`, `LimbArmorLayerMixin`, `LimbItemLayerMixin` 1:1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimbVisibilityFlags {
    pub hide_right_arm: bool,
    pub hide_left_arm: bool,
    pub hide_right_leg: bool,
    pub hide_left_leg: bool,
    pub hide_right_arm_armor: bool,
    pub hide_left_arm_armor: bool,
    pub hide_right_leg_armor: bool,
    pub hide_left_leg_armor: bool,
    pub hide_right_held_item: bool,
    pub hide_left_held_item: bool,
}

/// Unique handle to an active stretched limb entry in [`Limbs`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LimbHandle(pub u64);

/// Active stretched limb registration (`Limbs.Active` 1:1).
#[derive(Clone, Debug)]
pub struct ActiveLimb {
    pub handle: LimbHandle,
    pub entity_id: u32,
    pub spec: LimbSpec,
    pub done: bool,
    /// Conservative reach in blocks (up to 64+ blocks) for frustum culling expansion.
    pub reach_blocks: f64,
    /// Optional exact world-space AABB updated each frame from the live [`LimbPath`].
    pub world_bounds: Option<EntityAabb>,
}

/// Global stretched-limb manager (`com.devilfruits.client.limb.Limbs` 1:1).
#[derive(Clone, Debug, Default)]
pub struct Limbs {
    pub config: LimbConfig,
    next_id: u64,
    by_entity: HashMap<u32, Vec<LimbHandle>>,
    entries: HashMap<LimbHandle, ActiveLimb>,
    order: VecDeque<LimbHandle>,
    swarm: VecDeque<LimbHandle>,
    arms_gone_entities: HashMap<u32, bool>,
    extra_held_item_hide: HashMap<(u32, LimbSide), bool>,
}

impl Limbs {
    pub fn new(config: LimbConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    /// Active normal limb cap (`Limbs.cap` 1:1: halved below High quality, minimum 2).
    #[inline]
    pub fn normal_cap(&self) -> usize {
        let max = self.config.max_limbs;
        if matches!(
            self.config.quality,
            CinematicQuality::Ultra | CinematicQuality::High
        ) {
            max
        } else {
            (max / 2).max(2)
        }
    }

    /// Active swarm limb cap (`normal_cap * SWARM_FACTOR` = 3x).
    #[inline]
    pub fn swarm_cap(&self) -> usize {
        self.normal_cap() * SWARM_FACTOR
    }

    /// Registers a stretched limb on `entity_id`, evicting the oldest entry in its queue
    /// if the configured budget is reached (`Limbs.add` 1:1).
    pub fn add(&mut self, entity_id: u32, spec: LimbSpec, reach_blocks: f64) -> LimbHandle {
        let cap = if spec.swarm {
            self.swarm_cap()
        } else {
            self.normal_cap()
        };

        loop {
            let qlen = if spec.swarm {
                self.swarm.len()
            } else {
                self.order.len()
            };
            if qlen < cap {
                break;
            }
            let evicted = if spec.swarm {
                self.swarm.pop_front()
            } else {
                self.order.pop_front()
            };
            if let Some(old_handle) = evicted {
                self.remove_entry(old_handle);
            }
        }

        self.next_id += 1;
        let handle = LimbHandle(self.next_id);
        let is_swarm = spec.swarm;
        let entry = ActiveLimb {
            handle,
            entity_id,
            spec,
            done: false,
            reach_blocks: reach_blocks.max(0.0),
            world_bounds: None,
        };
        self.entries.insert(handle, entry);
        self.by_entity.entry(entity_id).or_default().push(handle);
        if is_swarm {
            self.swarm.push_back(handle);
        } else {
            self.order.push_back(handle);
        }
        handle
    }

    /// Marks an active limb as finished (`Limbs.Active.end`).
    pub fn end(&mut self, handle: LimbHandle) {
        if let Some(e) = self.entries.get_mut(&handle) {
            e.done = true;
        }
    }

    /// Updates a live limb's world-space bounding box and reach for frustum culling.
    pub fn update_bounds(&mut self, handle: LimbHandle, reach_blocks: f64, bounds: EntityAabb) {
        if let Some(e) = self.entries.get_mut(&handle) {
            e.reach_blocks = reach_blocks.max(0.0);
            e.world_bounds = Some(bounds);
        }
    }

    /// Marks an entity as having its arms removed/hidden (`Limbs.addArmsGone`).
    pub fn set_arms_gone(&mut self, entity_id: u32, gone: bool) {
        if gone {
            self.arms_gone_entities.insert(entity_id, true);
        } else {
            self.arms_gone_entities.remove(&entity_id);
        }
    }

    /// Marks an extra held-item hide override (`HeldItemHide.hides`).
    pub fn set_held_item_hidden(&mut self, entity_id: u32, side: LimbSide, hidden: bool) {
        if hidden {
            self.extra_held_item_hide.insert((entity_id, side), true);
        } else {
            self.extra_held_item_hide.remove(&(entity_id, side));
        }
    }

    #[inline]
    pub fn any(&self) -> bool {
        !self.by_entity.is_empty()
    }

    /// Returns all active (including not-yet-pruned) limbs registered for `entity_id`.
    pub fn of(&self, entity_id: u32) -> Vec<&ActiveLimb> {
        self.by_entity
            .get(&entity_id)
            .map(|hs| hs.iter().filter_map(|h| self.entries.get(h)).collect())
            .unwrap_or_default()
    }

    /// Returns `true` if `entity_id` has an active stretched limb replacing `(limb, side)` (`Limbs.replaces` 1:1).
    pub fn replaces(&self, entity_id: u32, limb: LimbKind, side: LimbSide) -> bool {
        let Some(handles) = self.by_entity.get(&entity_id) else {
            return false;
        };
        handles.iter().any(|h| {
            self.entries
                .get(h)
                .is_some_and(|a| !a.done && a.spec.replaces && a.spec.limb == limb && a.spec.side == side)
        })
    }

    /// Returns `true` if the entity's arm is replaced or gone (`Limbs.replacesArm` 1:1).
    pub fn replaces_arm(&self, entity_id: u32, side: LimbSide) -> bool {
        self.arms_gone(entity_id) || self.replaces(entity_id, LimbKind::Arm, side)
    }

    #[inline]
    pub fn arms_gone(&self, entity_id: u32) -> bool {
        self.arms_gone_entities.get(&entity_id).copied().unwrap_or(false)
    }

    /// Returns `true` if the held item on `side` must be hidden (`LimbItemLayerMixin` 1:1).
    pub fn hides_held_item(&self, entity_id: u32, side: LimbSide) -> bool {
        (self.any() && self.replaces_arm(entity_id, side))
            || self.arms_gone(entity_id)
            || self
                .extra_held_item_hide
                .get(&(entity_id, side))
                .copied()
                .unwrap_or(false)
    }

    /// Computes complete vanilla limb, armor, and held-item hide flags (`Limbs.hideArmor` +
    /// `LimbClient.onRenderPlayerPre` + `LimbItemLayerMixin` 1:1).
    pub fn visibility_flags(&self, entity_id: u32) -> LimbVisibilityFlags {
        let arms_gone = self.arms_gone(entity_id);
        let rep_ra = self.replaces(entity_id, LimbKind::Arm, LimbSide::Right);
        let rep_la = self.replaces(entity_id, LimbKind::Arm, LimbSide::Left);
        let rep_rl = self.replaces(entity_id, LimbKind::Leg, LimbSide::Right);
        let rep_ll = self.replaces(entity_id, LimbKind::Leg, LimbSide::Left);
        let hide_stretched_armor = !self.config.show_armor_on_stretch;

        LimbVisibilityFlags {
            hide_right_arm: rep_ra || arms_gone,
            hide_left_arm: rep_la || arms_gone,
            hide_right_leg: rep_rl,
            hide_left_leg: rep_ll,
            hide_right_arm_armor: arms_gone || (hide_stretched_armor && rep_ra),
            hide_left_arm_armor: arms_gone || (hide_stretched_armor && rep_la),
            hide_right_leg_armor: hide_stretched_armor && rep_rl,
            hide_left_leg_armor: hide_stretched_armor && rep_ll,
            hide_right_held_item: self.hides_held_item(entity_id, LimbSide::Right),
            hide_left_held_item: self.hides_held_item(entity_id, LimbSide::Left),
        }
    }

    /// Frustum-cull bypass predicate (`Limbs.keepsDrawn` + `LimbCullMixin` 1:1):
    /// keeps the entity drawn whenever it has active stretched limbs and is within 128 blocks
    /// (`distance_squared < 16384.0`) of the camera.
    pub fn keeps_drawn(&self, entity_id: u32, entity_pos: DVec3, cam_pos: DVec3) -> bool {
        let Some(handles) = self.by_entity.get(&entity_id) else {
            return false;
        };
        let has_active = handles
            .iter()
            .any(|h| self.entries.get(h).is_some_and(|a| !a.done));
        has_active && entity_pos.distance_squared(cam_pos) < KEEPS_DRAWN_DIST_SQ
    }

    /// Returns the expanded frustum-culling AABB for `entity_id` covering all active stretched limbs
    /// (up to 60+ blocks, using exact `world_bounds` when present or inflating by `reach_blocks`).
    pub fn expanded_culling_aabb(&self, entity_id: u32, base_aabb: EntityAabb) -> EntityAabb {
        let Some(handles) = self.by_entity.get(&entity_id) else {
            return base_aabb;
        };
        let mut expanded = base_aabb;
        for h in handles {
            let Some(a) = self.entries.get(h) else {
                continue;
            };
            if a.done {
                continue;
            }
            if let Some(wb) = a.world_bounds {
                expanded = expanded.union(wb);
            } else if a.reach_blocks > 0.0 {
                expanded = expanded.union(base_aabb.inflate(a.reach_blocks));
            }
        }
        expanded
    }

    /// Clears all limbs for one entity (`Limbs.clear` 1:1).
    pub fn clear(&mut self, entity_id: u32) {
        if let Some(handles) = self.by_entity.remove(&entity_id) {
            for h in &handles {
                self.entries.remove(h);
            }
            self.order.retain(|h| !handles.contains(h));
            self.swarm.retain(|h| !handles.contains(h));
        }
    }

    /// Clears all state (`Limbs.clearAll`).
    pub fn clear_all(&mut self) {
        self.by_entity.clear();
        self.entries.clear();
        self.order.clear();
        self.swarm.clear();
        self.arms_gone_entities.clear();
        self.extra_held_item_hide.clear();
    }

    /// Removes finished limbs and limbs belonging to despawned entities (`Limbs.prune` 1:1).
    pub fn prune<F>(&mut self, mut is_entity_alive: F)
    where
        F: FnMut(u32) -> bool,
    {
        if self.by_entity.is_empty() {
            return;
        }
        let mut to_remove = Vec::new();
        for (&h, entry) in &self.entries {
            if entry.done || !is_entity_alive(entry.entity_id) {
                to_remove.push(h);
            }
        }
        for h in to_remove {
            self.remove_entry(h);
        }
        self.order.retain(|h| self.entries.contains_key(h));
        self.swarm.retain(|h| self.entries.contains_key(h));
    }

    fn remove_entry(&mut self, handle: LimbHandle) {
        if let Some(mut entry) = self.entries.remove(&handle) {
            entry.done = true;
            if let Some(list) = self.by_entity.get_mut(&entry.entity_id) {
                list.retain(|&h| h != handle);
                if list.is_empty() {
                    self.by_entity.remove(&entry.entity_id);
                }
            }
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_limb_vertices_valid(verts: &[LimbVertex], uv_strict_unit: bool, ctx: &str) {
        assert!(!verts.is_empty(), "{ctx}: expected non-empty vertex list");
        assert_eq!(verts.len() % 4, 0, "{ctx}: quad vertices must be a multiple of 4");
        for (i, v) in verts.iter().enumerate() {
            assert!(
                v.pos.iter().all(|x| x.is_finite()),
                "{ctx} vert {i}: non-finite pos {:?}",
                v.pos
            );
            assert!(
                v.normal.iter().all(|x| x.is_finite()),
                "{ctx} vert {i}: non-finite normal {:?}",
                v.normal
            );
            let nlen = Vec3::from_array(v.normal).length();
            assert!(
                (nlen - 1.0).abs() < 1.0e-3,
                "{ctx} vert {i}: non-unit normal length {nlen}"
            );
            assert!(
                v.uv.iter().all(|x| x.is_finite()),
                "{ctx} vert {i}: non-finite uv {:?}",
                v.uv
            );
            if uv_strict_unit {
                assert!(
                    v.uv[0] >= -1.0e-5 && v.uv[0] <= 1.0 + 1.0e-5,
                    "{ctx} vert {i}: u={} out of [0,1]",
                    v.uv[0]
                );
                assert!(
                    v.uv[1] >= -1.0e-5 && v.uv[1] <= 1.0 + 1.0e-5,
                    "{ctx} vert {i}: v={} out of [0,1]",
                    v.uv[1]
                );
            }
        }
    }

    #[test]
    fn limb_uv_classic_slim_legacy_and_hd_bounds() {
        // Classic 64x64 right & left arms/legs + overlays
        for model in [SkinModel::Classic, SkinModel::Slim] {
            for side in [LimbSide::Right, LimbSide::Left] {
                for limb in [LimbKind::Arm, LimbKind::Leg] {
                    for layer in [SkinLayer::Skin, SkinLayer::Overlay] {
                        for &(tw, th) in &[(64, 64), (128, 128), (256, 256)] {
                            let layout = LimbUv::resolve(model, side, limb, tw, th, layer).unwrap();
                            let expected_w =
                                if limb == LimbKind::Arm && model == SkinModel::Slim {
                                    3
                                } else {
                                    4
                                };
                            assert_eq!(layout.width, expected_w);
                            for face in 0..4 {
                                for corner in 0..2 {
                                    let u = layout.u_coord(layout.side_u(face, corner));
                                    assert!((0.0..=1.0).contains(&u), "u={u} out of bounds");
                                }
                            }
                            let v_top = layout.v_coord(layout.side_top());
                            let v_bot = layout.v_coord(layout.side_top() + 12.0);
                            assert!((0.0..=1.0).contains(&v_top));
                            assert!((0.0..=1.0).contains(&v_bot));
                            for top in [true, false] {
                                for [_, _, cu, cv] in layout.cap(top) {
                                    assert!((0.0..=1.0).contains(&layout.u_coord(cu)));
                                    assert!((0.0..=1.0).contains(&layout.v_coord(cv)));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Legacy 64x32 skin: overlay returns None, left limbs mirror right UVs.
        assert!(LimbUv::is_legacy(64, 32));
        assert!(
            LimbUv::resolve(
                SkinModel::Classic,
                LimbSide::Left,
                LimbKind::Arm,
                64,
                32,
                SkinLayer::Overlay
            )
            .is_none()
        );
        let left_legacy = LimbUv::resolve(
            SkinModel::Classic,
            LimbSide::Left,
            LimbKind::Arm,
            64,
            32,
            SkinLayer::Skin,
        )
        .unwrap();
        assert!(left_legacy.mirror);
        assert_eq!((left_legacy.u, left_legacy.v), (40, 16));
        assert!((left_legacy.v_coord(left_legacy.side_top() + 12.0) - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn limb_bands_and_fill_detection() {
        // Short limb (< 8 px): 3 scaled bands.
        let short = LimbBands::build(6.0, 1.0, 1.0, 4);
        assert_eq!(short.len(), 3);
        assert!((LimbBands::total_length(&short) - 6.0).abs() < 1.0e-4);

        // Long stretched limb (320 px = 20 blocks) with fill=6 (2-row repeat).
        let long = LimbBands::build(320.0, 1.0, 1.25, 6);
        assert!(long.len() > 10);
        for seg in &long {
            assert!(seg.s1 >= seg.s0);
            assert!(seg.r0 >= 0.0 && seg.r1 <= 12.0);
        }

        // LimbFill sleeve cuff detection on synthetic 64x64 classic right arm:
        // rows 4..=5 blue sleeve, rows 6..=7 skin tone -> first_row should pick row 6!
        let layout = LimbUv::resolve(
            SkinModel::Classic,
            LimbSide::Right,
            LimbKind::Arm,
            64,
            64,
            SkinLayer::Skin,
        )
        .unwrap();
        let mut px = vec![0xFFFF_D2B0u32; 64 * 64]; // warm skin tone
        for row in 4..=5 {
            let y = (layout.side_top() as usize) + row;
            for x in (layout.u as usize)..(layout.u as usize + 16) {
                px[y * 64 + x] = 0xFF10_20E0; // bright blue sleeve
            }
        }
        assert_eq!(LimbFill::first_row(&px, 64, 64, &layout), 6);
    }

    #[test]
    fn curved_and_straight_swept_meshes_have_finite_normals_and_valid_uvs() {
        // 1. Curved twisted arm (classic + slim) with tension, fist swell, overlay, and swirl.
        for model in [SkinModel::Classic, SkinModel::Slim] {
            let skin = LimbUv::resolve(
                model,
                LimbSide::Right,
                LimbKind::Arm,
                64,
                64,
                SkinLayer::Skin,
            )
            .unwrap();
            let overlay = LimbUv::resolve(
                model,
                LimbSide::Right,
                LimbKind::Arm,
                64,
                64,
                SkinLayer::Overlay,
            )
            .unwrap();

            let mut path = LimbPath::new();
            path.add(Vec3::new(0.0, 0.0, 0.0))
                .add(Vec3::new(12.0, 24.0, 18.0))
                .add(Vec3::new(-8.0, 56.0, 48.0))
                .add(Vec3::new(4.0, 80.0, 96.0))
                .start_direction(Vec3::Y)
                .build();
            assert!(path.length() > 100.0);

            let spec = LimbSpec {
                tension: 0.8,
                thinnest: 0.7,
                fist: 1.5,
                twist_turns: 2.5,
                spin: 0.4,
                overlay: true,
                has_swirl: true,
                swirl_scroll: 3.25,
                ..LimbSpec::default()
            };
            let pose = Mat4::from_scale(Vec3::splat(1.0 / 16.0));
            let built = StretchedLimbMesh::build(
                pose,
                &skin,
                Some(&overlay),
                &path,
                Vec3::X,
                &spec,
                5,
                4,
                [0x00F0_00F0, 0x00C0_00D0, 0x0080_0090],
                CinematicQuality::Ultra,
                None,
                None,
            );
            assert_limb_vertices_valid(&built.skin_quads, true, "curved skin");
            assert_limb_vertices_valid(&built.overlay_quads, true, "curved overlay");
            assert_limb_vertices_valid(&built.swirl_quads, false, "curved swirl");

            let tris = StretchedLimbMesh::quads_to_triangles(&built.skin_quads);
            assert_eq!(tris.len(), (built.skin_quads.len() / 4) * 6);
        }

        // 2. Straight 60-block (960 px) leg stretch with LimbFrame root alignment.
        let leg_skin = LimbUv::resolve(
            SkinModel::Classic,
            LimbSide::Left,
            LimbKind::Leg,
            64,
            64,
            SkinLayer::Skin,
        )
        .unwrap();
        let to_world = Mat4::from_scale(Vec3::splat(1.0 / 16.0));
        let frame = LimbFrame::new(
            leg_skin,
            to_world,
            DVec3::ZERO,
            Vec3::new(leg_skin.pivot_x, leg_skin.pivot_y, leg_skin.pivot_z),
            Quat::IDENTITY,
        );
        let mut path = LimbPath::new();
        path.add(Vec3::ZERO).add(Vec3::new(0.0, 12.0, 960.0));
        let n0 = frame.align_root_and_build(&mut path);
        assert!(n0.is_finite() && (n0.length() - 1.0).abs() < 1.0e-4);
        assert!(path.length() > 900.0);

        let built = StretchedLimbMesh::build(
            to_world,
            &leg_skin,
            None,
            &path,
            n0,
            &LimbSpec::default().leg(LimbSide::Left),
            4,
            4,
            [0x00F0_00F0; 3],
            CinematicQuality::High,
            None,
            None,
        );
        assert_limb_vertices_valid(&built.skin_quads, true, "60-block leg");
    }

    #[test]
    fn limbs_hide_flags_caps_and_60_block_culling_aabb() {
        let mut limbs = Limbs::new(LimbConfig {
            max_limbs: 4,
            quality: CinematicQuality::Medium, // halves cap -> 2 normal, 6 swarm
            show_armor_on_stretch: false,
        });
        assert_eq!(limbs.normal_cap(), 2);
        assert_eq!(limbs.swarm_cap(), 6);

        let player_id = 42;
        let h_ra = limbs.add(player_id, LimbSpec::default().arm(LimbSide::Right), 62.0);
        let _h_ll = limbs.add(player_id, LimbSpec::default().leg(LimbSide::Left), 25.0);

        // Armor & held item hiding when show_armor_on_stretch == false.
        let flags = limbs.visibility_flags(player_id);
        assert!(flags.hide_right_arm);
        assert!(!flags.hide_left_arm);
        assert!(flags.hide_left_leg);
        assert!(!flags.hide_right_leg);
        assert!(flags.hide_right_arm_armor);
        assert!(!flags.hide_left_arm_armor);
        assert!(flags.hide_left_leg_armor);
        assert!(flags.hide_right_held_item);
        assert!(!flags.hide_left_held_item);

        // Toggle show_armor_on_stretch -> armor stays visible while arm & held item stay hidden.
        limbs.config.show_armor_on_stretch = true;
        let flags_armor_shown = limbs.visibility_flags(player_id);
        assert!(!flags_armor_shown.hide_right_arm_armor);
        assert!(!flags_armor_shown.hide_left_leg_armor);
        assert!(flags_armor_shown.hide_right_arm);
        assert!(flags_armor_shown.hide_right_held_item);

        // arms_gone always hides both arm armor slots & both held items.
        limbs.set_arms_gone(player_id, true);
        let flags_gone = limbs.visibility_flags(player_id);
        assert!(flags_gone.hide_right_arm_armor && flags_gone.hide_left_arm_armor);
        assert!(flags_gone.hide_right_held_item && flags_gone.hide_left_held_item);
        limbs.set_arms_gone(player_id, false);

        // Frustum culling: keeps_drawn true within 128 blocks, false past 128 blocks.
        let p_pos = DVec3::new(0.0, 64.0, 0.0);
        assert!(limbs.keeps_drawn(player_id, p_pos, DVec3::new(100.0, 64.0, 0.0)));
        assert!(!limbs.keeps_drawn(player_id, p_pos, DVec3::new(130.0, 64.0, 0.0)));

        // Expanded AABB covers 62-block reach in every direction!
        let base = EntityAabb::player_at(p_pos);
        let expanded = limbs.expanded_culling_aabb(player_id, base);
        assert!(expanded.contains_point(DVec3::new(61.5, 65.0, 0.0)));
        assert!(expanded.contains_point(DVec3::new(-61.5, 65.0, 0.0)));
        assert!(expanded.size().x >= 124.0);

        // Normal FIFO cap evicts oldest (h_ra) when a 3rd normal limb is added.
        let _h_la = limbs.add(player_id, LimbSpec::default().arm(LimbSide::Left), 15.0);
        assert!(!limbs.replaces(player_id, LimbKind::Arm, LimbSide::Right));
        assert!(limbs.replaces(player_id, LimbKind::Arm, LimbSide::Left));

        // Swarm budget is separate (6 entries) and doesn't evict normal limbs.
        for _ in 0..8 {
            let mut s = LimbSpec::default();
            s.swarm = true;
            s.replaces = false;
            limbs.add(player_id, s, 20.0);
        }
        assert_eq!(limbs.swarm.len(), 6);
        assert_eq!(limbs.order.len(), 2);

        // Ending & pruning cleans up cleanly.
        limbs.end(h_ra);
        limbs.prune(|id| id == player_id);
        limbs.clear(player_id);
        assert!(!limbs.any());
    }
}
