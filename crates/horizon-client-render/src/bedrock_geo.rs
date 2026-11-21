//! Bedrock 1.12.0 geometry renderer, Bedrock 1.8.0 animation engine, and Grand Line
//! 213-clip custom player animation controller.
//!
//! 1:1 Rust port of Grand Line 0.4.0 (`devilfruits`):
//! - `client.model.geo.{GeoModel, GeoParser, GeoPose, GeoRenderer, GeoAnimation, GeoResources}`
//! - `client.animation.{Easing, PlayerAnimation, AnimationController, ItemTurn}`
#![forbid(unsafe_code)]

use glam::{Mat3, Mat4, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::f32::consts::{FRAC_PI_2, PI};
use std::fmt;
use std::fs;
use std::path::Path;

/// Ticks per second used to convert between Bedrock seconds and Minecraft game ticks.
pub const TICKS_PER_SECOND: f32 = 20.0;

/// Default right-hand third-person item rotation pivot in model pixels (`AnimationController.DEFAULT_PIVOT_RIGHT`).
pub const DEFAULT_ITEM_PIVOT_RIGHT: Vec3 = Vec3::new(-1.0, 10.0, -2.0);
/// Default left-hand third-person item rotation pivot in model pixels (`AnimationController.DEFAULT_PIVOT_LEFT`).
pub const DEFAULT_ITEM_PIVOT_LEFT: Vec3 = Vec3::new(1.0, 10.0, -2.0);
/// Default fraction of third-person item turn blended into first-person view (`AnimationController.FIRST_PERSON_SHARE`).
pub const DEFAULT_FIRST_PERSON_ITEM_SHARE: f32 = 0.5;

/// Errors produced when loading or parsing Bedrock models, Bedrock animations, or player animations.
#[derive(Debug, Clone)]
pub enum GeoError {
    Io(String),
    Json(String),
    InvalidFormat(String),
    BoneCycle { model_id: String },
}

impl fmt::Display for GeoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "geo I/O error: {msg}"),
            Self::Json(msg) => write!(f, "geo JSON error: {msg}"),
            Self::InvalidFormat(msg) => write!(f, "invalid geo format: {msg}"),
            Self::BoneCycle { model_id } => {
                write!(f, "{model_id}: bone hierarchy has a cycle")
            }
        }
    }
}

impl std::error::Error for GeoError {}

// ============================================================================
// Easing Curves (1:1 port of `com.devilfruits.client.animation.Easing`)
// ============================================================================

/// All 19 easing curves used across Grand Line's player animations (`snake_case`)
/// and Bedrock/GeckoLib geo animations (`camelCase` via [`gecko_easing`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    #[default]
    Linear,
    Catmullrom,
    Step,
    InSine,
    OutSine,
    InOutSine,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    OutCubic,
    InOutCubic,
    InExpo,
    OutExpo,
    InBack,
    OutBack,
    InOutBack,
    OutElastic,
    OutBounce,
}

impl Easing {
    /// Evaluates the easing curve at normalized parameter `t` (typically in `[0.0, 1.0]`).
    #[inline]
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Self::Linear | Self::Catmullrom => t,
            Self::Step => {
                if t < 1.0 {
                    0.0
                } else {
                    1.0
                }
            }
            Self::InSine => 1.0 - (t * FRAC_PI_2).cos(),
            Self::OutSine => (t * FRAC_PI_2).sin(),
            Self::InOutSine => -((PI * t).cos() - 1.0) * 0.5,
            Self::InQuad => t * t,
            Self::OutQuad => {
                let u = 1.0 - t;
                1.0 - u * u
            }
            Self::InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    let u = -2.0 * t + 2.0;
                    1.0 - (u * u) * 0.5
                }
            }
            Self::InCubic => t * t * t,
            Self::OutCubic => {
                let u = 1.0 - t;
                1.0 - u * u * u
            }
            Self::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let u = -2.0 * t + 2.0;
                    1.0 - (u * u * u) * 0.5
                }
            }
            Self::InExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0_f64.powf(10.0 * f64::from(t) - 10.0) as f32
                }
            }
            Self::OutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f64.powf(-10.0 * f64::from(t)) as f32
                }
            }
            Self::InBack => 2.70158 * t * t * t - 1.70158 * t * t,
            Self::OutBack => {
                let u = t - 1.0;
                1.0 + 2.70158 * u * u * u + 1.70158 * u * u
            }
            Self::InOutBack => {
                let c2: f64 = 2.5949094;
                let td = f64::from(t);
                if t < 0.5 {
                    let a = 2.0 * td;
                    ((a * a) * ((c2 + 1.0) * a - c2) * 0.5) as f32
                } else {
                    let a = 2.0 * td - 2.0;
                    (((a * a) * ((c2 + 1.0) * a + c2) + 2.0) * 0.5) as f32
                }
            }
            Self::OutElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let td = f64::from(t);
                    (2.0_f64.powf(-10.0 * td)
                        * ((td * 10.0 - 0.75) * (std::f64::consts::TAU / 3.0)).sin()
                        + 1.0) as f32
                }
            }
            Self::OutBounce => {
                let n = 7.5625_f32;
                let d = 2.75_f32;
                if t < 1.0 / d {
                    n * t * t
                } else if t < 2.0 / d {
                    let u = t - 1.5 / d;
                    n * u * u + 0.75
                } else if t < 2.5 / d {
                    let u = t - 2.25 / d;
                    n * u * u + 0.9375
                } else {
                    let u = t - 2.625 / d;
                    n * u * u + 0.984375
                }
            }
        }
    }

    /// Parses either `snake_case` (e.g. `"in_out_sine"`) or GeckoLib `camelCase`
    /// (e.g. `"easeInOutSine"`), falling back to [`Easing::Linear`] for unknown names.
    pub fn parse(name: &str) -> Self {
        let trimmed = name.trim();
        let normalized = gecko_easing(trimmed);
        match normalized.to_ascii_uppercase().as_str() {
            "LINEAR" => Self::Linear,
            "CATMULLROM" => Self::Catmullrom,
            "STEP" => Self::Step,
            "IN_SINE" => Self::InSine,
            "OUT_SINE" => Self::OutSine,
            "IN_OUT_SINE" => Self::InOutSine,
            "IN_QUAD" => Self::InQuad,
            "OUT_QUAD" => Self::OutQuad,
            "IN_OUT_QUAD" => Self::InOutQuad,
            "IN_CUBIC" => Self::InCubic,
            "OUT_CUBIC" => Self::OutCubic,
            "IN_OUT_CUBIC" => Self::InOutCubic,
            "IN_EXPO" => Self::InExpo,
            "OUT_EXPO" => Self::OutExpo,
            "IN_BACK" => Self::InBack,
            "OUT_BACK" => Self::OutBack,
            "IN_OUT_BACK" => Self::InOutBack,
            "OUT_ELASTIC" => Self::OutElastic,
            "OUT_BOUNCE" => Self::OutBounce,
            _ => Self::Linear,
        }
    }
}

/// Converts a GeckoLib camelCase easing name (`"easeInOutSine"`) to `snake_case`
/// (`"in_out_sine"`), matching `GeoAnimation.geckoEasing` 1:1.
pub fn gecko_easing(name: &str) -> String {
    let Some(rest) = name.strip_prefix("ease") else {
        return name.to_string();
    };
    let mut out = String::with_capacity(rest.len() + 4);
    for ch in rest.chars() {
        if ch.is_ascii_uppercase() && !out.is_empty() {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Uniform Catmull-Rom spline interpolation (`0.5 * (...)`), matching `GeoAnimation` and `PlayerAnimation`.
#[inline]
pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

#[inline]
fn catmull_rom_vec3(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        catmull_rom(p0.x, p1.x, p2.x, p3.x, t),
        catmull_rom(p0.y, p1.y, p2.y, p3.y, t),
        catmull_rom(p0.z, p1.z, p2.z, p3.z, t),
    )
}

// ============================================================================
// Bedrock 1.12.0 Geometry (1:1 port of `GeoModel`, `GeoParser`, `GeoPose`, `GeoRenderer`)
// ============================================================================

/// A single baked or posed Bedrock geometry vertex (8 `f32`s = `GeoModel.STRIDE`, 32 bytes).
#[repr(C)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct GeoVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

/// UV rectangle for one face of a per-face UV cube (`uv` + `uv_size`, supporting negative `uv_size`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeoFaceUv {
    pub uv: [f32; 2],
    pub uv_size: [f32; 2],
}

/// Per-face UV map on a Bedrock 1.12.0 cube.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeoPerFaceUv {
    pub north: Option<GeoFaceUv>,
    pub south: Option<GeoFaceUv>,
    pub east: Option<GeoFaceUv>,
    pub west: Option<GeoFaceUv>,
    pub up: Option<GeoFaceUv>,
    pub down: Option<GeoFaceUv>,
}

/// Cube UV specification: either Bedrock box UV `[u, v]` or explicit per-face UV rectangles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GeoUv {
    Box([f32; 2]),
    PerFace(GeoPerFaceUv),
}

impl Default for GeoUv {
    fn default() -> Self {
        Self::Box([0.0, 0.0])
    }
}

/// A parsed Bedrock 1.12.0 cube belonging to a [`GeoBone`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeoCube {
    #[serde(with = "serde_vec3")]
    pub origin: Vec3,
    #[serde(with = "serde_vec3")]
    pub size: Vec3,
    pub inflate: f32,
    pub mirror: bool,
    #[serde(with = "serde_opt_vec3")]
    pub pivot: Option<Vec3>,
    #[serde(with = "serde_opt_vec3")]
    pub rotation_deg: Option<Vec3>,
    pub uv: GeoUv,
}

/// One topologically sorted bone in a [`GeoModel`].
#[derive(Clone, Debug, PartialEq)]
pub struct GeoBone {
    pub name: String,
    pub index: usize,
    pub parent: Option<usize>,
    /// Bone pivot in Java model pixel space (`[-bx, by, bz]`).
    pub pivot: Vec3,
    /// Rest rotation in Java model radians (`[-rx_deg, -ry_deg, +rz_deg]`).
    pub rotation_rad: Vec3,
    /// Original Bedrock pivot in pixels.
    pub bedrock_pivot: Vec3,
    /// Original Bedrock rest rotation in degrees.
    pub bedrock_rotation_deg: Vec3,
    pub mirror: bool,
    pub cubes: Vec<GeoCube>,
    /// Pre-baked bone-local quad vertices (4 vertices per visible cube face).
    pub baked_vertices: Vec<GeoVertex>,
}

impl GeoBone {
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.baked_vertices.len()
    }
}

/// Parsed Bedrock 1.12.0 geometry model (`minecraft:geometry[0]`).
#[derive(Clone, Debug)]
pub struct GeoModel {
    pub id: String,
    pub identifier: String,
    pub texture_width: u32,
    pub texture_height: u32,
    pub visible_bounds_width: f32,
    pub visible_bounds_height: f32,
    pub visible_bounds_offset: Vec3,
    /// Bones in topological order (every parent appears before its children).
    pub bones: Vec<GeoBone>,
    by_name: HashMap<String, usize>,
}

impl GeoModel {
    /// Parses a Bedrock 1.12.0 `.geo.json` document from a JSON string.
    pub fn from_json_str(id: impl Into<String>, src: &str) -> Result<Self, GeoError> {
        let root: Value =
            serde_json::from_str(src).map_err(|e| GeoError::Json(e.to_string()))?;
        Self::from_json_value(id, &root)
    }

    /// Parses a Bedrock 1.12.0 `.geo.json` document from disk.
    pub fn from_file(id: impl Into<String>, path: &Path) -> Result<Self, GeoError> {
        let src = fs::read_to_string(path)
            .map_err(|e| GeoError::Io(format!("{}: {e}", path.display())))?;
        Self::from_json_str(id, &src)
    }

    /// Parses a Bedrock 1.12.0 `.geo.json` [`serde_json::Value`], matching `GeoParser.parse` 1:1.
    pub fn from_json_value(id: impl Into<String>, root: &Value) -> Result<Self, GeoError> {
        let id = id.into();
        let geo = root
            .get("minecraft:geometry")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(Value::as_object)
            .ok_or_else(|| {
                GeoError::InvalidFormat(format!("{id}: missing minecraft:geometry[0]"))
            })?;

        let desc = geo
            .get("description")
            .and_then(Value::as_object)
            .ok_or_else(|| GeoError::InvalidFormat(format!("{id}: missing description")))?;

        let identifier = desc
            .get("identifier")
            .and_then(Value::as_str)
            .unwrap_or(&id)
            .to_string();
        let texture_width = desc
            .get("texture_width")
            .and_then(Value::as_u64)
            .unwrap_or(64) as u32;
        let texture_height = desc
            .get("texture_height")
            .and_then(Value::as_u64)
            .unwrap_or(64) as u32;
        if texture_width == 0 || texture_height == 0 {
            return Err(GeoError::InvalidFormat(format!(
                "{id}: texture dimensions must be non-zero"
            )));
        }
        let visible_bounds_width = desc
            .get("visible_bounds_width")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;
        let visible_bounds_height = desc
            .get("visible_bounds_height")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;
        let visible_bounds_offset = json_vec3(desc.get("visible_bounds_offset"), 0.0);

        let raw_bones_arr = geo
            .get("bones")
            .and_then(Value::as_array)
            .ok_or_else(|| GeoError::InvalidFormat(format!("{id}: missing bones array")))?;

        struct RawBone<'a> {
            name: String,
            parent: Option<String>,
            json: &'a serde_json::Map<String, Value>,
        }

