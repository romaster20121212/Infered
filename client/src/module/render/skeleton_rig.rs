 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::chams::{JOINT_STRIDE, SKELETON_JOINTS};

type V3 = [f32; 3];

 
 
pub(crate) const VERTEX_FLOATS: usize = 5;

 
 
 
const MAX_FLOATS: usize = 2_000_000;

 

 
 
 
 
pub(crate) const HEAD_PIVOT: usize = 0;
pub(crate) const CROWN: usize = 1;
pub(crate) const NECK: usize = 2;
pub(crate) const WAIST: usize = 3;
pub(crate) const SHOULDER_R: usize = 4;
pub(crate) const HAND_R: usize = 5;
pub(crate) const SHOULDER_L: usize = 6;
pub(crate) const HAND_L: usize = 7;
pub(crate) const HIP_R: usize = 8;
pub(crate) const FOOT_R: usize = 9;
pub(crate) const HIP_L: usize = 10;
pub(crate) const FOOT_L: usize = 11;

 
 
const RIB_SAMPLES: usize = 5;

 

 
 
 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Detail {
     
    pub(crate) sides: usize,
     
    pub(crate) rings: usize,
}

impl Detail {
    pub(crate) const LOW: Self = Self { sides: 6, rings: 3 };
    pub(crate) const MEDIUM: Self = Self { sides: 8, rings: 4 };
    pub(crate) const HIGH: Self = Self {
        sides: 12,
        rings: 6,
    };

     
    pub(crate) const OPTIONS: [&'static str; 3] = ["Low", "Medium", "High"];

    pub(crate) fn from_choice(choice: usize) -> Self {
        match choice {
            0 => Self::LOW,
            2 => Self::HIGH,
            _ => Self::MEDIUM,
        }
    }

    pub(crate) fn choice(self) -> usize {
        match self {
            Self::LOW => 0,
            Self::HIGH => 2,
            _ => 1,
        }
    }

     
     
     
    fn fine_sides(self) -> usize {
        (self.sides * 3 / 4).max(6)
    }
}

impl Default for Detail {
    fn default() -> Self {
        Self::MEDIUM
    }
}

 
 
#[derive(Clone, Copy, Debug)]
pub(crate) struct Anatomy {
    pub(crate) skull: bool,
    pub(crate) ribcage: bool,
    pub(crate) rib_pairs: usize,
    pub(crate) spine: bool,
    pub(crate) vertebrae: usize,
    pub(crate) pelvis: bool,
    pub(crate) clavicles: bool,
    pub(crate) arms: bool,
    pub(crate) legs: bool,
    pub(crate) hands: bool,
    pub(crate) feet: bool,
    pub(crate) joints: bool,
     
    pub(crate) thickness: f32,
    pub(crate) detail: Detail,
}

impl Default for Anatomy {
    fn default() -> Self {
        Self {
            skull: true,
            ribcage: true,
            rib_pairs: 6,
            spine: true,
            vertebrae: 7,
            pelvis: true,
            clavicles: true,
            arms: true,
            legs: true,
            hands: true,
            feet: true,
            joints: true,
            thickness: 0.035,
            detail: Detail::MEDIUM,
        }
    }
}

 

 
 
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub(crate) up: V3,
    pub(crate) right: V3,
    pub(crate) forward: V3,
}

 
 
 
 
 
 
pub(crate) fn frame(neck: V3, waist: V3, shoulder_r: V3, shoulder_l: V3) -> Frame {
    let up = unit(sub(neck, waist)).unwrap_or([0.0, 1.0, 0.0]);
     
     
    let lateral = sub(shoulder_r, shoulder_l);
    let right = unit(sub(lateral, mul(up, dot(lateral, up))))
        .or_else(|| unit(cross(up, [0.0, 0.0, 1.0])))
        .or_else(|| unit(cross(up, [1.0, 0.0, 0.0])))
        .unwrap_or([1.0, 0.0, 0.0]);
    let forward = unit(cross(up, right)).unwrap_or([0.0, 0.0, 1.0]);
    Frame {
        up,
        right: unit(cross(forward, up)).unwrap_or(right),
        forward,
    }
}

 
 
#[derive(Clone, Copy, Debug)]
struct Dims {
    torso: f32,
    span: f32,
    head: f32,
    arm: f32,
    leg: f32,
}

impl Dims {
     
     
     
    fn cage_half(&self) -> f32 {
        self.span * 0.42
    }

     
     
     
    fn cage_depth(&self) -> f32 {
        self.cage_half() * 0.62
    }
}

 

 
 
 
 
 
 
 
pub(crate) fn build(joints: &[f32], anatomy: &Anatomy) -> Vec<f32> {
    let mut out = Vec::new();
    if !anatomy.thickness.is_finite() {
        return out;
    }
    for chunk in joints.chunks_exact(SKELETON_JOINTS * JOINT_STRIDE) {
        if out.len() >= MAX_FLOATS {
            break;
        }
        let points: [V3; SKELETON_JOINTS] = std::array::from_fn(|i| {
            let at = i * JOINT_STRIDE;
            [chunk[at], chunk[at + 1], chunk[at + 2]]
        });
        if !points.iter().flatten().all(|v| v.is_finite()) {
            continue;
        }
        let mut one = Vec::new();
        build_player(&points, anatomy, &mut one);
        if out.len() + one.len() > MAX_FLOATS {
            break;
        }
        out.extend_from_slice(&one);
    }
    far_to_near(&mut out);
    out
}

