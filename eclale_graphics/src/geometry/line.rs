use flo_curves::bezier::{self, *};
use nalgebra::{Vector2, Vector3};

#[derive(Clone)]
pub struct Line {
    pub points: Vec<Vector3<f32>>,
}

#[derive(Clone)]
pub struct Curve {
    pub v0: Vector2<f32>,
    pub v1: Vector2<f32>,
    pub control_points: (Vector2<f32>, Vector2<f32>),
}

fn create_cubic_bezier_curve(
    v0: Vector2<f32>,
    v1: Vector2<f32>,
    control_points: (Vector2<f32>, Vector2<f32>),
) -> bezier::Curve<Coord2> {
    bezier::Curve::from_points(
        flo_curves::Coord2(v0.x as _, v0.y as _),
        (
            flo_curves::Coord2(control_points.0.x as _, control_points.0.y as _),
            flo_curves::Coord2(control_points.1.x as _, control_points.1.y as _),
        ),
        flo_curves::Coord2(v1.x as _, v1.y as _),
    )
}

/// Creates points for a curve on the xz 3D axis given cubic bezier parameters.
/// Points are represented as Vec2s in the xz axis.
pub(crate) fn cubic_bezier_curve_points_xz(
    v0: Vector2<f32>,
    v1: Vector2<f32>,
    control_points: (Vector2<f32>, Vector2<f32>),
    subdivisions: usize,
) -> Vec<Vector3<f32>> {
    let curve = create_cubic_bezier_curve(v0, v1, control_points);
    (0..subdivisions + 1)
        .step_by(1)
        .map(|t| curve.point_at_pos(t as f64 / subdivisions as f64))
        .map(|c| Vector3::new(c.0 as f32, 0.0, c.1 as f32))
        .collect::<Vec<_>>()
}

/// Gets a point's position within a curve.
pub fn cubic_bezier_curve_point_at_pos(
    v0: Vector2<f32>,
    v1: Vector2<f32>,
    control_points: (Vector2<f32>, Vector2<f32>),
    t: f64,
) -> Vector2<f32> {
    let curve = create_cubic_bezier_curve(v0, v1, control_points);
    let point = curve.point_at_pos(t);
    Vector2::new(point.0 as f32, point.1 as f32)
}

impl Curve {
    pub fn new(
        v0: Vector2<f32>,
        v1: Vector2<f32>,
        control_points: (Vector2<f32>, Vector2<f32>),
    ) -> Self {
        Self {
            v0,
            v1,
            control_points,
        }
    }

    pub fn to_points(self, subdivisions: usize) -> Vec<Vector3<f32>> {
        cubic_bezier_curve_points_xz(self.v0, self.v1, self.control_points, subdivisions)
    }
}

impl Line {
    pub fn from_points(points: Vec<Vector3<f32>>) -> Self {
        Self { points }
    }

    pub fn from_curve(curve: Curve, subdivisions: usize) -> Self {
        Self {
            points: curve.to_points(subdivisions),
        }
    }
}
