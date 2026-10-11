//! Geographic areas for location locks.
//!
//! Location is never certain, so every check takes the fix's accuracy into
//! account and answers `Unknown` when the fix could be on either side of
//! the boundary. Callers treat `Unknown` as inside (fail-closed).

use serde::{Deserialize, Serialize};

const EARTH_RADIUS_M: f64 = 6_371_008.8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum GeoArea {
    Circle { lat: f64, lon: f64, radius_m: f64 },
    /// Vertices as `[lat, lon]`, in order; the ring closes automatically.
    Polygon { points: Vec<[f64; 2]> },
}

/// A position estimate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocationFix {
    /// Location services off, no Wi-Fi, lookup failed, ...
    Unknown,
    Known { lat: f64, lon: f64, accuracy_m: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Inside,
    Outside,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GeoError {
    #[error("coordinates out of range")]
    BadCoordinates,
    #[error("radius must be between 10 m and 100 km")]
    BadRadius,
    #[error("an area needs at least 3 points")]
    TooFewPoints,
}

impl GeoArea {
    pub fn validate(&self) -> Result<(), GeoError> {
        let ok = |lat: f64, lon: f64| lat.is_finite() && lon.is_finite() && lat.abs() <= 90.0 && lon.abs() <= 180.0;
        match self {
            GeoArea::Circle { lat, lon, radius_m } => {
                if !ok(*lat, *lon) {
                    return Err(GeoError::BadCoordinates);
                }
                if !(10.0..=100_000.0).contains(radius_m) {
                    return Err(GeoError::BadRadius);
                }
            }
            GeoArea::Polygon { points } => {
                if points.len() < 3 {
                    return Err(GeoError::TooFewPoints);
                }
                if !points.iter().all(|[lat, lon]| ok(*lat, *lon)) {
                    return Err(GeoError::BadCoordinates);
                }
            }
        }
        Ok(())
    }

    pub fn relation(&self, fix: LocationFix) -> Relation {
        let LocationFix::Known { lat, lon, accuracy_m } = fix else { return Relation::Unknown };
        if !lat.is_finite() || !lon.is_finite() {
            return Relation::Unknown;
        }
        let accuracy = if accuracy_m.is_finite() { accuracy_m.max(0.0) } else { f64::INFINITY };
        // Signed distance to the boundary: negative inside, positive outside.
        let signed = match self {
            GeoArea::Circle { lat: clat, lon: clon, radius_m } => {
                haversine_m(lat, lon, *clat, *clon) - radius_m
            }
            GeoArea::Polygon { points } => {
                let d = distance_to_ring_m(lat, lon, points);
                if point_in_polygon(lat, lon, points) { -d } else { d }
            }
        };
        if signed + accuracy <= 0.0 {
            Relation::Inside
        } else if signed - accuracy > 0.0 {
            Relation::Outside
        } else {
            Relation::Unknown
        }
    }
}

pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = p2 - p1;
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

/// Ray casting in lat/lon space; fine for areas of city size and smaller.
fn point_in_polygon(lat: f64, lon: f64, ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let [yi, xi] = ring[i];
        let [yj, xj] = ring[j];
        if (yi > lat) != (yj > lat) && lon < (xj - xi) * (lat - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Shortest distance from the point to the polygon outline, using a local
/// flat projection centred on the point.
fn distance_to_ring_m(lat: f64, lon: f64, ring: &[[f64; 2]]) -> f64 {
    let k = EARTH_RADIUS_M * std::f64::consts::PI / 180.0;
    let cos_lat = lat.to_radians().cos();
    let project = |[plat, plon]: [f64; 2]| ((plon - lon) * cos_lat * k, (plat - lat) * k);
    let mut best = f64::INFINITY;
    for i in 0..ring.len() {
        let (ax, ay) = project(ring[i]);
        let (bx, by) = project(ring[(i + 1) % ring.len()]);
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 { 0.0 } else { (-(ax * dx + ay * dy) / len2).clamp(0.0, 1.0) };
        let (cx, cy) = (ax + t * dx, ay + t * dy);
        best = best.min((cx * cx + cy * cy).sqrt());
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    // Around the Kaaba, Makkah.
    const LAT: f64 = 21.4225;
    const LON: f64 = 39.8262;

    fn fix(lat: f64, lon: f64, accuracy_m: f64) -> LocationFix {
        LocationFix::Known { lat, lon, accuracy_m }
    }

    /// Offsets a point north by `m` metres.
    fn north(m: f64) -> f64 {
        LAT + m / (EARTH_RADIUS_M * std::f64::consts::PI / 180.0)
    }

    #[test]
    fn haversine_known_distance() {
        // Makkah → Madinah is roughly 337 km in a straight line.
        let d = haversine_m(LAT, LON, 24.4672, 39.6112);
        assert!((330_000.0..345_000.0).contains(&d), "{d}");
    }

    #[test]
    fn circle_relation() {
        let area = GeoArea::Circle { lat: LAT, lon: LON, radius_m: 500.0 };
        assert_eq!(area.relation(fix(LAT, LON, 20.0)), Relation::Inside);
        assert_eq!(area.relation(fix(north(2000.0), LON, 20.0)), Relation::Outside);
        // 450 m away with 100 m accuracy could be either side.
        assert_eq!(area.relation(fix(north(450.0), LON, 100.0)), Relation::Unknown);
        assert_eq!(area.relation(LocationFix::Unknown), Relation::Unknown);
        assert_eq!(area.relation(fix(LAT, LON, f64::NAN)), Relation::Unknown);
    }

    #[test]
    fn polygon_relation() {
        let d = 0.01; // ~1.1 km
        let area = GeoArea::Polygon {
            points: vec![[LAT - d, LON - d], [LAT - d, LON + d], [LAT + d, LON + d], [LAT + d, LON - d]],
        };
        assert_eq!(area.relation(fix(LAT, LON, 50.0)), Relation::Inside);
        assert_eq!(area.relation(fix(LAT + 0.05, LON, 50.0)), Relation::Outside);
        assert_eq!(area.relation(fix(LAT + d, LON, 50.0)), Relation::Unknown, "on the edge");
        assert_eq!(area.relation(fix(LAT, LON, 5_000.0)), Relation::Unknown, "very inaccurate");
    }

    #[test]
    fn validation() {
        assert!(GeoArea::Circle { lat: LAT, lon: LON, radius_m: 200.0 }.validate().is_ok());
        assert_eq!(GeoArea::Circle { lat: 91.0, lon: 0.0, radius_m: 200.0 }.validate(), Err(GeoError::BadCoordinates));
        assert_eq!(GeoArea::Circle { lat: 0.0, lon: 0.0, radius_m: 1.0 }.validate(), Err(GeoError::BadRadius));
        assert_eq!(GeoArea::Polygon { points: vec![[0.0, 0.0]; 2] }.validate(), Err(GeoError::TooFewPoints));
    }
}