fn build_player(points: &[V3; SKELETON_JOINTS], anatomy: &Anatomy, out: &mut Vec<f32>) {
     
     
     
    let torso = len(sub(points[NECK], points[WAIST]));
    let arm_length = (len(sub(points[HAND_R], points[SHOULDER_R]))
        + len(sub(points[HAND_L], points[SHOULDER_L])))
        * 0.5;
    let leg_length =
        (len(sub(points[FOOT_R], points[HIP_R])) + len(sub(points[FOOT_L], points[HIP_L]))) * 0.5;
    if !(torso > 0.05 || arm_length > 0.05 || leg_length > 0.05) {
        return;
    }
    let frame = frame(
        points[NECK],
        points[WAIST],
        points[SHOULDER_R],
        points[SHOULDER_L],
    );
    let dims = Dims {
        torso: clamp_finite(torso, 0.75, 0.1, 4.0),
        span: clamp_finite(
            len(sub(points[SHOULDER_R], points[SHOULDER_L])),
            0.62,
            0.1,
            4.0,
        ),
        head: clamp_finite(len(sub(points[CROWN], points[HEAD_PIVOT])), 0.5, 0.05, 2.0),
        arm: clamp_finite(arm_length, 0.62, 0.05, 4.0),
        leg: clamp_finite(leg_length, 0.75, 0.05, 4.0),
    };
    let base = clamp_finite(anatomy.thickness, 0.035, 0.002, 0.5);
    let detail = anatomy.detail;

    if anatomy.pelvis {
        pelvis(out, &frame, &dims, points, base, detail);
    }
    if anatomy.spine {
        spine(out, &frame, &dims, points, anatomy.vertebrae, base, detail);
    }
    if anatomy.ribcage {
        ribcage(out, &frame, &dims, points, anatomy.rib_pairs, base, detail);
    }
    if anatomy.clavicles {
        clavicles(out, &frame, &dims, points, base, detail);
    }
    if anatomy.skull {
        skull(out, &frame, &dims, points, base, detail);
    }
    if anatomy.arms {
        for (shoulder, hand) in [(SHOULDER_R, HAND_R), (SHOULDER_L, HAND_L)] {
            arm(
                out,
                &frame,
                &dims,
                points[shoulder],
                points[hand],
                anatomy.hands,
                base,
                detail,
            );
        }
    }
    if anatomy.legs {
        for (hip, foot) in [(HIP_R, FOOT_R), (HIP_L, FOOT_L)] {
            leg(
                out,
                &frame,
                &dims,
                points[hip],
                points[foot],
                anatomy.feet,
                base,
                detail,
            );
        }
    }
    if anatomy.joints {
        joint_balls(out, &frame, &dims, points, anatomy, base, detail);
    }
}

 

 
 
 
 
 
fn spine(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    points: &[V3; SKELETON_JOINTS],
    vertebrae: usize,
    base: f32,
    detail: Detail,
) {
    let count = vertebrae.clamp(1, 16);
    for i in 0..count {
        let t = (i as f32 + 0.5) / count as f32;
        let center = at(points[WAIST], points[NECK], t);
        let radius = base * (1.5 - 0.5 * t);
        push_ellipsoid(
            out,
            center,
            [frame.right, frame.up, frame.forward],
            [radius * 1.9, radius * 1.0, radius * 1.5],
            detail.fine_sides(),
            detail.rings.clamp(2, 4),
        );
    }
     
    let neck_top = add(points[NECK], mul(frame.up, dims.head * 0.25));
    push_capsule(
        out,
        points[NECK],
        neck_top,
        base * 0.9,
        detail.fine_sides(),
        1,
    );
}

 
 