        // Preserve insertion order (matching Java's LinkedHashMap; duplicate bone names overwrite).
        let mut raw_order: Vec<String> = Vec::with_capacity(raw_bones_arr.len());
        let mut raw_map: HashMap<String, RawBone<'_>> = HashMap::with_capacity(raw_bones_arr.len());
        for entry in raw_bones_arr {
            let Some(obj) = entry.as_object() else {
                continue;
            };
            let name = obj
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| GeoError::InvalidFormat(format!("{id}: bone missing name")))?
                .to_string();
            let parent = obj
                .get("parent")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            if !raw_map.contains_key(&name) {
                raw_order.push(name.clone());
            }
            raw_map.insert(
                name.clone(),
                RawBone {
                    name,
                    parent,
                    json: obj,
                },
            );
        }

        // Topological sort: parents before children; missing parents treated as roots.
        let mut ordered: Vec<&RawBone<'_>> = Vec::with_capacity(raw_map.len());
        let mut index_by_name: HashMap<String, usize> = HashMap::with_capacity(raw_map.len());
        while ordered.len() < raw_map.len() {
            let mut progress = false;
            for name in &raw_order {
                let bone = &raw_map[name];
                if index_by_name.contains_key(&bone.name) {
                    continue;
                }
                let parent_ready = match &bone.parent {
                    None => true,
                    Some(p) => index_by_name.contains_key(p) || !raw_map.contains_key(p),
                };
                if parent_ready {
                    index_by_name.insert(bone.name.clone(), ordered.len());
                    ordered.push(bone);
                    progress = true;
                }
            }
            if !progress {
                return Err(GeoError::BoneCycle { model_id: id });
            }
        }

        let mut bones = Vec::with_capacity(ordered.len());
        for (i, rb) in ordered.iter().enumerate() {
            let b = rb.json;
            let bedrock_pivot = json_vec3(b.get("pivot"), 0.0);
            let bedrock_rotation_deg = json_vec3(b.get("rotation"), 0.0);
            let pivot = bedrock_to_java_point(bedrock_pivot);
            let rotation_rad = bedrock_to_java_rotation(bedrock_rotation_deg);
            let bone_mirror = b.get("mirror").and_then(Value::as_bool).unwrap_or(false);

            let mut cubes = Vec::new();
            let mut baked_vertices = Vec::new();
            if let Some(cubes_arr) = b.get("cubes").and_then(Value::as_array) {
                cubes.reserve(cubes_arr.len());
                baked_vertices.reserve(cubes_arr.len() * 24);
                for cube_val in cubes_arr {
                    if let Some(c_obj) = cube_val.as_object() {
                        let cube = parse_cube(c_obj, bone_mirror);
                        bake_cube(&cube, texture_width, texture_height, &mut baked_vertices);
                        cubes.push(cube);
                    }
                }
            }

            let parent = rb
                .parent
                .as_ref()
                .and_then(|p| index_by_name.get(p).copied());
            bones.push(GeoBone {
                name: rb.name.clone(),
                index: i,
                parent,
                pivot,
                rotation_rad,
                bedrock_pivot,
                bedrock_rotation_deg,
                mirror: bone_mirror,
                cubes,
                baked_vertices,
            });
        }

        Ok(Self {
            id,
            identifier,
            texture_width,
            texture_height,
            visible_bounds_width,
            visible_bounds_height,
            visible_bounds_offset,
            bones,
            by_name: index_by_name,
        })
    }

    /// Looks up a bone's topological index by name.
    #[inline]
    pub fn bone_index(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    #[inline]
    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    /// Convenience helper: poses the entire bone hierarchy with `base_transform` and `pose`,
    /// returning all visible world-space quad vertices (`4` per face).
    pub fn evaluate_world_vertices(&self, base_transform: Mat4, pose: &GeoPose) -> Vec<GeoVertex> {
        let mut renderer = GeoRenderer::new(self);
        renderer.pose(base_transform, pose);
        renderer.emit_quad_vertices(pose)
    }

    /// Convenience helper: poses the hierarchy and returns triangulated world-space vertices
    /// (`6` per quad face: `0,1,2, 0,2,3`).
    pub fn evaluate_world_triangles(
        &self,
        base_transform: Mat4,
        pose: &GeoPose,
    ) -> Vec<GeoVertex> {
        let mut renderer = GeoRenderer::new(self);
        renderer.pose(base_transform, pose);
        renderer.emit_triangle_vertices(pose)
    }
}

/// Converts a Bedrock point `[x, y, z]` into Java model coordinates `[-x, y, z]`.
#[inline]
pub fn bedrock_to_java_point(bedrock: Vec3) -> Vec3 {
    Vec3::new(-bedrock.x, bedrock.y, bedrock.z)
}

/// Converts Bedrock Euler degrees `[rx, ry, rz]` into Java model radians `[-rx, -ry, +rz]`.
#[inline]
pub fn bedrock_to_java_rotation(degrees: Vec3) -> Vec3 {
    Vec3::new(
        -degrees.x.to_radians(),
        -degrees.y.to_radians(),
        degrees.z.to_radians(),
    )
}

/// Converts Java model radians back into Bedrock Euler degrees.
#[inline]
pub fn java_to_bedrock_rotation_deg(rad: Vec3) -> Vec3 {
    Vec3::new(
        -rad.x.to_degrees(),
        -rad.y.to_degrees(),
        rad.z.to_degrees(),
    )
}

#[inline]
fn box_uv_size(size: f32) -> f32 {
    (size + 1.0e-7_f32).floor()
}

fn parse_cube(c: &serde_json::Map<String, Value>, bone_mirror: bool) -> GeoCube {
    let origin = json_vec3(c.get("origin"), 0.0);
    let size = json_vec3(c.get("size"), 0.0);
    let inflate = c.get("inflate").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let mirror = c
        .get("mirror")
        .and_then(Value::as_bool)
        .unwrap_or(bone_mirror);
    let rotation_deg = c.get("rotation").map(|v| json_vec3(Some(v), 0.0));
    let pivot = c.get("pivot").map(|v| json_vec3(Some(v), 0.0));

    let uv = match c.get("uv") {
        Some(Value::Object(faces)) => GeoUv::PerFace(GeoPerFaceUv {
            north: parse_face_uv(faces.get("north")),
            south: parse_face_uv(faces.get("south")),
            east: parse_face_uv(faces.get("east")),
            west: parse_face_uv(faces.get("west")),
            up: parse_face_uv(faces.get("up")),
            down: parse_face_uv(faces.get("down")),
        }),
        Some(Value::Array(arr)) => {
            let u = arr.first().and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let v = arr.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32;
            GeoUv::Box([u, v])
        }
        _ => GeoUv::Box([0.0, 0.0]),
    };

    GeoCube {
        origin,
        size,
        inflate,
        mirror,
        pivot,
        rotation_deg,
        uv,
    }
}

fn parse_face_uv(val: Option<&Value>) -> Option<GeoFaceUv> {
    let obj = val?.as_object()?;
    let uv_arr = obj.get("uv")?.as_array()?;
    let size_arr = obj.get("uv_size")?.as_array()?;
    Some(GeoFaceUv {
        uv: [
            uv_arr.first().and_then(Value::as_f64).unwrap_or(0.0) as f32,
            uv_arr.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32,
        ],
        uv_size: [
            size_arr.first().and_then(Value::as_f64).unwrap_or(0.0) as f32,
            size_arr.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32,
        ],
    })
}

/// Bakes one Bedrock 1.12.0 cube into bone-local quad vertices (`GeoParser.bakeCube` 1:1).
fn bake_cube(cube: &GeoCube, tw: u32, th: u32, out: &mut Vec<GeoVertex>) {
    let x0 = -(cube.origin.x + cube.size.x) - cube.inflate;
    let x1 = -cube.origin.x + cube.inflate;
    let y0 = cube.origin.y - cube.inflate;
    let y1 = cube.origin.y + cube.size.y + cube.inflate;
    let z0 = cube.origin.z - cube.inflate;
    let z1 = cube.origin.z + cube.size.z + cube.inflate;

    let mut rot_mat: Option<Mat3> = None;
    let mut rot_pivot = Vec3::ZERO;
    if let Some(rot_deg) = cube.rotation_deg {
        let r = bedrock_to_java_rotation(rot_deg);
        if r.x != 0.0 || r.y != 0.0 || r.z != 0.0 {
            rot_mat = Some(
                Mat3::from_rotation_z(r.z)
                    * Mat3::from_rotation_y(r.y)
                    * Mat3::from_rotation_x(r.x),
            );
            rot_pivot = bedrock_to_java_point(cube.pivot.unwrap_or(Vec3::ZERO));
        }
    }

    // Face UVs: [u0, v0, u1, v1] in texture pixels.
    let (front_uv, back_uv, right_uv, left_uv, top_uv, bottom_uv): (
        Option<[f32; 4]>,
        Option<[f32; 4]>,
        Option<[f32; 4]>,
        Option<[f32; 4]>,
        Option<[f32; 4]>,
        Option<[f32; 4]>,
    ) = match &cube.uv {
        GeoUv::PerFace(faces) => {
            let rect = |f: Option<GeoFaceUv>| {
                f.map(|g| [g.uv[0], g.uv[1], g.uv[0] + g.uv_size[0], g.uv[1] + g.uv_size[1]])
            };
            // Exact mapping from GeoParser.java:101-106:
            // front=north, back=south, left=east, right=west, top=up, bottom=down.
            (
                rect(faces.north),
                rect(faces.south),
                rect(faces.west),
                rect(faces.east),
                rect(faces.up),
                rect(faces.down),
            )
        }
        GeoUv::Box([u, v]) => {
            let u = *u;
            let v = *v;
            let w = box_uv_size(cube.size.x);
            let h = box_uv_size(cube.size.y);
            let d = box_uv_size(cube.size.z);

            let mut right = [u, v + d, u + d, v + d + h];
            let mut left = [u + d + w, v + d, u + 2.0 * d + w, v + d + h];
            if cube.mirror {
                std::mem::swap(&mut right, &mut left);
            }
            let mut front = [u + d, v + d, u + d + w, v + d + h];
            let mut back = [u + 2.0 * d + w, v + d, u + 2.0 * d + 2.0 * w, v + d + h];
            let mut top = [u + d, v, u + d + w, v + d];
            let mut bottom = [u + d + w, v, u + d + 2.0 * w, v + d];

            if cube.mirror {
                for f in [
                    &mut right,
                    &mut front,
                    &mut left,
                    &mut back,
                    &mut top,
                    &mut bottom,
                ] {
                    f.swap(0, 2);
                }
            }
            (
                Some(front),
                Some(back),
                Some(right),
                Some(left),
                Some(top),
                Some(bottom),
            )
        }
    };

    let thin_x = x1 - x0 < 1.0e-4;
    let thin_y = y1 - y0 < 1.0e-4;
    let thin_z = z1 - z0 < 1.0e-4;

    let inv_tw = 1.0 / (tw as f32);
    let inv_th = 1.0 / (th as f32);

    if !thin_x && !thin_y {
        emit_cube_face(
            out,
            front_uv,
            inv_tw,
            inv_th,
            rot_mat,
            rot_pivot,
            Vec3::new(0.0, 0.0, -1.0),
            [
                Vec3::new(x1, y1, z0),
                Vec3::new(x1, y0, z0),
                Vec3::new(x0, y0, z0),
                Vec3::new(x0, y1, z0),
            ],
        );
        emit_cube_face(
            out,
            back_uv,
            inv_tw,
            inv_th,
            rot_mat,
            rot_pivot,
            Vec3::new(0.0, 0.0, 1.0),
            [
                Vec3::new(x0, y1, z1),
                Vec3::new(x0, y0, z1),
                Vec3::new(x1, y0, z1),
                Vec3::new(x1, y1, z1),
            ],
        );
    }

    if !thin_z && !thin_y {
        emit_cube_face(
            out,
            right_uv,
            inv_tw,
            inv_th,
            rot_mat,
            rot_pivot,
            Vec3::new(1.0, 0.0, 0.0),
            [
                Vec3::new(x1, y1, z1),
                Vec3::new(x1, y0, z1),
                Vec3::new(x1, y0, z0),
                Vec3::new(x1, y1, z0),
            ],
        );
        emit_cube_face(
            out,
            left_uv,
            inv_tw,
            inv_th,
            rot_mat,
            rot_pivot,
            Vec3::new(-1.0, 0.0, 0.0),
            [
                Vec3::new(x0, y1, z0),
                Vec3::new(x0, y0, z0),
                Vec3::new(x0, y0, z1),
                Vec3::new(x0, y1, z1),
            ],
        );
    }

    if !thin_x && !thin_z {
        emit_cube_face(
            out,
            top_uv,
            inv_tw,
            inv_th,
            rot_mat,
            rot_pivot,
            Vec3::new(0.0, 1.0, 0.0),
            [
                Vec3::new(x1, y1, z1),
                Vec3::new(x1, y1, z0),
                Vec3::new(x0, y1, z0),
                Vec3::new(x0, y1, z1),
            ],
        );
        emit_cube_face(
            out,
            bottom_uv,
            inv_tw,
            inv_th,
            rot_mat,
            rot_pivot,
            Vec3::new(0.0, -1.0, 0.0),
            [
                Vec3::new(x1, y0, z0),
                Vec3::new(x1, y0, z1),
                Vec3::new(x0, y0, z1),
                Vec3::new(x0, y0, z0),
            ],
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_cube_face(
    out: &mut Vec<GeoVertex>,
    uv: Option<[f32; 4]>,
    inv_tw: f32,
    inv_th: f32,
    rot: Option<Mat3>,
    pivot: Vec3,
    normal: Vec3,
    corners: [Vec3; 4],
) {
    let Some([u0p, v0p, u1p, v1p]) = uv else {
        return;
    };
    let u0 = u0p * inv_tw;
    let v0 = v0p * inv_th;
    let u1 = u1p * inv_tw;
    let v1 = v1p * inv_th;
    let uvs = [[u0, v0], [u0, v1], [u1, v1], [u1, v0]];

    let n = match rot {
        Some(m) => m * normal,
        None => normal,
    };

    for (corner, uv_coord) in corners.into_iter().zip(uvs) {
        let p = match rot {
            Some(m) => m * (corner - pivot) + pivot,
            None => corner,
        };
        out.push(GeoVertex {
            pos: p.to_array(),
            uv: uv_coord,
            normal: n.to_array(),
        });
    }
}

/// Animated Bedrock-space pose for a single bone (`rotation_deg`, `position`, `scale`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BonePose {
    /// Animated Euler rotation offset in Bedrock degrees.
    #[serde(with = "serde_vec3")]
    pub rotation_deg: Vec3,
    /// Animated translation offset in Bedrock pixels.
    #[serde(with = "serde_vec3")]
    pub position: Vec3,
    /// Animated scale multiplier (`Vec3::ONE` = unscaled).
    #[serde(with = "serde_vec3")]
    pub scale: Vec3,
}

impl Default for BonePose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl BonePose {
    pub const IDENTITY: Self = Self {
        rotation_deg: Vec3::ZERO,
        position: Vec3::ZERO,
        scale: Vec3::ONE,
    };

    /// Linearly interpolates between two [`BonePose`]s (for smooth cross-fading).
    #[inline]
    pub fn lerp(self, other: Self, alpha: f32) -> Self {
        let a = alpha.clamp(0.0, 1.0);
        Self {
            rotation_deg: self.rotation_deg.lerp(other.rotation_deg, a),
            position: self.position.lerp(other.position, a),
            scale: self.scale.lerp(other.scale, a),
        }
    }

    /// Converts this pose's Bedrock degrees rotation into Java model radians.
    #[inline]
    pub fn java_rotation_rad(self) -> Vec3 {
        bedrock_to_java_rotation(self.rotation_deg)
    }

    /// Converts this pose's Bedrock pixel offset into Java model pixel coordinates.
    #[inline]
    pub fn java_position(self) -> Vec3 {
        bedrock_to_java_point(self.position)
    }
}

/// Mutable per-bone pose buffer matching `com.devilfruits.client.model.geo.GeoPose` 1:1.
#[derive(Clone, Debug)]
pub struct GeoPose {
    /// Per-bone rotation offsets in Java radians.
    pub rot: Vec<Vec3>,
    /// Per-bone translation offsets in Java model pixels.
    pub pos: Vec<Vec3>,
    /// Per-bone scale multipliers.
    pub scale: Vec<Vec3>,
    /// Per-bone visibility flags (hiding a parent hides its descendants).
    pub hidden: Vec<bool>,
}

impl GeoPose {
    pub fn new(model: &GeoModel) -> Self {
        let n = model.bone_count();
        Self {
            rot: vec![Vec3::ZERO; n],
            pos: vec![Vec3::ZERO; n],
            scale: vec![Vec3::ONE; n],
            hidden: vec![false; n],
        }
    }

    pub fn reset(&mut self) {
        self.rot.fill(Vec3::ZERO);
        self.pos.fill(Vec3::ZERO);
        self.scale.fill(Vec3::ONE);
        self.hidden.fill(false);
    }

    #[inline]
    pub fn add_rotation(&mut self, bone: usize, delta_rad: Vec3) {
        if let Some(r) = self.rot.get_mut(bone) {
            *r += delta_rad;
        }
    }

    #[inline]
    pub fn add_position(&mut self, bone: usize, delta_px: Vec3) {
        if let Some(p) = self.pos.get_mut(bone) {
            *p += delta_px;
        }
    }

    #[inline]
    pub fn set_scale(&mut self, bone: usize, s: f32) {
        if let Some(sc) = self.scale.get_mut(bone) {
            *sc = Vec3::splat(s);
        }
    }

    #[inline]
    pub fn set_scale_vec(&mut self, bone: usize, s: Vec3) {
        if let Some(sc) = self.scale.get_mut(bone) {
            *sc = s;
        }
    }

    #[inline]
    pub fn hide(&mut self, bone: usize, hide: bool) {
        if let Some(h) = self.hidden.get_mut(bone) {
            *h = hide;
        }
    }

    /// Writes a Bedrock-space [`BonePose`] into a bone slot (converting to Java space).
    #[inline]
    pub fn set_bone_pose(&mut self, bone: usize, bp: BonePose) {
        if bone < self.rot.len() {
            self.rot[bone] = bp.java_rotation_rad();
            self.pos[bone] = bp.java_position();
            self.scale[bone] = bp.scale;
        }
    }

    /// Reads a bone slot back as a Bedrock-space [`BonePose`].
    #[inline]
    pub fn bone_pose(&self, bone: usize) -> BonePose {
        if bone < self.rot.len() {
            BonePose {
                rotation_deg: java_to_bedrock_rotation_deg(self.rot[bone]),
                position: bedrock_to_java_point(self.pos[bone]),
                scale: self.scale[bone],
            }
        } else {
            BonePose::IDENTITY
        }
    }
}

/// Hierarchical bone matrix evaluator and posed mesh emitter (`GeoRenderer.java` 1:1).
#[derive(Clone, Debug)]
pub struct GeoRenderer<'a> {
    model: &'a GeoModel,
    matrices: Vec<Mat4>,
    normals: Vec<Mat3>,
}

impl<'a> GeoRenderer<'a> {
    pub fn new(model: &'a GeoModel) -> Self {
        let n = model.bone_count();
        Self {
            model,
            matrices: vec![Mat4::IDENTITY; n],
            normals: vec![Mat3::IDENTITY; n],
        }
    }

    #[inline]
    pub fn model(&self) -> &'a GeoModel {
        self.model
    }

    #[inline]
    pub fn bone_matrix(&self, bone: usize) -> Mat4 {
        self.matrices[bone]
    }

    #[inline]
    pub fn bone_normal(&self, bone: usize) -> Mat3 {
        self.normals[bone]
    }

    /// Computes world/model-space bone matrices and normal matrices for all bones (`GeoRenderer.pose`).
    pub fn pose(&mut self, base: Mat4, pose: &GeoPose) {
        let base_normal = extract_normal_mat3(base);
        self.pose_with_normal(base, base_normal, pose);
    }

    /// Computes bone matrices using an explicit base normal matrix (matching `PoseStack.Pose`).
    pub fn pose_with_normal(&mut self, base: Mat4, base_normal: Mat3, pose: &GeoPose) {
        for bone in &self.model.bones {
            let i = bone.index;
            let (mut m, mut nm) = match bone.parent {
                Some(parent) => (self.matrices[parent], self.normals[parent]),
                None => (base, base_normal),
            };

            let rot_off = pose.rot.get(i).copied().unwrap_or(Vec3::ZERO);
            let pos_off = pose.pos.get(i).copied().unwrap_or(Vec3::ZERO);
            let sc = pose.scale.get(i).copied().unwrap_or(Vec3::ONE);

            let r = bone.rotation_rad + rot_off;
            let t_pivot = bone.pivot + pos_off;

            m *= Mat4::from_translation(t_pivot);
            if r.z != 0.0 {
                m *= Mat4::from_rotation_z(r.z);
                nm *= Mat3::from_rotation_z(r.z);
            }
            if r.y != 0.0 {
                m *= Mat4::from_rotation_y(r.y);
                nm *= Mat3::from_rotation_y(r.y);
            }
            if r.x != 0.0 {
                m *= Mat4::from_rotation_x(r.x);
                nm *= Mat3::from_rotation_x(r.x);
            }
            if sc != Vec3::ONE {
                m *= Mat4::from_scale(sc);
            }
            m *= Mat4::from_translation(-bone.pivot);

            self.matrices[i] = m;
            self.normals[i] = nm;
        }
    }

    /// Emits posed world-space quad vertices (4 per visible face) for all non-hidden bones.
    pub fn emit_quad_vertices(&self, pose: &GeoPose) -> Vec<GeoVertex> {
        let total: usize = self.model.bones.iter().map(GeoBone::vertex_count).sum();
        let mut out = Vec::with_capacity(total);
        self.emit_quad_vertices_into(pose, &mut out);
        out
    }

    /// Appends posed quad vertices into `out`, propagating parent `hidden` flags to descendants.
    pub fn emit_quad_vertices_into(&self, pose: &GeoPose, out: &mut Vec<GeoVertex>) {
        let n = self.model.bone_count();
        let mut eff_hidden = vec![false; n];
        for bone in &self.model.bones {
            let hidden = pose.hidden.get(bone.index).copied().unwrap_or(false)
                || bone.parent.is_some_and(|p| eff_hidden[p]);
            eff_hidden[bone.index] = hidden;
            if !hidden {
                self.emit_bone_quads(bone, out);
            }
        }
    }

    /// Emits posed world-space triangle vertices (`6` per quad face: `0, 1, 2, 0, 2, 3`).
    pub fn emit_triangle_vertices(&self, pose: &GeoPose) -> Vec<GeoVertex> {
        let quads = self.emit_quad_vertices(pose);
        quads_to_triangles(&quads)
    }

    /// Emits posed quad vertices for a single bone.
    pub fn emit_bone_quads(&self, bone: &GeoBone, out: &mut Vec<GeoVertex>) {
        if bone.baked_vertices.is_empty() {
            return;
        }
        let m = self.matrices[bone.index];
        let nm = self.normals[bone.index];
        for v in &bone.baked_vertices {
            let p = m.transform_point3(Vec3::from_array(v.pos));
            let mut n = nm * Vec3::from_array(v.normal);
            let len = n.length();
            if len > 1.0e-6 {
                n /= len;
            }
            out.push(GeoVertex {
                pos: p.to_array(),
                uv: v.uv,
                normal: n.to_array(),
            });
        }
    }
}

