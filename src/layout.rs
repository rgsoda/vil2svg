//! Physical key positions, read from a QMK `info.json`.
//!
//! This is the `qmk_info_json` slice of keymap-drawer's `physical_layout.py`.
//! The Corne has no rotated keys and no ISO enter, so the rotation and
//! ISO-enter machinery from upstream is deliberately absent.

use serde::Deserialize;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl std::ops::Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl std::ops::Mul<f64> for Point {
    type Output = Point;
    fn mul(self, k: f64) -> Point {
        Point::new(self.x * k, self.y * k)
    }
}

#[derive(Clone, Debug)]
pub struct PhysicalKey {
    /// Centre of the key, in pixels.
    pub pos: Point,
    pub width: f64,
    pub height: f64,
}

impl PhysicalKey {
    fn scaled(&self, k: f64) -> PhysicalKey {
        PhysicalKey {
            pos: self.pos * k,
            width: self.width * k,
            height: self.height * k,
        }
    }
}

#[derive(Deserialize)]
struct QmkKey {
    x: f64,
    y: f64,
    #[serde(default = "one")]
    w: f64,
    #[serde(default = "one")]
    h: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Clone, Debug)]
pub struct PhysicalLayout {
    pub keys: Vec<PhysicalKey>,
}

impl PhysicalLayout {
    /// Read one named layout out of a QMK `info.json`, scaling `1u` to
    /// `key_size` pixels on both axes and normalising to the origin.
    ///
    /// The scale is uniform: upstream only uses the separate `key_w` for its
    /// ortho layout generator, not for `info.json` layouts.
    pub fn from_qmk_info_json(
        path: &Path,
        layout_name: &str,
        key_size: f64,
    ) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let info: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("could not parse {}: {e}", path.display()))?;
        let raw = info
            .get("layouts")
            .and_then(|l| l.get(layout_name))
            .and_then(|l| l.get("layout"))
            .ok_or_else(|| format!("no layout {layout_name} in {}", path.display()))?;
        let qmk_keys: Vec<QmkKey> = serde_json::from_value(raw.clone())
            .map_err(|e| format!("malformed layout {layout_name}: {e}"))?;

        let keys = qmk_keys
            .iter()
            .map(|k| PhysicalKey {
                // upstream stores the centre, not the top-left corner
                pos: Point::new((k.x + k.w / 2.0) * key_size, (k.y + k.h / 2.0) * key_size),
                width: k.w * key_size,
                height: k.h * key_size,
            })
            .collect();

        Ok(PhysicalLayout { keys }.normalize())
    }

    fn normalize(mut self) -> Self {
        let min_x = self
            .keys
            .iter()
            .map(|k| k.pos.x - k.width / 2.0)
            .fold(f64::INFINITY, f64::min);
        let min_y = self
            .keys
            .iter()
            .map(|k| k.pos.y - k.height / 2.0)
            .fold(f64::INFINITY, f64::min);
        let shift = Point::new(min_x, min_y);
        for k in &mut self.keys {
            k.pos = k.pos - shift;
        }
        self
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn width(&self) -> f64 {
        self.keys
            .iter()
            .map(|k| k.pos.x + k.width / 2.0)
            .fold(0.0, f64::max)
    }

    pub fn height(&self) -> f64 {
        self.keys
            .iter()
            .map(|k| k.pos.y + k.height / 2.0)
            .fold(0.0, f64::max)
    }

    /// Shrunk copy, used for the separate combo diagrams.
    pub fn scaled(&self, k: f64) -> Self {
        PhysicalLayout {
            keys: self.keys.iter().map(|key| key.scaled(k)).collect(),
        }
    }
}