fn ribcage(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    points: &[V3; SKELETON_JOINTS],
    pairs: usize,
    base: f32,
    detail: Detail,
) {
    let pairs = pairs.clamp(0, 10);
    let half = dims.cage_half();
     
     
     
    let front = dims.cage_depth();
    for i in 0..pairs {
         
         
        let t = if pairs > 1 {
            0.18 + 0.74 * i as f32 / (pairs - 1) as f32
        } else {
            0.55
        };
        let spine_point = at(points[WAIST], points[NECK], t);
         
         
        let taper = 0.60 + 0.40 * (std::f32::consts::PI * t).sin();
        let width = half * taper;
         
         
        let drop = front * taper * (0.8 - 0.5 * t);
        for side in [-1.0f32, 1.0] {
            let mut path = Vec::with_capacity(RIB_SAMPLES);
            for k in 0..RIB_SAMPLES {
                let a = -std::f32::consts::FRAC_PI_2
                    + std::f32::consts::PI * k as f32 / (RIB_SAMPLES - 1) as f32;
                let (sin, cos) = a.sin_cos();
                let lateral = width * cos;
                let forward = front * (sin + 1.0) * 0.5;
                let down = drop * (sin + 1.0) * 0.5;
                path.push(add(
                    spine_point,
                    add(
                        mul(frame.right, side * lateral),
                        add(mul(frame.forward, forward), mul(frame.up, -down)),
                    ),
                ));
            }
            push_tube(out, &path, base * 0.32, detail.fine_sides());
        }
    }
     
     
    let sternum = at(points[WAIST], points[NECK], 0.62);
    push_box(
        out,
        add(sternum, mul(frame.forward, front)),
        [frame.right, frame.up, frame.forward],
        [dims.span * 0.045, dims.torso * 0.24, base * 0.45],
    );
     
     
     
    for side in [-1.0f32, 1.0] {
        let center = add(
            at(points[WAIST], points[NECK], 0.80),
            add(
                mul(frame.right, side * dims.span * 0.20),
                mul(frame.forward, -front * 0.5),
            ),
        );
        push_box(
            out,
            center,
            [frame.right, frame.up, frame.forward],
            [dims.span * 0.075, dims.torso * 0.10, base * 0.35],
        );
    }
}

 
 
 
 
 
fn skull(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    points: &[V3; SKELETON_JOINTS],
    base: f32,
    detail: Detail,
) {
    let head = dims.head;
    let center = add(points[HEAD_PIVOT], mul(frame.up, head * 0.55));
     
     
    push_ellipsoid(
        out,
        center,
        [frame.right, frame.up, frame.forward],
        [head * 0.42, head * 0.52, head * 0.46],
        detail.sides,
        detail.rings.max(4),
    );
     
     
    let jaw = add(
        add(center, mul(frame.forward, head * 0.22)),
        mul(frame.up, -head * 0.30),
    );
    push_ellipsoid(
        out,
        jaw,
        [frame.right, frame.up, frame.forward],
        [head * 0.28, head * 0.16, head * 0.24],
        detail.fine_sides(),
        3,
    );
     
     
    let brow = add(
        add(center, mul(frame.forward, head * 0.40)),
        mul(frame.up, head * 0.08),
    );
    push_box(
        out,
        brow,
        [frame.right, frame.up, frame.forward],
        [head * 0.30, head * 0.05, base * 0.6],
    );
}

 
 
 
fn pelvis(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    points: &[V3; SKELETON_JOINTS],
    base: f32,
    detail: Detail,
) {
    let waist = points[WAIST];
    let basis = [frame.right, frame.up, frame.forward];
     
    push_ellipsoid(
        out,
        waist,
        basis,
        [dims.span * 0.11, dims.torso * 0.08, dims.span * 0.10],
        detail.fine_sides(),
        3,
    );
     
     
     
    for side in [-1.0f32, 1.0] {
        let center = add(
            waist,
            add(
                mul(frame.right, side * dims.span * 0.22),
                mul(frame.up, dims.torso * 0.02),
            ),
        );
        push_ellipsoid(
            out,
            center,
            basis,
            [dims.span * 0.09, dims.torso * 0.14, dims.span * 0.07],
            detail.fine_sides(),
            detail.rings.clamp(2, 4),
        );
    }
    let pubis = add(
        add(waist, mul(frame.forward, dims.span * 0.08)),
        mul(frame.up, -dims.torso * 0.09),
    );
    push_box(
        out,
        pubis,
        basis,
        [dims.span * 0.10, dims.torso * 0.035, base * 0.7],
    );
     
    for hip in [HIP_R, HIP_L] {
        push_ellipsoid(
            out,
            points[hip],
            basis,
            [base * 1.5, base * 1.5, base * 1.5],
            detail.fine_sides(),
            3,
        );
    }
}

 
fn clavicles(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    points: &[V3; SKELETON_JOINTS],
    base: f32,
    detail: Detail,
) {
    let top = add(
        add(points[NECK], mul(frame.up, -dims.torso * 0.18)),
        mul(frame.forward, dims.cage_depth() * 0.9),
    );
    for shoulder in [SHOULDER_R, SHOULDER_L] {
        push_capsule(
            out,
            top,
            points[shoulder],
            base * 0.62,
            detail.fine_sides(),
            1,
        );
    }
}

 
fn arm(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    shoulder: V3,
    hand: V3,
    fingers: bool,
    base: f32,
    detail: Detail,
) {
    let Some(dir) = unit(sub(hand, shoulder)) else {
        return;
    };
    let length = dims.arm.max(0.05);
     
     
     
    let elbow = add(at(shoulder, hand, 0.52), mul(frame.forward, -length * 0.08));
    push_capsule(out, shoulder, elbow, base, detail.sides, 1);
     
     
    if let Some(side) = unit(cross(dir, frame.forward)) {
        let gap = length * 0.05;
        for sign in [-1.0f32, 1.0] {
            let offset = mul(side, sign * gap);
            push_capsule(
                out,
                add(elbow, offset),
                add(hand, mul(offset, 0.6)),
                base * 0.5,
                detail.fine_sides(),
                1,
            );
        }
    }
     
    let palm = add(hand, mul(dir, length * 0.10));
    push_capsule(out, hand, palm, base * 0.8, detail.fine_sides(), 1);
    if !fingers {
        return;
    }
    let Some(side) = unit(cross(dir, frame.forward)) else {
        return;
    };
    for k in 0..4 {
        let spread = (k as f32 - 1.5) * 0.30;
        let tip = add(
            add(palm, mul(dir, length * 0.15)),
            mul(side, spread * length * 0.06),
        );
        push_capsule(out, palm, tip, base * 0.22, 4, 1);
    }
}

 
fn leg(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    hip: V3,
    ankle: V3,
    foot: bool,
    base: f32,
    detail: Detail,
) {
    let Some(dir) = unit(sub(ankle, hip)) else {
        return;
    };
    let length = dims.leg.max(0.05);
     
    let knee = add(at(hip, ankle, 0.5), mul(frame.forward, length * 0.05));
    push_capsule(out, hip, knee, base * 1.2, detail.sides, 1);
    if let Some(side) = unit(cross(dir, frame.forward)) {
        let gap = length * 0.045;
         
         
        push_capsule(
            out,
            add(knee, mul(side, gap)),
            add(ankle, mul(side, gap * 0.7)),
            base * 0.62,
            detail.fine_sides(),
            1,
        );
        push_capsule(
            out,
            add(knee, mul(side, -gap)),
            add(ankle, mul(side, -gap * 0.7)),
            base * 0.32,
            4,
            1,
        );
    }
    if !foot {
        return;
    }
     
     
    let stride = length * 0.17;
    let sole = mul(frame.up, -length * 0.03);
    push_box(
        out,
        add(add(ankle, mul(frame.forward, stride * 0.38)), sole),
        [frame.right, frame.up, frame.forward],
        [length * 0.055, length * 0.030, stride * 0.55],
    );
    push_box(
        out,
        add(
            add(ankle, mul(frame.forward, -stride * 0.16)),
            mul(frame.up, -length * 0.045),
        ),
        [frame.right, frame.up, frame.forward],
        [length * 0.045, length * 0.045, stride * 0.18],
    );
}

 
 