/// Extracts an orthonormalized normal matrix from `base` without producing `NaN` on singular scales.
fn extract_normal_mat3(m: Mat4) -> Mat3 {
    let m3 = Mat3::from_mat4(m);
    let det = m3.determinant();
    if det.abs() > 1.0e-8 && det.is_finite() {
        m3.inverse().transpose()
    } else {
        m3
    }
}

/// Expands quad-list vertices (`4` per face) into a triangle list (`6` per face).
pub fn quads_to_triangles(quads: &[GeoVertex]) -> Vec<GeoVertex> {
    let mut tris = Vec::with_capacity((quads.len() / 4) * 6);
    for quad in quads.chunks_exact(4) {
        tris.push(quad[0]);
        tris.push(quad[1]);
        tris.push(quad[2]);
        tris.push(quad[0]);
        tris.push(quad[2]);
        tris.push(quad[3]);
    }
    tris
}

// ============================================================================
// Bedrock 1.8.0 Animations (1:1 port of `com.devilfruits.client.model.geo.GeoAnimation`)
// ============================================================================

/// Loop behavior specified by a Bedrock 1.8.0 animation clip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopMode {
    #[default]
    Once,
    Loop,
    HoldOnLastFrame,
}

impl LoopMode {
    #[inline]
    pub fn is_loop(self) -> bool {
        matches!(self, Self::Loop)
    }

    #[inline]
    pub fn is_hold_on_last_frame(self) -> bool {
        matches!(self, Self::HoldOnLastFrame)
    }
}

/// Interpolation mode for a Bedrock 1.8.0 keyframe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LerpMode {
    #[default]
    Linear,
    CatmullRom,
    Step,
}

/// One keyframe in a Bedrock 1.8.0 [`Track`] (time in seconds, values in Bedrock space).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    /// Timestamp in seconds.
    pub time_sec: f32,
    /// Outgoing / primary Bedrock value (`vector`, `post`, or `pre`).
    #[serde(with = "serde_vec3")]
    pub value: Vec3,
    /// Optional incoming value for discontinuous `pre`/`post` Bedrock keyframes.
    #[serde(with = "serde_opt_vec3")]
    pub pre: Option<Vec3>,
    pub lerp_mode: LerpMode,
    pub easing: Easing,
}

/// Sorted channel of [`Keyframe`]s for a single bone property (`rotation`, `position`, or `scale`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub keyframes: Vec<Keyframe>,
    #[serde(with = "serde_vec3")]
    pub default_value: Vec3,
}

impl Track {
    /// Samples the track at `time_sec` (seconds) in Bedrock space, matching `GeoAnimation.Track.sample` 1:1.
    pub fn sample(&self, time_sec: f32) -> Vec3 {
        let kfs = &self.keyframes;
        let n = kfs.len();
        if n == 0 {
            return self.default_value;
        }
        if n == 1 || time_sec <= kfs[0].time_sec {
            return kfs[0].value;
        }
        if time_sec >= kfs[n - 1].time_sec {
            return kfs[n - 1].value;
        }

        let mut i = 0;
        while i + 1 < n && kfs[i + 1].time_sec <= time_sec {
            i += 1;
        }

        let dt = (kfs[i + 1].time_sec - kfs[i].time_sec).max(1.0e-5);
        let raw_p = (time_sec - kfs[i].time_sec) / dt;

        if kfs[i].lerp_mode == LerpMode::Step {
            return kfs[i].value;
        }

        let p = kfs[i + 1].easing.apply(raw_p.clamp(0.0, 1.0));
        let catmull =
            kfs[i].lerp_mode == LerpMode::CatmullRom || kfs[i + 1].lerp_mode == LerpMode::CatmullRom;

        let p1 = kfs[i].value;
        let p2 = kfs[i + 1].pre.unwrap_or(kfs[i + 1].value);
        if catmull {
            let p0 = kfs[i.saturating_sub(1)].value;
            let p3 = kfs[(i + 2).min(n - 1)].value;
            catmull_rom_vec3(p0, p1, p2, p3, p)
        } else {
            p1 + (p2 - p1) * p
        }
    }

    #[inline]
    pub fn max_time_sec(&self) -> f32 {
        self.keyframes.last().map_or(0.0, |k| k.time_sec)
    }
}

/// Animated tracks (`rotation`, `position`, `scale`) for a single bone in a [`GeoAnimationClip`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoneTimeline {
    pub bone: String,
    pub rotation: Option<Track>,
    pub position: Option<Track>,
    pub scale: Option<Track>,
}

impl BoneTimeline {
    /// Evaluates this bone's Bedrock-space [`BonePose`] at `time_sec` (seconds).
    pub fn evaluate(&self, time_sec: f32) -> BonePose {
        BonePose {
            rotation_deg: self
                .rotation
                .as_ref()
                .map_or(Vec3::ZERO, |t| t.sample(time_sec)),
            position: self
                .position
                .as_ref()
                .map_or(Vec3::ZERO, |t| t.sample(time_sec)),
            scale: self.scale.as_ref().map_or(Vec3::ONE, |t| t.sample(time_sec)),
        }
    }
}

/// One Bedrock 1.8.0 animation clip (`GeoAnimation.java` 1:1).
#[derive(Clone, Debug, PartialEq)]
pub struct GeoAnimationClip {
    /// Full clip key from the JSON file (e.g. `"animation.champion.idle"`).
    pub name: String,
    /// Resource-style short clip id (e.g. `"doru_doru/champion/idle"`).
    pub id: String,
    pub length_seconds: f32,
    pub length_ticks: f32,
    pub loop_mode: LoopMode,
    /// Crossfade fade-in duration in ticks (`devilfruits:fade_in`, default `3.0`).
    pub fade_in: f32,
    /// Crossfade fade-out duration in ticks (`devilfruits:fade_out`, default `4.0`).
    pub fade_out: f32,
    /// Named event markers (`devilfruits:markers`), in ticks/seconds as authored in the clip.
    pub markers: BTreeMap<String, f32>,
    pub timelines: Vec<BoneTimeline>,
    by_bone: HashMap<String, usize>,
}

impl GeoAnimationClip {
    /// Resolves `elapsed_seconds` according to `loop_mode`.
    pub fn resolve_time_seconds(&self, elapsed_seconds: f32) -> f32 {
        let t = elapsed_seconds.max(0.0);
        if self.loop_mode.is_loop() && self.length_seconds > 0.0 {
            t % self.length_seconds
        } else if self.length_seconds > 0.0 {
            t.min(self.length_seconds)
        } else {
            t
        }
    }

    /// Resolves `elapsed_ticks` according to `loop_mode`.
    pub fn resolve_time_ticks(&self, elapsed_ticks: f32) -> f32 {
        self.resolve_time_seconds(elapsed_ticks / TICKS_PER_SECOND) * TICKS_PER_SECOND
    }

    /// Looks up a named `devilfruits:markers` timestamp with fallback (`GeoAnimation.marker`).
    #[inline]
    pub fn marker(&self, name: &str, fallback: f32) -> f32 {
        self.markers.get(name).copied().unwrap_or(fallback)
    }

    /// Samples a single bone's Bedrock-space [`BonePose`] at `time_sec`.
    pub fn sample_bone(&self, bone_name: &str, time_sec: f32) -> BonePose {
        let t = self.resolve_time_seconds(time_sec);
        self.by_bone
            .get(bone_name)
            .map_or(BonePose::IDENTITY, |&idx| self.timelines[idx].evaluate(t))
    }

    /// Samples every animated bone's Bedrock-space [`BonePose`] at `time_sec`.
    pub fn sample_all(&self, time_sec: f32) -> HashMap<String, BonePose> {
        let t = self.resolve_time_seconds(time_sec);
        let mut out = HashMap::with_capacity(self.timelines.len());
        for bt in &self.timelines {
            out.insert(bt.bone.clone(), bt.evaluate(t));
        }
        out
    }

    /// Additively layers this clip onto `pose` (`GeoAnimation.add` 1:1).
    pub fn add_to_pose(&self, model: &GeoModel, pose: &mut GeoPose, time_ticks: f32, weight: f32) {
        self.apply_to_pose(model, pose, time_ticks, weight, false);
    }

    /// Blends (overrides) this clip onto `pose` (`GeoAnimation.blend` 1:1).
    pub fn blend_to_pose(
        &self,
        model: &GeoModel,
        pose: &mut GeoPose,
        time_ticks: f32,
        weight: f32,
    ) {
        self.apply_to_pose(model, pose, time_ticks, weight, true);
    }

    fn apply_to_pose(
        &self,
        model: &GeoModel,
        pose: &mut GeoPose,
        time_ticks: f32,
        weight: f32,
        override_mode: bool,
    ) {
        if weight <= 0.001 {
            return;
        }
        let seconds = time_ticks / TICKS_PER_SECOND;
        for bt in &self.timelines {
            let Some(bone_idx) = model.bone_index(&bt.bone) else {
                continue;
            };
            if let Some(rot_track) = &bt.rotation {
                let v = bedrock_to_java_rotation(rot_track.sample(seconds));
                mix_vec3(&mut pose.rot[bone_idx], v, weight, override_mode);
            }
            if let Some(pos_track) = &bt.position {
                let v = bedrock_to_java_point(pos_track.sample(seconds));
                mix_vec3(&mut pose.pos[bone_idx], v, weight, override_mode);
            }
            if let Some(scale_track) = &bt.scale {
                let v = scale_track.sample(seconds);
                if override_mode {
                    mix_vec3(&mut pose.scale[bone_idx], v, weight, true);
                } else {
                    let s = &mut pose.scale[bone_idx];
                    s.x *= 1.0 + (v.x - 1.0) * weight;
                    s.y *= 1.0 + (v.y - 1.0) * weight;
                    s.z *= 1.0 + (v.z - 1.0) * weight;
                }
            }
        }
    }

    /// Smoothly cross-fades two clips (`from` -> `to`) at factor `alpha` in `[0.0, 1.0]` into `pose`.
    pub fn crossfade_into_pose(
        from: &Self,
        from_time_ticks: f32,
        to: &Self,
        to_time_ticks: f32,
        alpha: f32,
        model: &GeoModel,
        pose: &mut GeoPose,
    ) {
        let a = alpha.clamp(0.0, 1.0);
        pose.reset();
        let w_from = 1.0 - a;
        if w_from > 0.001 {
            from.blend_to_pose(model, pose, from_time_ticks, w_from);
        }
        if a > 0.001 {
            to.blend_to_pose(model, pose, to_time_ticks, a);
        }
    }

    /// Smoothly cross-fades two clips into Bedrock-space [`BonePose`] maps.
    pub fn crossfade_bone_poses(
        from: &Self,
        from_time_sec: f32,
        to: &Self,
        to_time_sec: f32,
        alpha: f32,
    ) -> HashMap<String, BonePose> {
        let a = alpha.clamp(0.0, 1.0);
        let p_from = from.sample_all(from_time_sec);
        let p_to = to.sample_all(to_time_sec);
        let mut out = HashMap::with_capacity(p_from.len() + p_to.len());
        for (bone, bp_from) in &p_from {
            let bp_to = p_to.get(bone).copied().unwrap_or(BonePose::IDENTITY);
            out.insert(bone.clone(), bp_from.lerp(bp_to, a));
        }
        for (bone, bp_to) in &p_to {
            if !out.contains_key(bone) {
                out.insert(bone.clone(), BonePose::IDENTITY.lerp(*bp_to, a));
            }
        }
        out
    }
}

#[inline]
fn mix_vec3(target: &mut Vec3, v: Vec3, w: f32, override_mode: bool) {
    if override_mode {
        *target += (v - *target) * w;
    } else {
        *target += v * w;
    }
}

/// Parsed collection of Bedrock 1.8.0 animation clips from a single `.animation.json` file.
#[derive(Clone, Debug)]
pub struct GeoAnimationSet {
    pub format_version: String,
    pub clips: Vec<GeoAnimationClip>,
    by_name: HashMap<String, usize>,
    by_id: HashMap<String, usize>,
}

impl GeoAnimationSet {
    /// Parses a Bedrock 1.8.0 `.animation.json` file from a JSON string.
    ///
    /// `prefix` matches `GeoResources.Animations` (e.g. `"doru_doru/champion/"`).
    pub fn from_json_str(prefix: &str, src: &str) -> Result<Self, GeoError> {
        let root: Value =
            serde_json::from_str(src).map_err(|e| GeoError::Json(e.to_string()))?;
        Self::from_json_value(prefix, &root)
    }

    /// Parses a Bedrock 1.8.0 `.animation.json` file from disk.
    pub fn from_file(prefix: &str, path: &Path) -> Result<Self, GeoError> {
        let src = fs::read_to_string(path)
            .map_err(|e| GeoError::Io(format!("{}: {e}", path.display())))?;
        Self::from_json_str(prefix, &src)
    }

    /// Parses a Bedrock 1.8.0 `.animation.json` [`serde_json::Value`], matching `GeoAnimation.parseFile` 1:1.
    pub fn from_json_value(prefix: &str, root: &Value) -> Result<Self, GeoError> {
        let format_version = root
            .get("format_version")
            .and_then(Value::as_str)
            .unwrap_or("1.8.0")
            .to_string();
        let animations = root
            .get("animations")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                GeoError::InvalidFormat("geo animation missing 'animations' object".to_string())
            })?;

        let mut clips = Vec::with_capacity(animations.len());
        let mut by_name = HashMap::with_capacity(animations.len());
        let mut by_id = HashMap::with_capacity(animations.len());

        for (clip_name, clip_val) in animations {
            let clip_obj = clip_val.as_object().ok_or_else(|| {
                GeoError::InvalidFormat(format!("clip {clip_name} is not an object"))
            })?;
            let short_name = strip_animation_prefix(clip_name);
            let id = format!("{prefix}{}", short_name.to_ascii_lowercase());
            let clip = parse_geo_clip(clip_name.clone(), id.clone(), clip_obj)?;
            let idx = clips.len();
            by_name.insert(clip_name.clone(), idx);
            by_id.insert(id, idx);
            clips.push(clip);
        }

        Ok(Self {
            format_version,
            clips,
            by_name,
            by_id,
        })
    }

    pub fn get_by_name(&self, name: &str) -> Option<&GeoAnimationClip> {
        self.by_name.get(name).map(|&i| &self.clips[i])
    }

    pub fn get_by_id(&self, id: &str) -> Option<&GeoAnimationClip> {
        self.by_id.get(id).map(|&i| &self.clips[i])
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }
}

/// Strips `"animation.<entity>."` matching `GeoAnimation.java:139`.
fn strip_animation_prefix(name: &str) -> &str {
    if let Some(after_anim) = name.strip_prefix("animation.")
        && let Some(dot) = after_anim.find('.')
        && dot > 0
    {
        return &after_anim[dot + 1..];
    }
    name
}

fn parse_geo_clip(
    name: String,
    id: String,
    json: &serde_json::Map<String, Value>,
) -> Result<GeoAnimationClip, GeoError> {
    let loop_mode = match json.get("loop") {
        Some(Value::Bool(true)) => LoopMode::Loop,
        Some(Value::String(s)) if s == "hold_on_last_frame" => LoopMode::HoldOnLastFrame,
        _ => LoopMode::Once,
    };

    let mut length_sec = 0.0_f32;
    let mut timelines = Vec::new();
    let mut by_bone = HashMap::new();

    if let Some(bones_obj) = json.get("bones").and_then(Value::as_object) {
        timelines.reserve(bones_obj.len());
        by_bone.reserve(bones_obj.len());
        for (bone_name, bone_val) in bones_obj {
            let Some(b) = bone_val.as_object() else {
                continue;
            };
            let rotation = b
                .get("rotation")
                .map(|v| parse_geo_track(v, Vec3::ZERO))
                .transpose()?;
            let position = b
                .get("position")
                .map(|v| parse_geo_track(v, Vec3::ZERO))
                .transpose()?;
            let scale = b
                .get("scale")
                .map(|v| parse_geo_track(v, Vec3::ONE))
                .transpose()?;

            for t in [&rotation, &position, &scale].into_iter().flatten() {
                length_sec = length_sec.max(t.max_time_sec());
            }

            let idx = timelines.len();
            by_bone.insert(bone_name.clone(), idx);
            timelines.push(BoneTimeline {
                bone: bone_name.clone(),
                rotation,
                position,
                scale,
            });
        }
    }

    if let Some(explicit_len) = json.get("animation_length").and_then(Value::as_f64) {
        length_sec = explicit_len as f32;
    }

    let fade_in = json
        .get("devilfruits:fade_in")
        .and_then(Value::as_f64)
        .unwrap_or(3.0) as f32;
    let fade_out = json
        .get("devilfruits:fade_out")
        .and_then(Value::as_f64)
        .unwrap_or(4.0) as f32;

    let mut markers = BTreeMap::new();
    if let Some(markers_obj) = json.get("devilfruits:markers").and_then(Value::as_object) {
        for (m_name, m_val) in markers_obj {
            if let Some(t) = m_val.as_f64() {
                markers.insert(m_name.clone(), t as f32);
            }
        }
    }

    Ok(GeoAnimationClip {
        name,
        id,
        length_seconds: length_sec,
        length_ticks: length_sec * TICKS_PER_SECOND,
        loop_mode,
        fade_in,
        fade_out,
        markers,
        timelines,
        by_bone,
    })
}

fn is_value_object(obj: &serde_json::Map<String, Value>) -> bool {
    obj.contains_key("vector") || obj.contains_key("post") || obj.contains_key("pre")
}

fn parse_geo_track(val: &Value, default_value: Vec3) -> Result<Track, GeoError> {
    let mut raw_frames: Vec<(f32, &Value)> = Vec::new();
    match val {
        Value::Object(obj) if !is_value_object(obj) => {
            raw_frames.reserve(obj.len());
            for (k, v) in obj {
                let ts = k.parse::<f32>().map_err(|_| {
                    GeoError::InvalidFormat(format!("invalid keyframe timestamp '{k}'"))
                })?;
                raw_frames.push((ts, v));
            }
        }
        _ => {
            raw_frames.push((0.0, val));
        }
    }

    raw_frames.sort_by(|a, b| a.0.total_cmp(&b.0));

    let def_scalar = default_value.x;
    let mut keyframes = Vec::with_capacity(raw_frames.len());
    for (time_sec, frame_val) in raw_frames {
        let mut lerp_mode = LerpMode::Linear;
        let mut easing = Easing::Linear;
        let mut pre: Option<Vec3> = None;
        let value = if let Some(obj) = frame_val.as_object() {
            if let Some(m) = obj.get("lerp_mode").and_then(Value::as_str) {
                lerp_mode = match m {
                    "catmullrom" => LerpMode::CatmullRom,
                    "step" => LerpMode::Step,
                    _ => LerpMode::Linear,
                };
            }
            if let Some(e) = obj.get("easing").and_then(Value::as_str) {
                easing = Easing::parse(e);
            }
            let primary_val = obj
                .get("vector")
                .or_else(|| obj.get("post"))
                .or_else(|| obj.get("pre"));
            if obj.contains_key("pre") && (obj.contains_key("post") || obj.contains_key("vector")) {
                pre = Some(parse_geo_vec(obj.get("pre"), def_scalar));
            }
            parse_geo_vec(primary_val, def_scalar)
        } else {
            parse_geo_vec(Some(frame_val), def_scalar)
        };

        keyframes.push(Keyframe {
            time_sec,
            value,
            pre,
            lerp_mode,
            easing,
        });
    }

    Ok(Track {
        keyframes,
        default_value,
    })
}

fn parse_geo_vec(val: Option<&Value>, def: f32) -> Vec3 {
    let Some(v) = val else {
        return Vec3::splat(def);
    };
    match v {
        Value::Number(n) => Vec3::splat(n.as_f64().map_or(def, |f| f as f32)),
        Value::String(s) => Vec3::splat(s.trim().parse::<f32>().unwrap_or(def)),
        Value::Array(arr) => Vec3::new(
            json_f32(arr.first(), def),
            json_f32(arr.get(1), def),
            json_f32(arr.get(2), def),
        ),
        _ => Vec3::splat(def),
    }
}

// ============================================================================
// Asset Library (`GeoLibrary` — matches `GeoResources` + `AnimationLoader`)
// ============================================================================

/// In-memory library of parsed Bedrock 1.12.0 models, Bedrock 1.8.0 geo animation clips,
/// and Grand Line custom player animations.
#[derive(Clone, Debug, Default)]
pub struct GeoLibrary {
    pub models: HashMap<String, GeoModel>,
    pub geo_animations: HashMap<String, GeoAnimationClip>,
    pub player_animations: HashMap<String, PlayerAnimation>,
    generation: u64,
}

impl GeoLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    #[inline]
    pub fn model(&self, id: &str) -> Option<&GeoModel> {
        self.models.get(id)
    }

    #[inline]
    pub fn geo_animation(&self, id: &str) -> Option<&GeoAnimationClip> {
        self.geo_animations.get(id)
    }

    #[inline]
    pub fn player_animation(&self, id: &str) -> Option<&PlayerAnimation> {
        self.player_animations.get(id)
    }

    /// Recursively loads all `*.geo.json` files under `dir`, returning count loaded.
    pub fn load_geo_dir(&mut self, dir: &Path) -> Result<usize, GeoError> {
        let mut files = Vec::new();
        collect_files_recursive(dir, &mut files)?;
        let mut loaded = 0;
        for path in files {
            let rel = relative_posix(dir, &path);
            let Some(stem) = rel.strip_suffix(".geo.json") else {
                continue;
            };
            let model = GeoModel::from_file(stem, &path)?;
            self.models.insert(stem.to_string(), model);
            loaded += 1;
        }
        self.generation = self.generation.wrapping_add(1);
        Ok(loaded)
    }

    /// Recursively loads all `*.animation.json` files under `dir`, returning `(files_loaded, clips_loaded)`.
    pub fn load_geo_animations_dir(&mut self, dir: &Path) -> Result<(usize, usize), GeoError> {
        let mut files = Vec::new();
        collect_files_recursive(dir, &mut files)?;
        let mut file_count = 0;
        let mut clip_count = 0;
        for path in files {
            let rel = relative_posix(dir, &path);
            let Some(stem) = rel.strip_suffix(".animation.json") else {
                continue;
            };
            let prefix = format!("{stem}/");
            let set = GeoAnimationSet::from_file(&prefix, &path)?;
            file_count += 1;
            for clip in set.clips {
                self.geo_animations.insert(clip.id.clone(), clip);
                clip_count += 1;
            }
        }
        Ok((file_count, clip_count))
    }

    /// Recursively loads all `*.json` player animations under `dir`, returning count loaded.
    pub fn load_player_animations_dir(&mut self, dir: &Path) -> Result<usize, GeoError> {
        let mut files = Vec::new();
        collect_files_recursive(dir, &mut files)?;
        let mut loaded = 0;
        for path in files {
            let rel = relative_posix(dir, &path);
            let Some(stem) = rel.strip_suffix(".json") else {
                continue;
            };
            let anim = PlayerAnimation::from_file(&path)?;
            self.player_animations.insert(stem.to_string(), anim);
            loaded += 1;
        }
        Ok(loaded)
    }
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), GeoError> {
    let entries = fs::read_dir(dir)
        .map_err(|e| GeoError::Io(format!("{}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| GeoError::Io(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn relative_posix(base: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(base).unwrap_or(file);
    rel.to_string_lossy().replace('\\', "/")
}

// ============================================================================
// Custom 213-Clip Player Animation System
// (1:1 port of `PlayerAnimation.java`, `AnimationController.java`, `ItemTurn.java`)
// ============================================================================

/// Humanoid body parts driven by Grand Line's player animation clips.
///
/// Discriminant order MUST match `PlayerAnimation.Part` (`0..=5`) because
/// `add_tremble` computes `seed = part.ordinal() as f32 * 1.7`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum PlayerPart {
    Head = 0,
    Body = 1,
    RightArm = 2,
    LeftArm = 3,
    RightLeg = 4,
    LeftLeg = 5,
}

impl PlayerPart {
    pub const ALL: [Self; 6] = [
        Self::Head,
        Self::Body,
        Self::RightArm,
        Self::LeftArm,
        Self::RightLeg,
        Self::LeftLeg,
    ];

    #[inline]
    pub fn ordinal(self) -> usize {
        self as usize
    }

    #[inline]
    pub fn is_limb(self) -> bool {
        matches!(
            self,
            Self::RightArm | Self::LeftArm | Self::RightLeg | Self::LeftLeg
        )
    }

    #[inline]
    pub fn is_leg(self) -> bool {
        matches!(self, Self::RightLeg | Self::LeftLeg)
    }

    pub fn parse(name: &str) -> Result<Self, GeoError> {
        match name.trim().to_ascii_uppercase().as_str() {
            "HEAD" => Ok(Self::Head),
            "BODY" => Ok(Self::Body),
            "RIGHT_ARM" => Ok(Self::RightArm),
            "LEFT_ARM" => Ok(Self::LeftArm),
            "RIGHT_LEG" => Ok(Self::RightLeg),
            "LEFT_LEG" => Ok(Self::LeftLeg),
            other => Err(GeoError::InvalidFormat(format!(
                "unknown player animation part '{other}'"
            ))),
        }
    }
}

/// Hand/arm selector matching Minecraft `HumanoidArm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandSide {
    Right,
    Left,
}

/// One keyframe in a [`PlayerChannel`] (`PlayerAnimation.Keyframe` 1:1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerKeyframe {
    /// Time in game ticks.
    pub t: f32,
    /// Rotation in radians (or aim weight in `rot.x` for aim channels).
    #[serde(with = "serde_vec3")]
    pub rot: Vec3,
    /// Position offset (or scale for `root_scale`).
    #[serde(with = "serde_vec3")]
    pub pos: Vec3,
    pub ease: Easing,
}