fn joint_balls(
    out: &mut Vec<f32>,
    frame: &Frame,
    dims: &Dims,
    points: &[V3; SKELETON_JOINTS],
    anatomy: &Anatomy,
    base: f32,
    detail: Detail,
) {
    let radius = base * 1.35;
    let basis = [frame.right, frame.up, frame.forward];
    let mut centers = vec![points[NECK], points[WAIST]];
    for (shoulder, hand) in [(SHOULDER_R, HAND_R), (SHOULDER_L, HAND_L)] {
        centers.push(points[shoulder]);
         
        centers.push(add(
            at(points[shoulder], points[hand], 0.52),
            mul(frame.forward, -dims.arm * 0.08),
        ));
    }
    for (hip, foot) in [(HIP_R, FOOT_R), (HIP_L, FOOT_L)] {
        centers.push(points[hip]);
         
        centers.push(add(
            at(points[hip], points[foot], 0.5),
            mul(frame.forward, dims.leg * 0.05),
        ));
        if anatomy.feet {
            centers.push(points[foot]);
        }
    }
    if anatomy.hands {
        centers.push(points[HAND_R]);
        centers.push(points[HAND_L]);
    }
    for center in centers {
        push_ellipsoid(
            out,
            center,
            basis,
            [radius; 3],
            detail.fine_sides(),
            detail.rings.clamp(2, 4),
        );
    }
}

 

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn len(a: V3) -> f32 {
    dot(a, a).sqrt()
}

fn unit(a: V3) -> Option<V3> {
    let length = len(a);
    if !length.is_finite() || length < 1e-6 {
        return None;
    }
    Some(mul(a, 1.0 / length))
}

 
fn at(a: V3, b: V3, t: f32) -> V3 {
    add(a, mul(sub(b, a), t))
}

 
 