/// Sorted channel of [`PlayerKeyframe`]s (`PlayerAnimation.Channel` 1:1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerChannel {
    pub frames: Vec<PlayerKeyframe>,
}

impl PlayerChannel {
    /// Samples `(rot, pos)` at `time` ticks, matching `PlayerAnimation.Channel.sample` 1:1.
    pub fn sample(&self, time: f32) -> (Vec3, Vec3) {
        let f = &self.frames;
        if f.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        if time <= f[0].t || f.len() == 1 {
            return (f[0].rot, f[0].pos);
        }
        for i in 1..f.len() {
            if time < f[i].t {
                let a = f[i - 1];
                let b = f[i];
                let p = (time - a.t) / (b.t - a.t).max(1.0e-4);
                if b.ease == Easing::Catmullrom {
                    let k0 = f[i.saturating_sub(2)];
                    let k3 = f[(i + 1).min(f.len() - 1)];
                    return (
                        catmull_rom_vec3(k0.rot, a.rot, b.rot, k3.rot, p),
                        catmull_rom_vec3(k0.pos, a.pos, b.pos, k3.pos, p),
                    );
                }
                let e = b.ease.apply(p.clamp(0.0, 1.0));
                return (a.rot.lerp(b.rot, e), a.pos.lerp(b.pos, e));
            }
        }
        let last = f[f.len() - 1];
        (last.rot, last.pos)
    }
}

/// One parsed Grand Line custom player animation clip (`player_animations/*.json`).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerAnimation {
    pub length: f32,
    pub loop_anim: bool,
    pub loop_from: f32,
    pub fade_in: f32,
    pub fade_out: f32,
    pub both_arms_first_person: bool,
    pub parts: BTreeMap<PlayerPart, PlayerChannel>,
    /// Procedural high-frequency tremble amplitude per part in radians.
    pub tremble: BTreeMap<PlayerPart, f32>,
    /// Camera-pitch/yaw aim tracking weight curves (`w` stored in `rot.x`).
    pub aim: BTreeMap<PlayerPart, PlayerChannel>,
    pub root: Option<PlayerChannel>,
    /// Present only if at least one `root` keyframe specified `"scale"` (`pos` holds scale).
    pub root_scale: Option<PlayerChannel>,
    pub fp_right: Option<PlayerChannel>,
    pub fp_left: Option<PlayerChannel>,
    pub item_right: Option<PlayerChannel>,
    pub item_left: Option<PlayerChannel>,
    pub item_pivot_right: Vec3,
    pub item_pivot_left: Vec3,
    /// Named event beat timestamps in ticks (`"markers"`).
    pub markers: BTreeMap<String, f32>,
}

impl PlayerAnimation {
    /// Parses a player animation JSON string.
    pub fn from_json_str(src: &str) -> Result<Self, GeoError> {
        let val: Value =
            serde_json::from_str(src).map_err(|e| GeoError::Json(e.to_string()))?;
        Self::from_json_value(&val)
    }

    /// Parses a player animation JSON file from disk.
    pub fn from_file(path: &Path) -> Result<Self, GeoError> {
        let src = fs::read_to_string(path)
            .map_err(|e| GeoError::Io(format!("{}: {e}", path.display())))?;
        Self::from_json_str(&src)
    }

    /// Parses a player animation [`serde_json::Value`], matching `PlayerAnimation.parse` 1:1.
    pub fn from_json_value(val: &Value) -> Result<Self, GeoError> {
        let obj = val.as_object().ok_or_else(|| {
            GeoError::InvalidFormat("player animation root must be a JSON object".to_string())
        })?;

        let length = obj
            .get("length")
            .and_then(Value::as_f64)
            .ok_or_else(|| GeoError::InvalidFormat("player animation missing 'length'".to_string()))?
            as f32;
        let loop_anim = obj.get("loop").and_then(Value::as_bool).unwrap_or(false);
        let loop_from = obj
            .get("loop_from")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;
        let fade_in = obj.get("fade_in").and_then(Value::as_f64).unwrap_or(2.0) as f32;
        let fade_out = obj.get("fade_out").and_then(Value::as_f64).unwrap_or(3.0) as f32;
        let both_arms_first_person = obj
            .get("both_arms_first_person")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let mut parts = BTreeMap::new();
        if let Some(parts_obj) = obj.get("parts").and_then(Value::as_object) {
            for (k, v) in parts_obj {
                let part = PlayerPart::parse(k)?;
                let arr = v.as_array().ok_or_else(|| {
                    GeoError::InvalidFormat(format!("parts.{k} must be an array"))
                })?;
                parts.insert(part, parse_player_channel(arr, true)?);
            }
        }

        let mut tremble = BTreeMap::new();
        if let Some(tremble_obj) = obj.get("tremble").and_then(Value::as_object) {
            for (k, v) in tremble_obj {
                let part = PlayerPart::parse(k)?;
                let deg = v.as_f64().ok_or_else(|| {
                    GeoError::InvalidFormat(format!("tremble.{k} must be numeric"))
                })? as f32;
                tremble.insert(part, deg.to_radians());
            }
        }

        let mut aim = BTreeMap::new();
        if let Some(aim_obj) = obj.get("aim").and_then(Value::as_object) {
            for (k, v) in aim_obj {
                let part = PlayerPart::parse(k)?;
                let arr = v.as_array().ok_or_else(|| {
                    GeoError::InvalidFormat(format!("aim.{k} must be an array"))
                })?;
                aim.insert(part, parse_weight_channel(arr)?);
            }
        }

        let root_arr = obj.get("root").and_then(Value::as_array);
        let root = root_arr
            .map(|arr| parse_player_channel(arr, true))
            .transpose()?;
        let root_scale = root_arr
            .map(|arr| parse_scale_channel(arr))
            .transpose()?
            .flatten();

        let fp_right = obj
            .get("fp_right")
            .and_then(Value::as_array)
            .map(|a| parse_player_channel(a, true))
            .transpose()?;
        let fp_left = obj
            .get("fp_left")
            .and_then(Value::as_array)
            .map(|a| parse_player_channel(a, true))
            .transpose()?;
        let item_right = obj
            .get("item_right")
            .and_then(Value::as_array)
            .map(|a| parse_player_channel(a, true))
            .transpose()?;
        let item_left = obj
            .get("item_left")
            .and_then(Value::as_array)
            .map(|a| parse_player_channel(a, true))
            .transpose()?;

        let item_pivot_right = obj
            .get("item_pivot_right")
            .map(|v| json_vec3(Some(v), 0.0))
            .unwrap_or(DEFAULT_ITEM_PIVOT_RIGHT);
        let item_pivot_left = obj
            .get("item_pivot_left")
            .map(|v| json_vec3(Some(v), 0.0))
            .unwrap_or(DEFAULT_ITEM_PIVOT_LEFT);

        let mut markers = BTreeMap::new();
        if let Some(markers_obj) = obj.get("markers").and_then(Value::as_object) {
            for (k, v) in markers_obj {
                let tick = v.as_f64().ok_or_else(|| {
                    GeoError::InvalidFormat(format!("markers.{k} must be numeric"))
                })? as f32;
                markers.insert(k.clone(), tick);
            }
        }

        Ok(Self {
            length,
            loop_anim,
            loop_from,
            fade_in,
            fade_out,
            both_arms_first_person,
            parts,
            tremble,
            aim,
            root,
            root_scale,
            fp_right,
            fp_left,
            item_right,
            item_left,
            item_pivot_right,
            item_pivot_left,
            markers,
        })
    }

    /// Looks up a named marker tick.
    #[inline]
    pub fn marker(&self, name: &str) -> Option<f32> {
        self.markers.get(name).copied()
    }
}

fn parse_player_channel(arr: &[Value], degrees: bool) -> Result<PlayerChannel, GeoError> {
    let mut frames = Vec::with_capacity(arr.len().max(1));
    let mut last_rot = Vec3::ZERO;
    let mut last_pos = Vec3::ZERO;
    let rot_scale = if degrees {
        std::f32::consts::PI / 180.0
    } else {
        1.0
    };

    for element in arr {
        let k = element.as_object().ok_or_else(|| {
            GeoError::InvalidFormat("player channel keyframe must be an object".to_string())
        })?;
        let t = k
            .get("t")
            .and_then(Value::as_f64)
            .ok_or_else(|| GeoError::InvalidFormat("keyframe missing 't'".to_string()))?
            as f32;
        let rot = match k.get("rot") {
            Some(v) => json_vec3(Some(v), 0.0) * rot_scale,
            None => last_rot,
        };
        let pos = match k.get("pos") {
            Some(v) => json_vec3(Some(v), 0.0),
            None => last_pos,
        };
        let ease = k
            .get("ease")
            .and_then(Value::as_str)
            .map_or(Easing::Linear, Easing::parse);

        frames.push(PlayerKeyframe { t, rot, pos, ease });
        last_rot = rot;
        last_pos = pos;
    }

    if frames.is_empty() {
        frames.push(PlayerKeyframe {
            t: 0.0,
            rot: Vec3::ZERO,
            pos: Vec3::ZERO,
            ease: Easing::Linear,
        });
    }
    frames.sort_by(|a, b| a.t.total_cmp(&b.t));
    Ok(PlayerChannel { frames })
}

fn parse_weight_channel(arr: &[Value]) -> Result<PlayerChannel, GeoError> {
    let mut frames = Vec::with_capacity(arr.len().max(1));
    for element in arr {
        let k = element.as_object().ok_or_else(|| {
            GeoError::InvalidFormat("aim weight keyframe must be an object".to_string())
        })?;
        let t = k
            .get("t")
            .and_then(Value::as_f64)
            .ok_or_else(|| GeoError::InvalidFormat("aim keyframe missing 't'".to_string()))?
            as f32;
        let w = k.get("w").and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let ease = k
            .get("ease")
            .and_then(Value::as_str)
            .map_or(Easing::Linear, Easing::parse);
        frames.push(PlayerKeyframe {
            t,
            rot: Vec3::new(w, 0.0, 0.0),
            pos: Vec3::ZERO,
            ease,
        });
    }
    if frames.is_empty() {
        frames.push(PlayerKeyframe {
            t: 0.0,
            rot: Vec3::ZERO,
            pos: Vec3::ZERO,
            ease: Easing::Linear,
        });
    }
    frames.sort_by(|a, b| a.t.total_cmp(&b.t));
    Ok(PlayerChannel { frames })
}

fn parse_scale_channel(arr: &[Value]) -> Result<Option<PlayerChannel>, GeoError> {
    let mut frames = Vec::with_capacity(arr.len());
    let mut last = Vec3::ONE;
    let mut any = false;
    for element in arr {
        let k = element.as_object().ok_or_else(|| {
            GeoError::InvalidFormat("root keyframe must be an object".to_string())
        })?;
        let t = k
            .get("t")
            .and_then(Value::as_f64)
            .ok_or_else(|| GeoError::InvalidFormat("root keyframe missing 't'".to_string()))?
            as f32;
        if let Some(s) = k.get("scale") {
            last = json_vec3(Some(s), 1.0);
            any = true;
        }
        let ease = k
            .get("ease")
            .and_then(Value::as_str)
            .map_or(Easing::Linear, Easing::parse);
        frames.push(PlayerKeyframe {
            t,
            rot: Vec3::ZERO,
            pos: last,
            ease,
        });
    }
    if !any {
        return Ok(None);
    }
    frames.sort_by(|a, b| a.t.total_cmp(&b.t));
    Ok(Some(PlayerChannel { frames }))
}

/// Pose of one vanilla `ModelPart` (`xRot/yRot/zRot` in radians, `x/y/z` pivot in pixels).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartPose {
    #[serde(with = "serde_vec3")]
    pub rot: Vec3,
    #[serde(with = "serde_vec3")]
    pub pos: Vec3,
}

impl PartPose {
    pub const fn new(rot: Vec3, pos: Vec3) -> Self {
        Self { rot, pos }
    }
}

/// Complete posed state of a humanoid player model (`HumanoidModel` + root + first-person + item turns).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerPose {
    pub head: PartPose,
    pub hat: PartPose,
    pub body: PartPose,
    pub right_arm: PartPose,
    pub left_arm: PartPose,
    pub right_leg: PartPose,
    pub left_leg: PartPose,
    /// Whole-body root transform matrix (`AnimationController.applyRoot`).
    pub root_transform: Mat4,
    pub root_translation: Vec3,
    pub root_rotation: Vec3,
    pub root_scale: Vec3,
    pub fp_right: Option<Mat4>,
    pub fp_left: Option<Mat4>,
    pub item_right_turn: Option<(Quat, Vec3)>,
    pub item_left_turn: Option<(Quat, Vec3)>,
    pub both_arms_first_person: bool,
}

impl Default for PlayerPose {
    fn default() -> Self {
        Self::vanilla_rest(false)
    }
}

impl PlayerPose {
    /// Returns vanilla `HumanoidModel` initial poses (optionally for slim arms).
    pub fn vanilla_rest(slim_arms: bool) -> Self {
        let arm_y = if slim_arms { 2.5 } else { 2.0 };
        Self {
            head: PartPose::new(Vec3::ZERO, Vec3::ZERO),
            hat: PartPose::new(Vec3::ZERO, Vec3::ZERO),
            body: PartPose::new(Vec3::ZERO, Vec3::ZERO),
            right_arm: PartPose::new(Vec3::ZERO, Vec3::new(-5.0, arm_y, 0.0)),
            left_arm: PartPose::new(Vec3::ZERO, Vec3::new(5.0, arm_y, 0.0)),
            right_leg: PartPose::new(Vec3::ZERO, Vec3::new(-1.9, 12.0, 0.0)),
            left_leg: PartPose::new(Vec3::ZERO, Vec3::new(1.9, 12.0, 0.0)),
            root_transform: Mat4::IDENTITY,
            root_translation: Vec3::ZERO,
            root_rotation: Vec3::ZERO,
            root_scale: Vec3::ONE,
            fp_right: None,
            fp_left: None,
            item_right_turn: None,
            item_left_turn: None,
            both_arms_first_person: false,
        }
    }

    /// Restores vanilla `restoreUnposed` coordinates before applying animations (`AnimationController.restoreUnposed`).
    pub fn restore_unposed(&mut self) {
        self.right_leg.pos.x = -1.9;
        self.left_leg.pos.x = 1.9;
        self.head.pos.x = 0.0;
        self.body.pos.x = 0.0;
        self.head.pos.z = 0.0;
        self.body.pos.z = 0.0;
    }

    #[inline]
    pub fn part(&self, part: PlayerPart) -> PartPose {
        match part {
            PlayerPart::Head => self.head,
            PlayerPart::Body => self.body,
            PlayerPart::RightArm => self.right_arm,
            PlayerPart::LeftArm => self.left_arm,
            PlayerPart::RightLeg => self.right_leg,
            PlayerPart::LeftLeg => self.left_leg,
        }
    }

    #[inline]
    pub fn part_mut(&mut self, part: PlayerPart) -> &mut PartPose {
        match part {
            PlayerPart::Head => &mut self.head,
            PlayerPart::Body => &mut self.body,
            PlayerPart::RightArm => &mut self.right_arm,
            PlayerPart::LeftArm => &mut self.left_arm,
            PlayerPart::RightLeg => &mut self.right_leg,
            PlayerPart::LeftLeg => &mut self.left_leg,
        }
    }
}

/// Marker event emitted when a playing clip crosses one of its `"markers"` beats.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerEvent {
    pub entity_id: u32,
    pub clip_id: String,
    pub marker: String,
    pub tick: f32,
}

/// Additive pose layer entry (`AnimationController.PoseSink` + `lean`).
#[derive(Clone, Debug)]
pub struct AdditiveLayer {
    pub clip_id: String,
    pub time_ticks: f32,
    pub weight: f32,
}

/// Previous-clip snapshot retained during crossfade transitions.
#[derive(Clone, Debug)]
struct PreviousClip {
    id: String,
    time_ticks: f32,
    weight: f32,
}

/// Active playback state for one entity (`AnimationController.Active` 1:1).
#[derive(Clone, Debug)]
struct ActivePlayback {
    id: String,
    start_age: f32,
    stop_age: Option<f32>,
    last_marker_time: Option<f32>,
    previous: Option<PreviousClip>,
}

impl ActivePlayback {
    fn time(&self, clip: &PlayerAnimation, age: f32) -> f32 {
        let t = (age - self.start_age).max(0.0);
        if clip.loop_anim {
            let from = clip.loop_from;
            if from > 0.0 && from < clip.length {
                if t < clip.length {
                    t
                } else {
                    from + (t - clip.length) % (clip.length - from)
                }
            } else if clip.length <= 0.0 {
                0.0
            } else {
                t % clip.length
            }
        } else {
            t.min(clip.length)
        }
    }