fn clamp_finite(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

 
 
 
 
 
fn oct_encode(n: V3) -> [f32; 2] {
    let sum = n[0].abs() + n[1].abs() + n[2].abs();
    if !sum.is_finite() || sum < 1e-6 {
        return [0.0, 0.0];
    }
    let (mut x, mut y) = (n[0] / sum, n[1] / sum);
    if n[2] < 0.0 {
        let sign = |v: f32| if v >= 0.0 { 1.0 } else { -1.0 };
        let ox = (1.0 - y.abs()) * sign(x);
        let oy = (1.0 - x.abs()) * sign(y);
        x = ox;
        y = oy;
    }
    [x, y]
}

fn push_vertex(out: &mut Vec<f32>, position: V3, normal: V3) {
    let Some(n) = unit(normal) else {
         
         
        let uv = oct_encode([0.0, 1.0, 0.0]);
        out.extend_from_slice(&[position[0], position[1], position[2], uv[0], uv[1]]);
        return;
    };
    let uv = oct_encode(n);
    out.extend_from_slice(&[position[0], position[1], position[2], uv[0], uv[1]]);
}

fn push_triangle(out: &mut Vec<f32>, positions: [V3; 3], normals: [V3; 3]) {
    for i in 0..3 {
        push_vertex(out, positions[i], normals[i]);
    }
}

 
fn push_quad(out: &mut Vec<f32>, corners: [V3; 4], normal: V3) {
    push_triangle(out, [corners[0], corners[1], corners[2]], [normal; 3]);
    push_triangle(out, [corners[0], corners[2], corners[3]], [normal; 3]);
}

 
 
 
 
 
fn push_capsule(out: &mut Vec<f32>, a: V3, b: V3, radius: f32, sides: usize, cap_segments: usize) {
    let Some(axis) = unit(sub(b, a)) else {
        return;
    };
    if !radius.is_finite() || radius <= 0.0 {
        return;
    }
    let sides = sides.max(3);
    let cap_segments = cap_segments.max(1);
    let helper = if axis[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let Some(v) = unit(cross(helper, axis)) else {
        return;
    };
    let w = cross(axis, v);

     
     
     
    let mut rings: Vec<(V3, f32, f32, f32)> = Vec::with_capacity(2 * (cap_segments + 1));
    for (end, half) in [(a, 0.0), (b, 1.0)] {
        for i in 0..=cap_segments {
            let phi = std::f32::consts::FRAC_PI_2 * (half + i as f32 / cap_segments as f32);
            rings.push((end, -phi.cos() * radius, phi.sin() * radius, phi.cos()));
        }
    }

    let vertex = |ring: (V3, f32, f32, f32), side: usize| -> (V3, V3) {
        let angle = std::f32::consts::TAU * side as f32 / sides as f32;
        let (center, axial, ring_radius, cos_phi) = ring;
        let (s, c) = angle.sin_cos();
        let radial = add(mul(v, c), mul(w, s));
        let position = add(center, add(mul(axis, axial), mul(radial, ring_radius)));
         
         
        let sin_phi = if radius > 0.0 {
            ring_radius / radius
        } else {
            0.0
        };
        let normal = add(mul(axis, -cos_phi), mul(radial, sin_phi));
        (position, normal)
    };

    for k in 0..rings.len() - 1 {
        for side in 0..sides {
            let (a0, n0) = vertex(rings[k], side);
            let (a1, n1) = vertex(rings[k], (side + 1) % sides);
            let (b0, m0) = vertex(rings[k + 1], side);
            let (b1, m1) = vertex(rings[k + 1], (side + 1) % sides);
            push_triangle(out, [a0, b0, b1], [n0, m0, m1]);
            push_triangle(out, [a0, b1, a1], [n0, m1, n1]);
        }
    }
}

 
 
 
fn push_tube(out: &mut Vec<f32>, points: &[V3], radius: f32, sides: usize) {
    if points.len() < 2 || !radius.is_finite() || radius <= 0.0 {
        return;
    }
    let sides = sides.max(3);
     
    let tangents: Vec<V3> = (0..points.len())
        .map(|i| {
            let previous = if i == 0 {
                None
            } else {
                unit(sub(points[i], points[i - 1]))
            };
            let next = if i + 1 == points.len() {
                None
            } else {
                unit(sub(points[i + 1], points[i]))
            };
            match (previous, next) {
                (Some(p), Some(n)) => unit(add(p, n)).unwrap_or(n),
                (Some(p), None) => p,
                (None, Some(n)) => n,
                (None, None) => [0.0, 1.0, 0.0],
            }
        })
        .collect();

     
     
    let helper = if tangents[0][1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let mut carried = unit(cross(helper, tangents[0])).unwrap_or([1.0, 0.0, 0.0]);
    let mut rings: Vec<(V3, V3, V3)> = Vec::with_capacity(points.len());
    for (i, tangent) in tangents.iter().enumerate() {
        let projected = sub(carried, mul(*tangent, dot(carried, *tangent)));
        carried = unit(projected)
            .or_else(|| unit(cross(helper, *tangent)))
            .unwrap_or([1.0, 0.0, 0.0]);
        rings.push((points[i], carried, cross(*tangent, carried)));
    }

    let ring_vertex = |ring: (V3, V3, V3), side: usize| -> (V3, V3) {
        let angle = std::f32::consts::TAU * side as f32 / sides as f32;
        let (s, c) = angle.sin_cos();
        let radial = add(mul(ring.1, c), mul(ring.2, s));
        (add(ring.0, mul(radial, radius)), radial)
    };

    for k in 0..rings.len() - 1 {
        for side in 0..sides {
            let (a0, n0) = ring_vertex(rings[k], side);
            let (a1, n1) = ring_vertex(rings[k], (side + 1) % sides);
            let (b0, m0) = ring_vertex(rings[k + 1], side);
            let (b1, m1) = ring_vertex(rings[k + 1], (side + 1) % sides);
            push_triangle(out, [a0, b0, b1], [n0, m0, m1]);
            push_triangle(out, [a0, b1, a1], [n0, m1, n1]);
        }
    }
     
     
    for (index, normal) in [
        (0usize, mul(tangents[0], -1.0)),
        (rings.len() - 1, tangents[rings.len() - 1]),
    ] {
        let ring = rings[index];
        for side in 0..sides {
            let (p0, _) = ring_vertex(ring, side);
            let (p1, _) = ring_vertex(ring, (side + 1) % sides);
            push_triangle(out, [ring.0, p0, p1], [normal; 3]);
        }
    }
}

 
 
fn push_ellipsoid(
    out: &mut Vec<f32>,
    center: V3,
    basis: [V3; 3],
    radii: V3,
    sides: usize,
    rings: usize,
) {
    if !radii.iter().all(|r| r.is_finite() && *r > 0.0) {
        return;
    }
    let sides = sides.max(3);
    let rings = rings.max(2);
    let vertex = |ring: usize, side: usize| -> (V3, V3) {
        let theta = std::f32::consts::PI * ring as f32 / rings as f32;
        let phi = std::f32::consts::TAU * side as f32 / sides as f32;
        let (sin_theta, cos_theta) = theta.sin_cos();
        let (sin_phi, cos_phi) = phi.sin_cos();
        let local = [sin_theta * cos_phi, cos_theta, sin_theta * sin_phi];
        let position = add(
            center,
            add(
                mul(basis[0], local[0] * radii[0]),
                add(
                    mul(basis[1], local[1] * radii[1]),
                    mul(basis[2], local[2] * radii[2]),
                ),
            ),
        );
         
         
        let normal = add(
            mul(basis[0], local[0] / radii[0]),
            add(
                mul(basis[1], local[1] / radii[1]),
                mul(basis[2], local[2] / radii[2]),
            ),
        );
        (position, normal)
    };
    for ring in 0..rings {
        for side in 0..sides {
            let (p00, n00) = vertex(ring, side);
            let (p01, n01) = vertex(ring, side + 1);
            let (p10, n10) = vertex(ring + 1, side);
            let (p11, n11) = vertex(ring + 1, side + 1);
            push_triangle(out, [p00, p10, p11], [n00, n10, n11]);
            push_triangle(out, [p00, p11, p01], [n00, n11, n01]);
        }
    }
}

 
 
fn push_box(out: &mut Vec<f32>, center: V3, basis: [V3; 3], half: V3) {
    if !half.iter().all(|h| h.is_finite() && *h > 0.0) {
        return;
    }
    let corner = |sx: f32, sy: f32, sz: f32| -> V3 {
        add(
            center,
            add(
                mul(basis[0], sx * half[0]),
                add(mul(basis[1], sy * half[1]), mul(basis[2], sz * half[2])),
            ),
        )
    };
     
    let faces: [([V3; 4], V3); 6] = [
        (
            [
                corner(1.0, -1.0, -1.0),
                corner(1.0, 1.0, -1.0),
                corner(1.0, 1.0, 1.0),
                corner(1.0, -1.0, 1.0),
            ],
            basis[0],
        ),
        (
            [
                corner(-1.0, -1.0, 1.0),
                corner(-1.0, 1.0, 1.0),
                corner(-1.0, 1.0, -1.0),
                corner(-1.0, -1.0, -1.0),
            ],
            mul(basis[0], -1.0),
        ),
        (
            [
                corner(-1.0, 1.0, -1.0),
                corner(-1.0, 1.0, 1.0),
                corner(1.0, 1.0, 1.0),
                corner(1.0, 1.0, -1.0),
            ],
            basis[1],
        ),
        (
            [
                corner(-1.0, -1.0, 1.0),
                corner(-1.0, -1.0, -1.0),
                corner(1.0, -1.0, -1.0),
                corner(1.0, -1.0, 1.0),
            ],
            mul(basis[1], -1.0),
        ),
        (
            [
                corner(-1.0, -1.0, 1.0),
                corner(1.0, -1.0, 1.0),
                corner(1.0, 1.0, 1.0),
                corner(-1.0, 1.0, 1.0),
            ],
            basis[2],
        ),
        (
            [
                corner(1.0, -1.0, -1.0),
                corner(-1.0, -1.0, -1.0),
                corner(-1.0, 1.0, -1.0),
                corner(1.0, 1.0, -1.0),
            ],
            mul(basis[2], -1.0),
        ),
    ];
    for (corners, normal) in faces {
        push_quad(out, corners, normal);
    }
}

 
 
 
 
 
 
 
fn far_to_near(vertices: &mut Vec<f32>) {
    const TRIANGLE: usize = VERTEX_FLOATS * 3;
    if vertices.len() < TRIANGLE || vertices.len() % TRIANGLE != 0 {
        return;
    }
    let count = vertices.len() / TRIANGLE;
    let mut order: Vec<(f32, usize)> = (0..count)
        .map(|index| {
            let t = &vertices[index * TRIANGLE..(index + 1) * TRIANGLE];
            let distance = [0, 1, 2]
                .iter()
                .map(|&axis| {
                    (t[axis] + t[VERTEX_FLOATS + axis] + t[2 * VERTEX_FLOATS + axis]) / 3.0
                })
                .map(|v| v * v)
                .sum::<f32>();
            (distance, index)
        })
        .collect();
    order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    let mut sorted = Vec::with_capacity(vertices.len());
    for (_, index) in order {
        sorted.extend_from_slice(&vertices[index * TRIANGLE..(index + 1) * TRIANGLE]);
    }
    *vertices = sorted;
}

 
 
 
 
#[cfg(test)]
pub(crate) fn standing_pose() -> [V3; SKELETON_JOINTS] {
    let mut points = [[0.0f32; 3]; SKELETON_JOINTS];
    points[HEAD_PIVOT] = [0.0, 1.5, 0.0];
    points[CROWN] = [0.0, 2.0, 0.0];
    points[NECK] = [0.0, 1.5, 0.0];
    points[WAIST] = [0.0, 0.75, 0.0];
    points[SHOULDER_R] = [-0.31, 1.44, 0.0];
    points[HAND_R] = [-0.31, 0.81, 0.0];
    points[SHOULDER_L] = [0.31, 1.44, 0.0];
    points[HAND_L] = [0.31, 0.81, 0.0];
    points[HIP_R] = [-0.12, 0.75, 0.0];
    points[FOOT_R] = [-0.12, 0.0, 0.0];
    points[HIP_L] = [0.12, 0.75, 0.0];
    points[FOOT_L] = [0.12, 0.0, 0.0];
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose() -> [V3; SKELETON_JOINTS] {
        standing_pose()
    }

    fn flat(points: [V3; SKELETON_JOINTS]) -> Vec<f32> {
        points.iter().flat_map(|p| p.iter().copied()).collect()
    }

     
     
    fn only(points: [V3; SKELETON_JOINTS], part: impl Fn(&mut Anatomy)) -> Vec<f32> {
        let mut anatomy = Anatomy {
            skull: false,
            ribcage: false,
            rib_pairs: 0,
            spine: false,
            vertebrae: 0,
            pelvis: false,
            clavicles: false,
            arms: false,
            legs: false,
            hands: false,
            feet: false,
            joints: false,
            ..Anatomy::default()
        };
        part(&mut anatomy);
        build(&flat(points), &anatomy)
    }

    fn bounds(mesh: &[f32]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for vertex in mesh.chunks_exact(VERTEX_FLOATS) {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex[axis]);
                max[axis] = max[axis].max(vertex[axis]);
            }
        }
        (min, max)
    }

     
     
     
     
    #[test]
    fn the_frame_faces_where_the_model_faces() {
        let p = pose();
        let frame = frame(p[NECK], p[WAIST], p[SHOULDER_R], p[SHOULDER_L]);
        assert!((frame.up[1] - 1.0).abs() < 1e-4, "up {:?}", frame.up);
        assert!(frame.right[0] < -0.99, "right {:?}", frame.right);
        assert!(frame.forward[2] > 0.99, "forward {:?}", frame.forward);
         
        for (a, b) in [
            (frame.up, frame.right),
            (frame.up, frame.forward),
            (frame.right, frame.forward),
        ] {
            assert!(dot(a, b).abs() < 1e-4, "axes are not square: {a:?} {b:?}");
        }
    }

     
     
     
    #[test]
    fn a_leaning_torso_tilts_the_whole_rig() {
        let mut p = pose();
         
        p[NECK] = [0.0, 1.45, 0.35];
        p[HEAD_PIVOT] = [0.0, 1.45, 0.35];
        p[CROWN] = [0.0, 1.92, 0.55];
        let frame = frame(p[NECK], p[WAIST], p[SHOULDER_R], p[SHOULDER_L]);
        assert!(
            frame.up[2] > 0.2,
            "up did not follow the lean: {:?}",
            frame.up
        );
        let mesh = only(p, |a| a.ribcage = true);
        assert!(!mesh.is_empty());
        assert!(mesh.iter().all(|v| v.is_finite()));
    }

     
     
    #[test]
    fn the_feet_point_the_way_the_player_faces() {
        let p = pose();
        let mesh = only(p, |a| {
            a.legs = true;
            a.feet = true;
        });
        let (min, max) = bounds(&mesh);
         
        assert!(max[2] > 0.08, "no toe in front of the ankle: {max:?}");
        assert!(min[2] < 0.0, "no heel behind the ankle: {min:?}");
    }

     
     
    #[test]
    fn the_skull_sits_between_the_neck_and_the_crown() {
        let p = pose();
        let mesh = only(p, |a| a.skull = true);
        let (min, max) = bounds(&mesh);
        assert!(min[1] > p[HEAD_PIVOT][1] - 0.06, "skull sinks: {min:?}");
        assert!(
            max[1] < p[CROWN][1] + 0.08,
            "skull floats above the crown: {max:?}"
        );
        assert!(
            max[1] - min[1] > 0.3,
            "the skull is not tall enough to be a skull"
        );
    }

     
     
    #[test]
    fn the_ribcage_wraps_around_the_chest() {
        let p = pose();
        let mesh = only(p, |a| {
            a.ribcage = true;
            a.rib_pairs = 6;
        });
        let (min, max) = bounds(&mesh);
        assert!(max[0] - min[0] > 0.4, "no width across the chest: {max:?}");
        assert!(max[2] > 0.14, "no sternum in front: {max:?}");
         
        assert!(
            max[2] < (max[0] - min[0]) * 0.6,
            "the cage is deeper than a chest: {max:?}"
        );
        assert!(min[1] > 0.6 && max[1] < 1.6, "the cage misses the chest");
    }

     
    #[test]
    fn every_anatomy_switch_changes_the_rig() {
         
         
         
        let cases: [(&str, fn(&mut Anatomy), fn(&mut Anatomy)); 11] = [
            ("skull", |_| {}, |a| a.skull = true),
            ("ribcage", |_| {}, |a| a.ribcage = true),
            ("spine", |_| {}, |a| a.spine = true),
            ("pelvis", |_| {}, |a| a.pelvis = true),
            ("clavicles", |_| {}, |a| a.clavicles = true),
            ("arms", |_| {}, |a| a.arms = true),
            ("legs", |_| {}, |a| a.legs = true),
            (
                "hands",
                |a| a.arms = true,
                |a| {
                    a.arms = true;
                    a.hands = true;
                },
            ),
            (
                "feet",
                |a| a.legs = true,
                |a| {
                    a.legs = true;
                    a.feet = true;
                },
            ),
            ("joints", |_| {}, |a| a.joints = true),
            (
                "detail",
                |a| a.arms = true,
                |a| {
                    a.arms = true;
                    a.detail = Detail::HIGH;
                },
            ),
        ];
        for (name, base, set) in cases {
            let off = only(pose(), base);
            let on = only(pose(), set);
            assert!(
                on.len() > off.len(),
                "the {name} switch draws nothing extra ({} vs {})",
                on.len(),
                off.len()
            );
        }
    }

     
     
    #[test]
    fn the_default_rig_has_limbs() {
        let full = build(&flat(pose()), &Anatomy::default());
        let (min, max) = bounds(&full);
        assert!(min[1] < 0.1, "the legs stop above the ankles: {min:?}");
        assert!(max[0] - min[0] > 0.5, "the arms are missing: {max:?}");
         
        let arms = only(pose(), |a| a.arms = true);
        assert!(!arms.is_empty());
        let (min, max) = bounds(&arms);
        assert!(min[1] > 0.5, "the arm rig grew legs: {min:?}");
        assert!(max[0] - min[0] > 0.5, "the arms are not drawn: {max:?}");
    }

     
    #[test]
    fn more_rib_pairs_draw_more_ribs() {
        let few = only(pose(), |a| {
            a.ribcage = true;
            a.rib_pairs = 3;
        });
        let many = only(pose(), |a| {
            a.ribcage = true;
            a.rib_pairs = 9;
        });
        assert!(
            many.len() > few.len() * 2,
            "rib pairs barely change the cage: {} vs {}",
            few.len(),
            many.len()
        );
    }

     
    #[test]
    fn thickness_scales_the_whole_rig() {
        let width = |thickness: f32| {
            let mut anatomy = Anatomy::default();
            anatomy.thickness = thickness;
            let mesh = build(&flat(pose()), &anatomy);
            let (min, max) = bounds(&mesh);
            max[0] - min[0]
        };
        let thin = width(0.03);
        let thick = width(0.06);
        assert!(thin.is_finite() && thick.is_finite());
        assert!(
            thick > thin,
            "a thicker rig is not wider: {thin} -> {thick}"
        );
    }

     
     
    #[test]
    fn every_vertex_is_finite_and_carries_a_packable_normal() {
        let mesh = build(&flat(pose()), &Anatomy::default());
        assert_eq!(mesh.len() % VERTEX_FLOATS, 0);
        assert_eq!(mesh.len() % (VERTEX_FLOATS * 3), 0, "half a triangle");
        let mut vertices = 0;
        for vertex in mesh.chunks_exact(VERTEX_FLOATS) {
            assert!(vertex.iter().all(|v| v.is_finite()), "{vertex:?}");
            assert!(
                vertex[3].abs() <= 1.0001 && vertex[4].abs() <= 1.0001,
                "packed normal outside the disc: {vertex:?}"
            );
            vertices += 1;
        }
         
         
        assert!(vertices > 2_000, "only {vertices} vertices");
        assert!(vertices < 20_000, "{vertices} vertices is too many to draw");
    }

     
     
    #[test]
    fn triangles_go_to_the_buffer_far_first() {
        let mut p = pose();
        for point in p.iter_mut() {
            point[2] -= 6.0;
        }
        let mesh = build(&flat(p), &Anatomy::default());
        let mut previous = f32::INFINITY;
        for triangle in mesh.chunks_exact(VERTEX_FLOATS * 3) {
            let distance = [0, 1, 2]
                .iter()
                .map(|&axis| {
                    (triangle[axis]
                        + triangle[VERTEX_FLOATS + axis]
                        + triangle[2 * VERTEX_FLOATS + axis])
                        / 3.0
                })
                .map(|v| v * v)
                .sum::<f32>();
            assert!(
                distance <= previous + 1e-2,
                "triangle order is not far-to-near: {distance} after {previous}"
            );
            previous = distance;
        }
    }

     
     
    #[test]
    fn a_degenerate_pose_draws_nothing() {
        let points = [[0.0f32; 3]; SKELETON_JOINTS];
        let mesh = build(&flat(points), &Anatomy::default());
        assert!(
            mesh.is_empty(),
            "{} floats from a collapsed pose",
            mesh.len()
        );
    }

     
    #[test]
    fn a_broken_player_is_skipped_not_fatal() {
        let mut joints = flat(pose());
        joints.extend_from_slice(&[f32::NAN; SKELETON_JOINTS * JOINT_STRIDE]);
        joints.extend_from_slice(&flat(pose()));
        let mesh = build(&joints, &Anatomy::default());
        assert!(!mesh.is_empty());
        assert!(mesh.iter().all(|v| v.is_finite()));
         
         
        let one = build(&flat(pose()), &Anatomy::default());
        assert!(
            (mesh.len() as f32 / one.len() as f32 - 2.0).abs() < 0.05,
            "{} vs {}",
            mesh.len(),
            one.len()
        );
    }

     
    #[test]
    fn detail_only_changes_the_tessellation() {
        let low = Detail::LOW;
        let high = Detail::HIGH;
        assert!(high.sides > low.sides && high.rings > low.rings);
        assert_eq!(Detail::from_choice(0), Detail::LOW);
        assert_eq!(Detail::from_choice(1), Detail::MEDIUM);
        assert_eq!(Detail::from_choice(2), Detail::HIGH);
         
        assert_eq!(Detail::from_choice(99), Detail::MEDIUM);
        assert_eq!(Detail::from_choice(high.choice()), Detail::HIGH);
        let mut anatomy = Anatomy::default();
        anatomy.detail = low;
        let coarse = build(&flat(pose()), &anatomy);
        anatomy.detail = high;
        let fine = build(&flat(pose()), &anatomy);
        assert!(fine.len() > coarse.len());
    }

     
     
     
    fn oct_decode(e: [f32; 2]) -> V3 {
        let (mut x, mut y) = (e[0], e[1]);
        let z = 1.0 - x.abs() - y.abs();
        if z < 0.0 {
            let sign = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
            let (sx, sy) = (sign(x), sign(y));
            let (ox, oy) = ((1.0 - y.abs()) * sx, (1.0 - x.abs()) * sy);
            x = ox;
            y = oy;
        }
        let length = (x * x + y * y + z * z).sqrt();
        [x / length, y / length, z / length]
    }

    #[test]
    fn normals_survive_the_octahedral_round_trip() {
        let mut checked = 0;
        for n in [
            [0.0f32, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.577, 0.577, 0.577],
            [-0.577, 0.577, -0.577],
            [0.27, -0.41, -0.87],
        ] {
            let uv = oct_encode(n);
            assert!(uv[0].abs().max(uv[1].abs()) <= 1.0001, "{uv:?}");
            assert!(uv[0].is_finite() && uv[1].is_finite());
            let back = oct_decode(uv);
            for i in 0..3 {
                assert!((back[i] - n[i]).abs() < 2e-3, "{n:?} decoded as {back:?}");
            }
            checked += 1;
        }
        assert_eq!(checked, 9);
    }

     
     
    #[test]
    fn every_direction_packs_within_the_disc() {
        let mut checked = 0;
        for x in -8..=8 {
            for y in -8..=8 {
                for z in -8..=8 {
                    let n = [x as f32, y as f32, z as f32];
                    let Some(n) = unit(n) else { continue };
                    let uv = oct_encode(n);
                    assert!(uv[0].abs().max(uv[1].abs()) <= 1.0001, "{n:?} -> {uv:?}");
                    let back = oct_decode(uv);
                    for i in 0..3 {
                        assert!((back[i] - n[i]).abs() < 2e-3, "{n:?} -> {back:?}");
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked > 1_000);
    }

     
    #[test]
    fn primitives_reject_degenerate_input() {
        let mut out = Vec::new();
        push_capsule(&mut out, [0.0; 3], [0.0; 3], 0.02, 8, 1);
        push_capsule(&mut out, [0.0; 3], [0.0, 1.0, 0.0], 0.0, 8, 1);
        push_capsule(&mut out, [0.0; 3], [0.0, 1.0, 0.0], f32::NAN, 8, 1);
        push_tube(&mut out, &[], 0.02, 6);
        push_tube(&mut out, &[[0.0; 3]], 0.02, 6);
        push_ellipsoid(
            &mut out,
            [0.0; 3],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0, 1.0, 1.0],
            6,
            3,
        );
        push_box(
            &mut out,
            [0.0; 3],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [1.0, -1.0, 1.0],
        );
        assert!(out.is_empty(), "a degenerate primitive emitted geometry");
    }
}