    fn weight(&self, clip: &PlayerAnimation, age: f32) -> f32 {
        let t = age - self.start_age;
        let mut w = if clip.fade_in > 0.0 {
            (t / clip.fade_in).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if !clip.loop_anim && t > clip.length {
            w *= 1.0 - ((t - clip.length) / clip.fade_out.max(0.01)).clamp(0.0, 1.0);
        }
        if let Some(stop_age) = self.stop_age {
            w *= 1.0 - ((age - stop_age) / clip.fade_out.max(0.01)).clamp(0.0, 1.0);
        }
        w
    }

    fn fade_in_progress(&self, clip: &PlayerAnimation, age: f32) -> f32 {
        if clip.fade_in > 0.0 {
            ((age - self.start_age) / clip.fade_in).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    fn finished(&self, clip: &PlayerAnimation, age: f32) -> bool {
        let t = age - self.start_age;
        match self.stop_age {
            Some(stop_age) => age - stop_age > clip.fade_out,
            None => !clip.loop_anim && t > clip.length + clip.fade_out,
        }
    }
}

/// Snapshot of an entity's active animation state (`AnimationController.ClipState` 1:1).
#[derive(Clone, Debug, PartialEq)]
pub struct ClipState {
    pub id: String,
    pub time: f32,
    pub weight: f32,
    pub previous_id: Option<String>,
    pub previous_time: f32,
    pub previous_weight: f32,
}

/// Runtime player animation controller managing per-entity playback, crossfades,
/// additive pose layers, marker events, first-person arms, and item turns.
#[derive(Clone, Debug)]
pub struct PlayerAnimationController {
    clips: HashMap<String, PlayerAnimation>,
    active: HashMap<u32, ActivePlayback>,
    pub first_person_animations_enabled: bool,
    pub first_person_item_share: f32,
}

impl Default for PlayerAnimationController {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerAnimationController {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            active: HashMap::new(),
            first_person_animations_enabled: true,
            first_person_item_share: DEFAULT_FIRST_PERSON_ITEM_SHARE,
        }
    }

    /// Registers a clip by resource id (e.g. `"gomu_gomu/gear2_enter"`).
    pub fn register_clip(&mut self, id: impl Into<String>, clip: PlayerAnimation) {
        self.clips.insert(id.into(), clip);
    }

    /// Loads all `*.json` clips from a `player_animations` directory recursively.
    pub fn load_dir(&mut self, dir: &Path) -> Result<usize, GeoError> {
        let mut files = Vec::new();
        collect_files_recursive(dir, &mut files)?;
        let mut count = 0;
        for path in files {
            let rel = relative_posix(dir, &path);
            let Some(stem) = rel.strip_suffix(".json") else {
                continue;
            };
            let clip = PlayerAnimation::from_file(&path)?;
            self.clips.insert(stem.to_string(), clip);
            count += 1;
        }
        Ok(count)
    }

    #[inline]
    pub fn clip(&self, id: &str) -> Option<&PlayerAnimation> {
        self.clips.get(id)
    }

    #[inline]
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    /// Starts playing `clip_id` on `entity_id` at tick `entity_tick`, capturing any currently
    /// playing clip as the crossfade source (`AnimationController.play` 1:1).
    pub fn play(&mut self, entity_id: u32, clip_id: &str, entity_tick: f32) -> bool {
        if !self.clips.contains_key(clip_id) {
            return false;
        }
        let previous = self.active.get(&entity_id).and_then(|old| {
            let old_clip = self.clips.get(&old.id)?;
            Some(PreviousClip {
                id: old.id.clone(),
                time_ticks: old.time(old_clip, entity_tick),
                weight: old.weight(old_clip, entity_tick),
            })
        });

        self.active.insert(
            entity_id,
            ActivePlayback {
                id: clip_id.to_string(),
                start_age: entity_tick,
                stop_age: None,
                last_marker_time: None,
                previous,
            },
        );
        true
    }

    /// Begins fading out an entity's current animation (`AnimationController.stop` 1:1).
    pub fn stop(&mut self, entity_id: u32, entity_tick: f32) {
        if let Some(a) = self.active.get_mut(&entity_id)
            && a.stop_age.is_none()
        {
            a.stop_age = Some(entity_tick);
        }
    }

    #[inline]
    pub fn is_animating(&self, entity_id: u32) -> bool {
        self.active.contains_key(&entity_id)
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }

    /// Queries the current playback state for `entity_id` at `age_ticks` (`AnimationController.current`).
    pub fn current(&self, entity_id: u32, age_ticks: f32) -> Option<ClipState> {
        let active = self.active.get(&entity_id)?;
        let clip = self.clips.get(&active.id)?;
        let time = active.time(clip, age_ticks);
        let weight = active.weight(clip, age_ticks);
        let (previous_id, previous_time, previous_weight) = match &active.previous {
            Some(prev) => (
                Some(prev.id.clone()),
                prev.time_ticks,
                prev.weight * (1.0 - active.fade_in_progress(clip, age_ticks)),
            ),
            None => (None, 0.0, 0.0),
        };
        Some(ClipState {
            id: active.id.clone(),
            time,
            weight,
            previous_id,
            previous_time,
            previous_weight,
        })
    }

    /// Advances an entity to `age_ticks`, emits all crossed [`MarkerEvent`]s in chronological order,
    /// and removes the playback entry once its fade-out finishes.
    pub fn advance_entity(&mut self, entity_id: u32, age_ticks: f32) -> Vec<MarkerEvent> {
        let mut events = Vec::new();
        let mut finished = false;

        if let Some(active) = self.active.get_mut(&entity_id) {
            if let Some(clip) = self.clips.get(&active.id) {
                let cur_time = active.time(clip, age_ticks);
                collect_marker_events(
                    entity_id,
                    &active.id,
                    clip,
                    active.last_marker_time,
                    cur_time,
                    &mut events,
                );
                active.last_marker_time = Some(cur_time);
                finished = active.finished(clip, age_ticks);
            } else {
                finished = true;
            }
        }

        if finished {
            self.active.remove(&entity_id);
        }
        events.sort_by(|a, b| a.tick.total_cmp(&b.tick));
        events
    }

    /// Ticks all active entities using a tick lookup closure (`None` = entity despawned),
    /// pruning finished entries and returning all emitted [`MarkerEvent`]s.
    pub fn tick_all<F>(&mut self, mut entity_tick: F) -> Vec<MarkerEvent>
    where
        F: FnMut(u32) -> Option<f32>,
    {
        let ids: Vec<u32> = self.active.keys().copied().collect();
        let mut all_events = Vec::new();
        for id in ids {
            match entity_tick(id) {
                Some(age) => all_events.extend(self.advance_entity(id, age)),
                None => {
                    self.active.remove(&id);
                }
            }
        }
        all_events
    }

    /// Evaluates additive layers + active/previous animation onto `pose` (`AnimationController.applyToModel` 1:1).
    ///
    /// Expects `pose` to already contain the player's vanilla `setupAnim` state (including
    /// `pose.head.rot.x` = look pitch and `pose.head.rot.y` = look yaw for `aim` channels!).
    pub fn evaluate_pose(
        &self,
        entity_id: u32,
        age_ticks: f32,
        body_yaw_deg: f32,
        additive_layers: &[AdditiveLayer],
        lean_pitch_roll: Vec2,
        pose: &mut PlayerPose,
    ) {
        pose.restore_unposed();

        let mut layered = false;
        let mut layer_root_rot = Vec3::new(lean_pitch_roll.x, 0.0, lean_pitch_roll.y);
        let mut layer_root_pos = Vec3::ZERO;

        for layer in additive_layers {
            if layer.weight <= 0.001 {
                continue;
            }
            if let Some(clip) = self.clips.get(&layer.clip_id) {
                let t = resolve_layer_time(clip, layer.time_ticks);
                let w = layer.weight.min(1.0);
                apply_additive(pose, clip, t, w, age_ticks);
                if let Some(root_ch) = &clip.root {
                    let (r, p) = root_ch.sample(t);
                    layer_root_rot += r * w;
                    layer_root_pos += p * w;
                }
                layered = true;
            }
        }

        let active = self.active.get(&entity_id);
        let cur_clip = active.and_then(|a| self.clips.get(&a.id));

        if let (Some(active), Some(clip)) = (active, cur_clip) {
            if let Some(prev) = &active.previous
                && let Some(prev_clip) = self.clips.get(&prev.id)
            {
                let pw = prev.weight * (1.0 - active.fade_in_progress(clip, age_ticks));
                if pw > 0.001 {
                    apply_animation(pose, prev_clip, prev.time_ticks, pw, age_ticks);
                }
            }

            let w = active.weight(clip, age_ticks);
            let t = active.time(clip, age_ticks);
            if w > 0.001 {
                apply_animation(pose, clip, t, w, age_ticks);
            }
            pose.hat = pose.head;

            // Root transform (`AnimationController.applyRoot` 1:1).
            let (root_mat, root_tr, root_rot, root_sc) = compute_root_transform(
                body_yaw_deg,
                layer_root_rot,
                layer_root_pos,
                Some((clip, t, w)),
            );
            pose.root_transform = root_mat;
            pose.root_translation = root_tr;
            pose.root_rotation = root_rot;
            pose.root_scale = root_sc;

            // First-person transforms (`AnimationController.applyFirstPerson` 1:1).
            pose.fp_right = self.first_person_transform(entity_id, HandSide::Right, age_ticks);
            pose.fp_left = self.first_person_transform(entity_id, HandSide::Left, age_ticks);
            pose.both_arms_first_person = self.wants_both_arms_first_person(entity_id, age_ticks);

            // Third-person item turns (`AnimationController.itemTurn` 1:1).
            pose.item_right_turn = self.item_turn(entity_id, HandSide::Right, age_ticks);
            pose.item_left_turn = self.item_turn(entity_id, HandSide::Left, age_ticks);
        } else {
            if layered {
                pose.hat = pose.head;
            }
            let (root_mat, root_tr, root_rot, root_sc) =
                compute_root_transform(body_yaw_deg, layer_root_rot, layer_root_pos, None);
            pose.root_transform = root_mat;
            pose.root_translation = root_tr;
            pose.root_rotation = root_rot;
            pose.root_scale = root_sc;
            pose.fp_right = None;
            pose.fp_left = None;
            pose.item_right_turn = None;
            pose.item_left_turn = None;
            pose.both_arms_first_person = false;
        }
    }

    /// Computes first-person arm transform matrix (`AnimationController.applyFirstPerson` 1:1).
    pub fn first_person_transform(
        &self,
        entity_id: u32,
        arm: HandSide,
        age_ticks: f32,
    ) -> Option<Mat4> {
        if !self.first_person_animations_enabled {
            return None;
        }
        let active = self.active.get(&entity_id)?;
        let clip = self.clips.get(&active.id)?;
        let channel = match arm {
            HandSide::Right => clip.fp_right.as_ref()?,
            HandSide::Left => clip.fp_left.as_ref()?,
        };
        let w = active.weight(clip, age_ticks);
        let (rot, pos) = channel.sample(active.time(clip, age_ticks));
        let side = match arm {
            HandSide::Right => 1.0_f32,
            HandSide::Left => -1.0_f32,
        };
        let m = Mat4::from_translation(Vec3::new(side * pos.x * w, pos.y * w, -pos.z * w))
            * Mat4::from_rotation_x(rot.x * w)
            * Mat4::from_rotation_y(side * rot.y * w)
            * Mat4::from_rotation_z(side * rot.z * w);
        Some(m)
    }

    /// Returns whether both arms should be rendered in first person (`AnimationController.wantsBothArmsFirstPerson`).
    pub fn wants_both_arms_first_person(&self, entity_id: u32, age_ticks: f32) -> bool {
        if !self.first_person_animations_enabled {
            return false;
        }
        let Some(active) = self.active.get(&entity_id) else {
            return false;
        };
        let Some(clip) = self.clips.get(&active.id) else {
            return false;
        };
        clip.both_arms_first_person && active.weight(clip, age_ticks) > 0.05
    }

    /// Evaluates the third-person held-item rotation and pivot in pixels (`AnimationController.itemTurn` 1:1).
    pub fn item_turn(&self, entity_id: u32, arm: HandSide, age_ticks: f32) -> Option<(Quat, Vec3)> {
        let active = self.active.get(&entity_id)?;
        let cur_clip = self.clips.get(&active.id)?;
        let right = arm == HandSide::Right;
        let mut out = Quat::IDENTITY;
        let mut any = false;
        let mut pivot = if right {
            DEFAULT_ITEM_PIVOT_RIGHT
        } else {
            DEFAULT_ITEM_PIVOT_LEFT
        };

        if let Some(prev) = &active.previous
            && let Some(prev_clip) = self.clips.get(&prev.id)
        {
            let ch = if right {
                prev_clip.item_right.as_ref()
            } else {
                prev_clip.item_left.as_ref()
            };
            let pw = prev.weight * (1.0 - active.fade_in_progress(cur_clip, age_ticks));
            if let Some(ch) = ch
                && pw > 0.001
            {
                let (rot, _) = ch.sample(prev.time_ticks);
                let q_prev = quat_zyx(rot.z, rot.y, rot.x);
                out = out.slerp(q_prev, pw.min(1.0));
                pivot = if right {
                    prev_clip.item_pivot_right
                } else {
                    prev_clip.item_pivot_left
                };
                any = true;
            }
        }

        let w = active.weight(cur_clip, age_ticks);
        if w > 0.001 {
            let ch = if right {
                cur_clip.item_right.as_ref()
            } else {
                cur_clip.item_left.as_ref()
            };
            let q_cur = if let Some(ch) = ch {
                let (rot, _) = ch.sample(active.time(cur_clip, age_ticks));
                pivot = if right {
                    cur_clip.item_pivot_right
                } else {
                    cur_clip.item_pivot_left
                };
                any = true;
                quat_zyx(rot.z, rot.y, rot.x)
            } else {
                Quat::IDENTITY
            };
            out = out.slerp(q_cur, w.min(1.0));
        }

        if any && out.w.abs() < 0.99999 {
            Some((out, pivot))
        } else {
            None
        }
    }

    /// Evaluates first-person held-item orientation (`AnimationController.firstPersonItem` 1:1).
    pub fn first_person_item(
        &self,
        entity_id: u32,
        arm: HandSide,
        age_ticks: f32,
        view_pitch_rad: f32,
        rest: Quat,
    ) -> Option<Quat> {
        let active = self.active.get(&entity_id)?;
        let cur_clip = self.clips.get(&active.id)?;
        let right = arm == HandSide::Right;
        let share = self.first_person_item_share;
        let mut out = rest;
        let mut any = false;

        if let Some(prev) = &active.previous
            && let Some(prev_clip) = self.clips.get(&prev.id)
        {
            let pw = prev.weight * (1.0 - active.fade_in_progress(cur_clip, age_ticks));
            if pw > 0.001
                && let Some(q_prev) =
                    item_view(prev_clip, prev.time_ticks, right, view_pitch_rad, rest, share)
            {
                out = out.slerp(q_prev, pw.min(1.0));
                any = true;
            }
        }

        let w = active.weight(cur_clip, age_ticks);
        if w > 0.001 {
            let q_cur = match item_view(
                cur_clip,
                active.time(cur_clip, age_ticks),
                right,
                view_pitch_rad,
                rest,
                share,
            ) {
                Some(q) => {
                    any = true;
                    q
                }
                None => rest,
            };
            out = out.slerp(q_cur, w.min(1.0));
        }

        any.then_some(out)
    }
}

/// Builds the third-person item pivot rotation matrix (`ItemTurn.thirdPerson` 1:1).
#[inline]
pub fn item_turn_matrix(turn: Quat, pivot_px: Vec3) -> Mat4 {
    let p = pivot_px / 16.0;
    Mat4::from_translation(p) * Mat4::from_quat(turn) * Mat4::from_translation(-p)
}

#[inline]
fn quat_zyx(rz: f32, ry: f32, rx: f32) -> Quat {
    Quat::from_rotation_z(rz) * Quat::from_rotation_y(ry) * Quat::from_rotation_x(rx)
}

fn item_view(
    anim: &PlayerAnimation,
    t: f32,
    right: bool,
    pitch: f32,
    rest: Quat,
    share: f32,
) -> Option<Quat> {
    let item = if right {
        anim.item_right.as_ref()?
    } else {
        anim.item_left.as_ref()?
    };
    let arm_part = if right {
        PlayerPart::RightArm
    } else {
        PlayerPart::LeftArm
    };
    let arm_ch = anim.parts.get(&arm_part)?;

    let mut body_a = Quat::from_rotation_x(pitch);
    if let Some(root_ch) = &anim.root {
        let (rot, _) = root_ch.sample(t);
        body_a = body_a
            * Quat::from_rotation_y(rot.y)
            * Quat::from_rotation_x(-rot.x)
            * Quat::from_rotation_z(rot.z);
    }

    let body_yaw = anim
        .parts
        .get(&PlayerPart::Body)
        .map_or(0.0, |b| b.sample(t).0.y);

    let flip = Quat::from_rotation_z(PI);
    body_a *= flip;
    let (arm_rot, _) = arm_ch.sample(t);
    body_a *= quat_zyx(arm_rot.z, arm_rot.y + body_yaw, arm_rot.x);

    let (item_rot, _) = item.sample(t);
    let delta = body_a * quat_zyx(item_rot.z, item_rot.y, item_rot.x) * body_a.conjugate();
    let shared = Quat::IDENTITY.slerp(delta, share);
    Some(shared * rest)
}

fn collect_marker_events(
    entity_id: u32,
    clip_id: &str,
    clip: &PlayerAnimation,
    prev_time: Option<f32>,
    cur_time: f32,
    out: &mut Vec<MarkerEvent>,
) {
    if clip.markers.is_empty() {
        return;
    }
    for (marker, &tick) in &clip.markers {
        let fired = match prev_time {
            None => tick <= cur_time,
            Some(prev) if cur_time >= prev => tick > prev && tick <= cur_time,
            Some(prev) => {
                // Loop wrapped from `prev -> clip.length`, then `loop_from -> cur_time`.
                (tick > prev && tick <= clip.length)
                    || (tick >= clip.loop_from && tick <= cur_time)
            }
        };
        if fired {
            out.push(MarkerEvent {
                entity_id,
                clip_id: clip_id.to_string(),
                marker: marker.clone(),
                tick,
            });
        }
    }
}

fn resolve_layer_time(clip: &PlayerAnimation, time: f32) -> f32 {
    let len = clip.length;
    if clip.loop_anim && len > 0.0 && time > len {
        let from = clip.loop_from.clamp(0.0, (len - 0.001).max(0.0));
        from + (time - from) % (len - from).max(0.001)
    } else {
        time
    }
}

fn add_tremble(anim: &PlayerAnimation, part: PlayerPart, age: f32, rot: &mut Vec3) {
    if let Some(&amp) = anim.tremble.get(&part) {
        let seed = (part.ordinal() as f32) * 1.7;
        rot.x += amp * ((age * 4.1 + seed).sin() * 0.6 + (age * 9.7 + seed * 2.0).sin() * 0.4);
        rot.y += amp * 0.5 * (age * 6.3 + seed * 3.0).sin();
        rot.z += amp * ((age * 7.9 + seed).sin() * 0.5 + (age * 13.1).sin() * 0.3);
    }
}

fn apply_additive(pose: &mut PlayerPose, anim: &PlayerAnimation, t: f32, w: f32, age: f32) {
    for (&p, ch) in &anim.parts {
        let (mut rot, pos) = ch.sample(t);
        add_tremble(anim, p, age, &mut rot);
        let part = pose.part_mut(p);
        part.rot.x += rot.x * w;
        part.rot.y += rot.y * w;
        if p.is_limb() {
            part.rot.z += rot.z * w;
        }
        part.pos.y += pos.y * w;
        if p.is_leg() {
            part.pos.z += pos.z * w;
        }
    }
}

fn apply_animation(pose: &mut PlayerPose, anim: &PlayerAnimation, t: f32, w: f32, age: f32) {
    let body_yaw_before = pose.body.rot.y;
    let look_pitch = pose.head.rot.x;
    let look_yaw = pose.head.rot.y;

    if let Some(body_ch) = anim.parts.get(&PlayerPart::Body) {
        let (mut rot, pos) = body_ch.sample(t);
        add_tremble(anim, PlayerPart::Body, age, &mut rot);
        blend_part(&mut pose.body, rot, pos, w);
        let by = pose.body.rot.y;
        pose.right_arm.pos.z = by.sin() * 5.0;
        pose.right_arm.pos.x = -by.cos() * 5.0;
        pose.left_arm.pos.z = -by.sin() * 5.0;
        pose.left_arm.pos.x = by.cos() * 5.0;
    }

    let body_yaw_delta = pose.body.rot.y - body_yaw_before;

    for (&part, ch) in &anim.parts {
        if part == PlayerPart::Body {
            continue;
        }
        let (mut rot, pos) = ch.sample(t);
        add_tremble(anim, part, age, &mut rot);
        if matches!(part, PlayerPart::RightArm | PlayerPart::LeftArm) {
            rot.y += pose.body.rot.y;
        }
        if let Some(aim_ch) = anim.aim.get(&part) {
            let (aim_val, _) = aim_ch.sample(t);
            let a = aim_val.x;
            rot.x += a * look_pitch;
            rot.y += a * (look_yaw - pose.body.rot.y);
        }
        blend_part(pose.part_mut(part), rot, pos, w);
    }

    if body_yaw_delta != 0.0 {
        if !anim.parts.contains_key(&PlayerPart::RightArm) {
            pose.right_arm.rot.y += body_yaw_delta;
        }
        if !anim.parts.contains_key(&PlayerPart::LeftArm) {
            pose.left_arm.rot.y += body_yaw_delta;
        }
    }
}

#[inline]
fn blend_part(part: &mut PartPose, rot: Vec3, pos: Vec3, w: f32) {
    part.rot = part.rot.lerp(rot, w);
    part.pos += pos * w;
}

fn compute_root_transform(
    body_yaw_deg: f32,
    layer_root_rot: Vec3,
    layer_root_pos: Vec3,
    clip_sample: Option<(&PlayerAnimation, f32, f32)>,
) -> (Mat4, Vec3, Vec3, Vec3) {
    let has_layer_root = layer_root_rot.abs().max_element() > 1.0e-5
        || layer_root_pos.abs().max_element() > 1.0e-5;
    let has_clip_root = clip_sample.is_some_and(|(c, _, _)| c.root.is_some());

    if !has_layer_root && !has_clip_root {
        return (Mat4::IDENTITY, Vec3::ZERO, Vec3::ZERO, Vec3::ONE);
    }

    let yaw_rad = (180.0 - body_yaw_deg).to_radians();
    let mut m = Mat4::from_rotation_y(yaw_rad);
    let mut total_trans = Vec3::ZERO;
    let mut total_rot = Vec3::ZERO;
    let mut total_scale = Vec3::ONE;

    if has_layer_root {
        let lt = Vec3::new(layer_root_pos.x, layer_root_pos.y, -layer_root_pos.z);
        total_trans += lt;
        total_rot += layer_root_rot;
        m *= Mat4::from_translation(lt);
        m *= Mat4::from_translation(Vec3::new(0.0, 0.9, 0.0));
        m *= Mat4::from_rotation_y(layer_root_rot.y);
        m *= Mat4::from_rotation_x(-layer_root_rot.x);
        m *= Mat4::from_rotation_z(layer_root_rot.z);
        m *= Mat4::from_translation(Vec3::new(0.0, -0.9, 0.0));
    }

    if let Some((clip, t, w)) = clip_sample
        && let Some(root_ch) = &clip.root
    {
        let (rot, pos) = root_ch.sample(t);
        let ct = Vec3::new(pos.x * w, pos.y * w, -pos.z * w);
        total_trans += ct;
        total_rot += rot * w;
        m *= Mat4::from_translation(ct);
        if let Some(scale_ch) = &clip.root_scale {
            let (_, s) = scale_ch.sample(t);
            let sc = Vec3::ONE.lerp(s, w);
            total_scale *= sc;
            m *= Mat4::from_scale(sc);
        }
        m *= Mat4::from_translation(Vec3::new(0.0, 0.9, 0.0));
        m *= Mat4::from_rotation_y(rot.y * w);
        m *= Mat4::from_rotation_x(-rot.x * w);
        m *= Mat4::from_rotation_z(rot.z * w);
        m *= Mat4::from_translation(Vec3::new(0.0, -0.9, 0.0));
    }

    m *= Mat4::from_rotation_y(-yaw_rad);
    (m, total_trans, total_rot, total_scale)
}

// ============================================================================
// Serde Helpers for glam::Vec3 (without requiring glam's `serde` feature)
// ============================================================================

mod serde_vec3 {
    use glam::Vec3;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec3, s: S) -> Result<S::Ok, S::Error> {
        v.to_array().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec3, D::Error> {
        let a = <[f32; 3]>::deserialize(d)?;
        Ok(Vec3::from_array(a))
    }
}

mod serde_opt_vec3 {
    use glam::Vec3;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<Vec3>, s: S) -> Result<S::Ok, S::Error> {
        v.map(|x| x.to_array()).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec3>, D::Error> {
        let a = Option::<[f32; 3]>::deserialize(d)?;
        Ok(a.map(Vec3::from_array))
    }
}

// ============================================================================
// JSON Helpers
// ============================================================================

#[inline]
fn json_f32(val: Option<&Value>, def: f32) -> f32 {
    match val {
        Some(Value::Number(n)) => n.as_f64().map_or(def, |f| f as f32),
        Some(Value::String(s)) => s.trim().parse::<f32>().unwrap_or(def),
        _ => def,
    }
}

#[inline]
fn json_vec3(val: Option<&Value>, def: f32) -> Vec3 {
    let Some(arr) = val.and_then(Value::as_array) else {
        return Vec3::splat(def);
    };
    Vec3::new(
        json_f32(arr.first(), def),
        json_f32(arr.get(1), def),
        json_f32(arr.get(2), def),
    )
}

// ============================================================================
// Unit & Full Corpus Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_GRANDLINE_ASSETS: &str =
        r"C:\Users\jak\Downloads\grandline-1.21.1-0.4.0\assets\devilfruits";

    fn grandline_assets_dir() -> Option<std::path::PathBuf> {
        if let Ok(p) = std::env::var("GRANDLINE_ASSETS_DIR") {
            let pb = std::path::PathBuf::from(p);
            if pb.is_dir() {
                return Some(pb);
            }
        }
        let pb = std::path::PathBuf::from(DEFAULT_GRANDLINE_ASSETS);
        pb.is_dir().then_some(pb)
    }

    fn assert_vec3_finite(v: Vec3, ctx: &str) {
        assert!(
            v.x.is_finite() && v.y.is_finite() && v.z.is_finite(),
            "non-finite Vec3 ({v:?}) in {ctx}"
        );
    }

    fn assert_vertex_finite(v: &GeoVertex, ctx: &str) {
        assert!(
            v.pos.iter().all(|x| x.is_finite())
                && v.uv.iter().all(|x| x.is_finite())
                && v.normal.iter().all(|x| x.is_finite()),
            "non-finite GeoVertex ({v:?}) in {ctx}"
        );
    }

    fn assert_mat4_finite(m: Mat4, ctx: &str) {
        assert!(
            m.to_cols_array().iter().all(|x| x.is_finite()),
            "non-finite Mat4 in {ctx}"
        );
    }

    #[test]
    fn easing_curves_endpoints_and_gecko_aliases() {
        let all = [
            Easing::Linear,
            Easing::Catmullrom,
            Easing::Step,
            Easing::InSine,
            Easing::OutSine,
            Easing::InOutSine,
            Easing::InQuad,
            Easing::OutQuad,
            Easing::InOutQuad,
            Easing::InCubic,
            Easing::OutCubic,
            Easing::InOutCubic,
            Easing::InExpo,
            Easing::OutExpo,
            Easing::InBack,
            Easing::OutBack,
            Easing::InOutBack,
            Easing::OutElastic,
            Easing::OutBounce,
        ];
        for e in all {
            let v0 = e.apply(0.0);
            let v_mid = e.apply(0.5);
            let v1 = e.apply(1.0);
            assert!(v0.is_finite() && v_mid.is_finite() && v1.is_finite());
            assert!(
                (v0 - 0.0).abs() < 1.0e-3,
                "{e:?} at t=0 was {v0}, expected ~0"
            );
            assert!(
                (v1 - 1.0).abs() < 1.0e-3,
                "{e:?} at t=1 was {v1}, expected ~1"
            );
        }
        assert_eq!(Easing::Step.apply(0.99), 0.0);
        assert_eq!(Easing::Step.apply(1.0), 1.0);

        // All 9 GeckoLib camelCase easings in Grand Line geo_animations:
        assert_eq!(Easing::parse("easeInOutSine"), Easing::InOutSine);
        assert_eq!(Easing::parse("easeOutQuad"), Easing::OutQuad);
        assert_eq!(Easing::parse("easeOutExpo"), Easing::OutExpo);
        assert_eq!(Easing::parse("easeInOutQuad"), Easing::InOutQuad);
        assert_eq!(Easing::parse("easeOutBack"), Easing::OutBack);
        assert_eq!(Easing::parse("easeInQuad"), Easing::InQuad);
        assert_eq!(Easing::parse("easeOutSine"), Easing::OutSine);
        assert_eq!(Easing::parse("easeInCubic"), Easing::InCubic);
        assert_eq!(Easing::parse("easeInExpo"), Easing::InExpo);

        // All 10 snake_case easings in Grand Line player_animations:
        assert_eq!(Easing::parse("out_quad"), Easing::OutQuad);
        assert_eq!(Easing::parse("in_quad"), Easing::InQuad);
        assert_eq!(Easing::parse("out_expo"), Easing::OutExpo);
        assert_eq!(Easing::parse("in_out_sine"), Easing::InOutSine);
        assert_eq!(Easing::parse("out_back"), Easing::OutBack);
        assert_eq!(Easing::parse("in_out_quad"), Easing::InOutQuad);
        assert_eq!(Easing::parse("in_expo"), Easing::InExpo);
        assert_eq!(Easing::parse("catmullrom"), Easing::Catmullrom);
        assert_eq!(Easing::parse("out_sine"), Easing::OutSine);
        assert_eq!(Easing::parse("step"), Easing::Step);
    }

    #[test]
    fn bedrock_geo_box_uv_mirror_per_face_negative_uv_and_cycle() {
        let src = r#"{
          "format_version": "1.12.0",
          "minecraft:geometry": [{
            "description": { "identifier": "geometry.test", "texture_width": 64, "texture_height": 64 },
            "bones": [
              {
                "name": "child",
                "parent": "root",
                "pivot": [2, 10, 0],
                "rotation": [0, 90, 0],
                "cubes": [
                  {
                    "origin": [-2, 6, -2],
                    "size": [4, 8, 4],
                    "inflate": 0.25,
                    "mirror": true,
                    "pivot": [0, 10, 0],
                    "rotation": [15, 0, -10],
                    "uv": [0, 16]
                  }
                ]
              },
              {
                "name": "root",
                "pivot": [0, 0, 0],
                "cubes": [
                  {
                    "origin": [-4, 0, -4],
                    "size": [8, 8, 8],
                    "uv": {
                      "north": { "uv": [16, 16], "uv_size": [-8, 8] },
                      "up": { "uv": [8, 0], "uv_size": [8, -8] }
                    }
                  }
                ]
              }
            ]
          }]
        }"#;

        let model = GeoModel::from_json_str("test", src).unwrap();
        // Topological sort puts root (0) before child (1).
        assert_eq!(model.bone_index("root"), Some(0));
        assert_eq!(model.bone_index("child"), Some(1));
        assert_eq!(model.bones[1].parent, Some(0));
        // Root has only 2 faces specified -> 8 quad vertices.
        assert_eq!(model.bones[0].vertex_count(), 8);
        // Verify negative uv_size preserved: north uv [16, 16] + size [-8, 8] -> u0=16/64, u1=8/64.
        assert!((model.bones[0].baked_vertices[0].uv[0] - 0.25).abs() < 1.0e-5);
        assert!((model.bones[0].baked_vertices[2].uv[0] - 0.125).abs() < 1.0e-5);
        // Child has 6 faces -> 24 quad vertices.
        assert_eq!(model.bones[1].vertex_count(), 24);

        let mut pose = GeoPose::new(&model);
        pose.set_bone_pose(
            0,
            BonePose {
                rotation_deg: Vec3::new(0.0, 45.0, 0.0),
                position: Vec3::new(4.0, 2.0, -1.0),
                scale: Vec3::splat(1.5),
            },
        );
        let verts = model.evaluate_world_vertices(Mat4::IDENTITY, &pose);
        assert_eq!(verts.len(), 32);
        for v in &verts {
            assert_vertex_finite(v, "test model");
        }
        // Hide root -> child must also be hidden.
        pose.hide(0, true);
        assert!(model.evaluate_world_vertices(Mat4::IDENTITY, &pose).is_empty());

        // Cycle detection.
        let cycle_src = r#"{
          "format_version": "1.12.0",
          "minecraft:geometry": [{
            "description": { "identifier": "geometry.cycle", "texture_width": 16, "texture_height": 16 },
            "bones": [
              { "name": "a", "parent": "b" },
              { "name": "b", "parent": "a" }
            ]
          }]
        }"#;
        assert!(matches!(
            GeoModel::from_json_str("cycle", cycle_src),
            Err(GeoError::BoneCycle { .. })
        ));
    }

    #[test]
    fn bedrock_animation_formats_pre_post_catmull_hold_and_crossfade() {
        let src = r#"{
          "format_version": "1.8.0",
          "animations": {
            "animation.test.a": {
              "loop": "hold_on_last_frame",
              "animation_length": 1.0,
              "devilfruits:fade_in": 4,
              "devilfruits:fade_out": 5,
              "devilfruits:markers": { "hit": 0.5 },
              "bones": {
                "arm": {
                  "rotation": [10, 20, 30],
                  "position": {
                    "0.0": { "post": [0, 0, 0], "lerp_mode": "catmullrom" },
                    "0.5": { "pre": [0, 5, 0], "post": [0, 10, 0], "easing": "easeOutBack" },
                    "1.0": { "vector": [0, 20, 0], "easing": "easeInOutSine" }
                  },
                  "scale": 2.0
                }
              }
            },
            "animation.test.b": {
              "loop": true,
              "animation_length": 2.0,
              "bones": {
                "arm": {
                  "rotation": {
                    "0.0": [0, 0, 0],
                    "2.0": { "vector": [0, 180, 0], "easing": "easeInQuad" }
                  }
                }
              }
            }
          }
        }"#;

        let set = GeoAnimationSet::from_json_str("test/", src).unwrap();
        assert_eq!(set.len(), 2);
        let a = set.get_by_id("test/a").unwrap();
        let b = set.get_by_id("test/b").unwrap();
        assert_eq!(a.loop_mode, LoopMode::HoldOnLastFrame);
        assert_eq!(b.loop_mode, LoopMode::Loop);
        assert_eq!(a.marker("hit", -1.0), 0.5);
        assert_eq!(a.fade_in, 4.0);
        assert_eq!(a.fade_out, 5.0);

        let p0 = a.sample_bone("arm", 0.0);
        assert_eq!(p0.rotation_deg, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(p0.scale, Vec3::splat(2.0));
        // Hold on last frame clamps time > length to last frame.
        let p_hold = a.sample_bone("arm", 5.0);
        assert!((p_hold.position.y - 20.0).abs() < 1.0e-4);
        // Loop wraps cleanly.
        let pb_mid = b.sample_bone("arm", 3.0); // 3.0 % 2.0 = 1.0 -> InQuad(0.5)=0.25 -> 45 deg
        assert!((pb_mid.rotation_deg.y - 45.0).abs() < 1.0e-3);

        // Crossfade between a and b.
        let xf = GeoAnimationClip::crossfade_bone_poses(a, 0.0, b, 1.0, 0.5);
        let arm_xf = xf["arm"];
        assert!((arm_xf.rotation_deg.x - 5.0).abs() < 1.0e-3);
        assert!((arm_xf.rotation_deg.y - 32.5).abs() < 1.0e-3);
        assert!((arm_xf.scale.x - 1.5).abs() < 1.0e-3);
    }

    #[test]
    fn player_controller_markers_aim_tremble_loop_from_and_fp() {
        let clip_json = r#"{
          "length": 20,
          "loop": true,
          "loop_from": 10,
          "fade_in": 4,
          "fade_out": 4,
          "both_arms_first_person": true,
          "markers": { "start": 0, "windup": 5, "loop_hit": 15 },
          "parts": {
            "body": [
              { "t": 0, "rot": [0, 0, 0] },
              { "t": 10, "rot": [0, 30, 0], "ease": "out_quad" }
            ],
            "right_arm": [
              { "t": 0, "rot": [-90, 0, 0] },
              { "t": 10, "rot": [-120, 10, 0], "ease": "catmullrom" },
              { "t": 20, "rot": [-90, 0, 0], "ease": "catmullrom" }
            ]
          },
          "aim": {
            "right_arm": [
              { "t": 0, "w": 0 },
              { "t": 10, "w": 1 }
            ]
          },
          "tremble": { "right_arm": 2.0 },
          "root": [
            { "t": 0, "rot": [0, 0, 0], "pos": [0, 0, 0], "scale": [1, 1, 1] },
            { "t": 10, "rot": [15, 0, 0], "pos": [0, -0.3, 0.1], "scale": [1.05, 0.95, 1.05] }
          ],
          "fp_right": [
            { "t": 0, "rot": [0, 0, 0], "pos": [0, 0, 0] },
            { "t": 10, "rot": [-30, 15, 0], "pos": [0.1, 0.2, -0.3] }
          ],
          "item_right": [
            { "t": 0, "rot": [0, 0, 0] },
            { "t": 10, "rot": [60, -30, 15] }
          ],
          "item_pivot_right": [-1.0, 7.5, -1.5]
        }"#;

        let mut ctrl = PlayerAnimationController::new();
        ctrl.register_clip("test_clip", PlayerAnimation::from_json_str(clip_json).unwrap());
        assert!(ctrl.play(7, "test_clip", 100.0));
        assert!(ctrl.is_animating(7));

        // Frame 0 fires "start" marker at tick 0.
        let ev0 = ctrl.advance_entity(7, 100.0);
        assert_eq!(ev0.len(), 1);
        assert_eq!(ev0[0].marker, "start");

        // Advance to tick 6 -> fires "windup" (5).
        let ev1 = ctrl.advance_entity(7, 106.0);
        assert_eq!(ev1.len(), 1);
        assert_eq!(ev1[0].marker, "windup");

        // Evaluate at tick 10 (full weight, body yaw = 30 deg, aim = 1.0).
        let mut pose = PlayerPose::vanilla_rest(false);
        pose.head.rot = Vec3::new(-0.5, 0.4, 0.0);
        ctrl.evaluate_pose(7, 110.0, 90.0, &[], Vec2::ZERO, &mut pose);
        assert!(pose.both_arms_first_person);
        assert!(pose.fp_right.is_some());
        assert!(pose.item_right_turn.is_some());
        let (turn, pivot) = pose.item_right_turn.unwrap();
        assert!(turn.is_finite());
        assert_eq!(pivot, Vec3::new(-1.0, 7.5, -1.5));
        assert_mat4_finite(pose.root_transform, "root_transform");
        assert!((pose.root_scale.y - 0.95).abs() < 1.0e-4);

        // First-person item orientation is finite.
        let fp_item = ctrl
            .first_person_item(7, HandSide::Right, 110.0, -0.3, Quat::IDENTITY)
            .unwrap();
        assert!(fp_item.is_finite());

        // Advance past loop boundary (100 + 26 -> time = 10 + 6 = 16): must fire loop_hit twice across wrap!
        let ev2 = ctrl.advance_entity(7, 116.0);
        assert_eq!(ev2.iter().map(|e| e.marker.as_str()).collect::<Vec<_>>(), vec!["loop_hit"]);
        let ev3 = ctrl.advance_entity(7, 126.0);
        assert_eq!(ev3.iter().map(|e| e.marker.as_str()).collect::<Vec<_>>(), vec!["loop_hit"]);

        // Stop & fade out.
        ctrl.stop(7, 130.0);
        assert!(ctrl.current(7, 132.0).unwrap().weight < 0.6);
        let _ = ctrl.advance_entity(7, 135.0);
        assert!(!ctrl.is_animating(7));
    }

    #[test]
    fn parse_and_evaluate_all_grandline_assets_on_disk() {
        let Some(base) = grandline_assets_dir() else {
            eprintln!("skipping Grand Line corpus test: asset dir not present");
            return;
        };

        let mut lib = GeoLibrary::new();

        // 1. ALL 45 Bedrock 1.12.0 .geo.json models
        let geo_count = lib.load_geo_dir(&base.join("geo")).expect("load all 45 geo models");
        assert_eq!(geo_count, 45, "expected all 45 Grand Line .geo.json files");

        let mut total_geo_verts = 0usize;
        for (id, model) in &lib.models {
            assert!(model.bone_count() > 0, "model {id} has no bones");
            let pose = GeoPose::new(model);
            let verts = model.evaluate_world_vertices(Mat4::IDENTITY, &pose);
            total_geo_verts += verts.len();
            for v in &verts {
                assert_vertex_finite(v, id);
            }
        }
        assert!(total_geo_verts > 0);

        // 2. ALL 30 Bedrock 1.8.0 .animation.json files (167 clips)
        let (anim_files, clip_count) = lib
            .load_geo_animations_dir(&base.join("geo_animations"))
            .expect("load all 30 geo animation files");
        assert_eq!(anim_files, 30, "expected all 30 Grand Line .animation.json files");
        assert_eq!(clip_count, 167, "expected all 167 Grand Line geo animation clips");

        for (clip_id, clip) in &lib.geo_animations {
            let model_key = clip_id
                .rsplit_once('/')
                .map(|(prefix, _)| prefix)
                .unwrap_or(clip_id);
            let matching_model = lib.models.get(model_key);

            for frac in [0.0_f32, 0.25, 0.5, 0.75, 1.0, 1.25] {
                let t_sec = clip.length_seconds * frac;
                for (bone, bp) in clip.sample_all(t_sec) {
                    assert_vec3_finite(bp.rotation_deg, &format!("{clip_id}:{bone}:rot"));
                    assert_vec3_finite(bp.position, &format!("{clip_id}:{bone}:pos"));
                    assert_vec3_finite(bp.scale, &format!("{clip_id}:{bone}:scale"));
                }
                if let Some(model) = matching_model {
                    let mut pose = GeoPose::new(model);
                    clip.blend_to_pose(model, &mut pose, t_sec * TICKS_PER_SECOND, 1.0);
                    let verts = model.evaluate_world_vertices(Mat4::IDENTITY, &pose);
                    for v in &verts {
                        assert_vertex_finite(v, clip_id);
                    }
                }
            }
        }

        // 3. ALL 213 player_animations/*.json clips
        let mut ctrl = PlayerAnimationController::new();
        let pa_count = ctrl
            .load_dir(&base.join("player_animations"))
            .expect("load all 213 player animations");
        assert_eq!(pa_count, 213, "expected all 213 Grand Line player_animations/*.json");

        let clip_ids: Vec<String> = ctrl.clips.keys().cloned().collect();
        for (idx, clip_id) in clip_ids.iter().enumerate() {
            let eid = idx as u32 + 1;
            let len = ctrl.clip(clip_id).unwrap().length;
            assert!(ctrl.play(eid, clip_id, 0.0));

            let sample_ticks = [
                0.0,
                (len * 0.25).max(0.5),
                (len * 0.5).max(1.0),
                len,
                len + 2.0,
            ];
            for &tick in &sample_ticks {
                let _events = ctrl.advance_entity(eid, tick);
                let mut pose = PlayerPose::vanilla_rest(false);
                pose.head.rot = Vec3::new(0.25, -0.35, 0.0);
                ctrl.evaluate_pose(eid, tick, 45.0, &[], Vec2::ZERO, &mut pose);

                for part in PlayerPart::ALL {
                    let pp = pose.part(part);
                    assert_vec3_finite(pp.rot, &format!("{clip_id}:{part:?}:rot"));
                    assert_vec3_finite(pp.pos, &format!("{clip_id}:{part:?}:pos"));
                }
                assert_mat4_finite(pose.root_transform, clip_id);
                if let Some(fp) = pose.fp_right {
                    assert_mat4_finite(fp, &format!("{clip_id}:fp_right"));
                }
                if let Some(fp) = pose.fp_left {
                    assert_mat4_finite(fp, &format!("{clip_id}:fp_left"));
                }
                if let Some((q, piv)) = pose.item_right_turn {
                    assert!(q.is_finite(), "non-finite item_right_turn in {clip_id}");
                    assert_vec3_finite(piv, &format!("{clip_id}:item_pivot_right"));
                }
                if let Some((q, piv)) = pose.item_left_turn {
                    assert!(q.is_finite(), "non-finite item_left_turn in {clip_id}");
                    assert_vec3_finite(piv, &format!("{clip_id}:item_pivot_left"));
                }
            }
        }
    }
}
