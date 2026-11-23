//! # maniac
//!
//! A tiny, fast, very colorful ASCII animation engine.
//!
//! Every animation is a pure function of time: `|canvas, tick| { draw stuff }`.
//! A [`Scene`] is a timeline of those functions. Run it in the terminal with
//! [`Scene::run`], or bake it into a self-contained HTML player (JetBrains Mono,
//! bloom, seeking) with [`Scene::html`]. [`Scene::show`] picks for you.
//!
//! The world is like manim's: origin in the middle, y points up, the screen is
//! always 8 units tall (`y` in `-4..4`) and `c.w()` units wide on each side.
//!
//! Three layers are composed into each terminal cell:
//! * **pixels** — two full-color pixels per cell (`▀` half blocks): fills, shaders, glows, big text
//! * **dots**   — eight braille dots per cell: hairline strokes, curves, particles
//! * **text**   — plain characters: labels, typewriter, lit 3D surfaces

use std::io::Write;
use std::time::{Duration, Instant};

pub use std::f32::consts::{PI, TAU};

// ───────────────────────────── color ─────────────────────────────

/// A 24-bit color.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn hex(h: u32) -> Rgb {
        Rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
    }
    /// Blend toward `o` by `t` (0 = self, 1 = o).
    pub fn mix(self, o: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t + 0.5) as u8;
        Rgb(m(self.0, o.0), m(self.1, o.1), m(self.2, o.2))
    }
    /// Brightness: 0 = black, 1 = unchanged.
    pub fn dim(self, k: f32) -> Rgb {
        Rgb(0, 0, 0).mix(self, k)
    }
}

/// Hue in turns (0..1 wraps), saturation and value in 0..1.
pub fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let (i, f) = (h as i32, h.fract());
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    let u = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u8;
    Rgb(u(r), u(g), u(b))
}

/// A pastel-neon rainbow. Works directly as an [`Ink`]: `c.stroke(&p, rainbow)`.
pub fn rainbow(t: f32) -> Rgb {
    hsv(t, 0.7, 1.0)
}

/// Sample a multi-stop gradient at `t` in 0..1.
pub fn grad(stops: &[Rgb], t: f32) -> Rgb {
    if stops.len() < 2 {
        return stops.first().copied().unwrap_or_default();
    }
    let x = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (x as usize).min(stops.len() - 2);
    stops[i].mix(stops[i + 1], x - i as f32)
}

pub const BG: Rgb = Rgb::hex(0x0B0D17);
pub const WHITE: Rgb = Rgb::hex(0xF2F2F2);
pub const GREY: Rgb = Rgb::hex(0x7A8099);
pub const BLACK: Rgb = Rgb::hex(0x000000);
pub const BLUE: Rgb = Rgb::hex(0x58C4DD);
pub const SKY: Rgb = Rgb::hex(0x7FD4FF);
pub const TEAL: Rgb = Rgb::hex(0x5CD0B3);
pub const GREEN: Rgb = Rgb::hex(0x83C167);
pub const LIME: Rgb = Rgb::hex(0xB6FF3B);
pub const YELLOW: Rgb = Rgb::hex(0xF7E86F);
pub const GOLD: Rgb = Rgb::hex(0xF0B453);
pub const ORANGE: Rgb = Rgb::hex(0xFF862F);
pub const RED: Rgb = Rgb::hex(0xFC6255);
pub const PINK: Rgb = Rgb::hex(0xFF6FB5);
pub const MAGENTA: Rgb = Rgb::hex(0xFF2BD6);
pub const PURPLE: Rgb = Rgb::hex(0x9A72FF);
pub const CYAN: Rgb = Rgb::hex(0x00F0FF);

pub const SUNSET: [Rgb; 3] = [Rgb::hex(0xFF2E63), Rgb::hex(0xFF8C42), Rgb::hex(0xFFE66D)];
pub const OCEAN: [Rgb; 3] = [Rgb::hex(0x3A86FF), Rgb::hex(0x00F5D4), Rgb::hex(0xB8F2E6)];
pub const NEON: [Rgb; 3] = [Rgb::hex(0xFF00E5), Rgb::hex(0x7A5CFF), Rgb::hex(0x00E5FF)];
pub const AURORA: [Rgb; 3] = [Rgb::hex(0x00FFA3), Rgb::hex(0x03E1FF), Rgb::hex(0xDC1FFF)];
pub const FIRE: [Rgb; 4] = [Rgb::hex(0x3A0A00), Rgb::hex(0xC81D11), Rgb::hex(0xFF8C00), Rgb::hex(0xFFF3B0)];
pub const ICE: [Rgb; 3] = [Rgb::hex(0x10204A), Rgb::hex(0x2F80ED), Rgb::hex(0xCFF4FF)];

/// Anything that gives a color for a position `t` in 0..1 along a stroke, string or fill.
/// Implemented for a plain [`Rgb`], gradient arrays like [`SUNSET`], and any `Fn(f32) -> Rgb`.
pub trait Ink {
    fn at(&self, t: f32) -> Rgb;
}
impl Ink for Rgb {
    fn at(&self, _: f32) -> Rgb {
        *self
    }
}
impl<const N: usize> Ink for [Rgb; N] {
    fn at(&self, t: f32) -> Rgb {
        grad(self, t)
    }
}
impl<F: Fn(f32) -> Rgb> Ink for F {
    fn at(&self, t: f32) -> Rgb {
        self(t)
    }
}

// ───────────────────────────── math ─────────────────────────────

pub type V2 = (f32, f32);
pub type V3 = (f32, f32, f32);

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
pub fn lerp2(a: V2, b: V2, t: f32) -> V2 {
    (lerp(a.0, b.0, t), lerp(a.1, b.1, t))
}
pub fn dist(a: V2, b: V2) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
pub fn rot2((x, y): V2, a: f32) -> V2 {
    let (s, c) = a.sin_cos();
    (x * c - y * s, x * s + y * c)
}
/// Rotate around x, then y, then z.
pub fn rot3((x, y, z): V3, ax: f32, ay: f32, az: f32) -> V3 {
    let (y, z) = rot2((y, z), ax);
    let (z, x) = rot2((z, x), ay);
    let (x, y) = rot2((x, y), az);
    (x, y, z)
}
pub fn add3(a: V3, b: V3) -> V3 {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2)
}
fn sub3(a: V3, b: V3) -> V3 {
    (a.0 - b.0, a.1 - b.1, a.2 - b.2)
}
fn norm3(a: V3) -> V3 {
    let l = (a.0 * a.0 + a.1 * a.1 + a.2 * a.2).sqrt().max(1e-9);
    (a.0 / l, a.1 / l, a.2 / l)
}
/// Torus surface for [`Canvas::surface`]; `u`, `v` in 0..1.
pub fn torus(u: f32, v: f32, big: f32, small: f32) -> V3 {
    let (a, b) = (u * TAU, v * TAU);
    let r = big + small * b.cos();
    (r * a.cos(), r * a.sin(), small * b.sin())
}
/// Sphere surface for [`Canvas::surface`]; `u`, `v` in 0..1.
pub fn sphere(u: f32, v: f32, r: f32) -> V3 {
    let (a, b) = (u * TAU, (v - 0.5) * PI);
    (r * b.cos() * a.cos(), r * b.sin(), r * b.cos() * a.sin())
}
/// Unit cube wireframe: (vertices, edges) for [`Canvas::mesh`].
pub fn cube() -> (Vec<V3>, Vec<(usize, usize)>) {
    let v = (0..8).map(|i| ((i & 1) as f32 * 2.0 - 1.0, (i >> 1 & 1) as f32 * 2.0 - 1.0, (i >> 2) as f32 * 2.0 - 1.0)).collect();
    let e = (0..8usize).flat_map(|i| [1, 2, 4].into_iter().filter(move |b| i & b == 0).map(move |b| (i, i | b))).collect();
    (v, e)
}

/// Deterministic pseudo-random number in 0..1. Same input, same output, so scenes stay seekable.
pub fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}
/// Smooth 2D value noise in 0..1.
pub fn noise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (ease::smooth(x - ix as f32), ease::smooth(y - iy as f32));
    let h = |a: i32, b: i32| hash((a as u32).wrapping_mul(73_856_093) ^ (b as u32).wrapping_mul(19_349_663));
    lerp(lerp(h(ix, iy), h(ix + 1, iy), fx), lerp(h(ix, iy + 1), h(ix + 1, iy + 1), fx), fy)
}
/// manim's LaggedStart: the local progress of item `i` of `n` when items start `lag` apart (0 = together, 1 = one after another).
pub fn stagger(p: f32, i: usize, n: usize, lag: f32) -> f32 {
    let d = 1.0 / (1.0 + (n.max(1) - 1) as f32 * lag);
    ((p - i as f32 * lag * d) / d).clamp(0.0, 1.0)
}

/// Easing curves, `fn(f32) -> f32` on 0..1.
pub mod ease {
    use std::f32::consts::TAU;
    pub fn linear(t: f32) -> f32 {
        t
    }
    pub fn smooth(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }
    pub fn in_cubic(t: f32) -> f32 {
        t * t * t
    }
    pub fn out_cubic(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(3)
    }
    pub fn in_out(t: f32) -> f32 {
        if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (2.0 - 2.0 * t).powi(3) / 2.0 }
    }
    pub fn back(t: f32) -> f32 {
        let c = 1.70158;
        1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
    }
    pub fn elastic(t: f32) -> f32 {
        if t <= 0.0 || t >= 1.0 { t.clamp(0.0, 1.0) } else { 2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * TAU / 3.0).sin() + 1.0 }
    }
    pub fn bounce(t: f32) -> f32 {
        let (n, d) = (7.5625, 2.75);
        if t < 1.0 / d {
            n * t * t
        } else if t < 2.0 / d {
            let t = t - 1.5 / d;
            n * t * t + 0.75
        } else if t < 2.5 / d {
            let t = t - 2.25 / d;
            n * t * t + 0.9375
        } else {
            let t = t - 2.625 / d;
            n * t * t + 0.984375
        }
    }
    pub fn there_and_back(t: f32) -> f32 {
        smooth(1.0 - (2.0 * t - 1.0).abs())
    }
}

// ───────────────────────────── paths ─────────────────────────────

/// A polyline in world units. Every built-in shape starts at the top and runs clockwise,
/// so any two of them [`morph`](Path::morph) into each other cleanly.
#[derive(Clone, Debug, Default)]
pub struct Path {
    pub pts: Vec<V2>,
    pub closed: bool,
}

impl Path {
    pub fn new(pts: Vec<V2>, closed: bool) -> Path {
        Path { pts, closed }
    }
    pub fn line(a: V2, b: V2) -> Path {
        Path::new(vec![a, b], false)
    }
    pub fn poly(n: usize, r: f32) -> Path {
        Path::new((0..n).map(|i| rot2((0.0, r), -TAU * i as f32 / n as f32)).collect(), true)
    }
    pub fn circle(r: f32) -> Path {
        Path::poly(120, r)
    }
    pub fn star(n: usize, outer: f32, inner: f32) -> Path {
        let pts = (0..n * 2).map(|i| rot2((0.0, if i % 2 == 0 { outer } else { inner }), -PI * i as f32 / n as f32));
        Path::new(pts.collect(), true)
    }
    pub fn rect(w: f32, h: f32) -> Path {
        let (x, y) = (w / 2.0, h / 2.0);
        Path::new(vec![(0.0, y), (x, y), (x, -y), (-x, -y), (-x, y)], true)
    }
    pub fn heart(s: f32) -> Path {
        Path::param(|t| {
            let t = t * TAU;
            let x = 16.0 * t.sin().powi(3);
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            (x * s / 16.0, (y + 2.0) * s / 16.0)
        })
        .close()
    }
    /// Graph of `y = f(x)` for x in `x0..x1`.
    pub fn func(f: impl Fn(f32) -> f32, x0: f32, x1: f32) -> Path {
        Path::new((0..=300).map(|i| lerp(x0, x1, i as f32 / 300.0)).map(|x| (x, f(x))).collect(), false)
    }
    /// Parametric curve, `t` runs 0..1.
    pub fn param(f: impl Fn(f32) -> V2) -> Path {
        Path::new((0..=360).map(|i| f(i as f32 / 360.0)).collect(), false)
    }
    pub fn close(mut self) -> Path {
        self.closed = true;
        self
    }
    /// Apply any point transform.
    pub fn map(mut self, f: impl Fn(V2) -> V2) -> Path {
        self.pts.iter_mut().for_each(|p| *p = f(*p));
        self
    }
    pub fn at(self, x: f32, y: f32) -> Path {
        self.map(|(a, b)| (a + x, b + y))
    }
    pub fn scale(self, s: f32) -> Path {
        self.map(|(a, b)| (a * s, b * s))
    }
    pub fn rot(self, a: f32) -> Path {
        self.map(|p| rot2(p, a))
    }
    fn segs(&self) -> impl Iterator<Item = (V2, V2)> + '_ {
        let close = (self.closed && self.pts.len() > 2).then(|| (self.pts[self.pts.len() - 1], self.pts[0]));
        self.pts.windows(2).map(|w| (w[0], w[1])).chain(close)
    }
    pub fn len(&self) -> f32 {
        self.segs().map(|(a, b)| dist(a, b)).sum()
    }
    /// Point at fraction `t` of the arc length.
    pub fn point(&self, t: f32) -> V2 {
        let mut left = t.clamp(0.0, 1.0) * self.len();
        for (a, b) in self.segs() {
            let l = dist(a, b);
            if left <= l && l > 0.0 {
                return lerp2(a, b, left / l);
            }
            left -= l;
        }
        self.pts.last().copied().unwrap_or_default()
    }
    /// `n` points evenly spaced by arc length.
    pub fn resample(&self, n: usize) -> Path {
        let total = self.len();
        if total <= 0.0 || n < 2 {
            return Path::new(vec![self.pts.first().copied().unwrap_or_default(); n], self.closed);
        }
        let span = if self.closed { n } else { n - 1 } as f32;
        let (mut out, mut segs) = (Vec::with_capacity(n), self.segs().peekable());
        let mut acc = 0.0;
        for i in 0..n {
            let want = total * i as f32 / span;
            while let Some(&(a, b)) = segs.peek() {
                let l = dist(a, b);
                if acc + l >= want {
                    out.push(if l > 0.0 { lerp2(a, b, (want - acc) / l) } else { a });
                    break;
                }
                acc += l;
                segs.next();
            }
            if out.len() <= i {
                out.push(*self.pts.last().unwrap());
            }
        }
        Path::new(out, self.closed)
    }
    /// The first `p` (0..1) of the path — manim's `Create`.
    pub fn partial(&self, p: f32) -> Path {
        if p >= 1.0 {
            return self.clone();
        }
        let mut left = p.max(0.0) * self.len();
        let mut out = vec![self.pts.first().copied().unwrap_or_default()];
        for (a, b) in self.segs() {
            let l = dist(a, b);
            if left <= l {
                out.push(if l > 0.0 { lerp2(a, b, left / l) } else { b });
                break;
            }
            out.push(b);
            left -= l;
        }
        Path::new(out, false)
    }
    /// Smoothly turn `self` into `other` — manim's `Transform`.
    pub fn morph(&self, other: &Path, p: f32) -> Path {
        let (a, b) = (self.resample(240), other.resample(240));
        let pts = a.pts.iter().zip(&b.pts).map(|(&x, &y)| lerp2(x, y, p)).collect();
        Path::new(pts, if p < 0.5 { self.closed } else { other.closed })
    }
}

// ───────────────────────────── physics ─────────────────────────────

/// A deterministic, seekable simulation. `at(t)` steps forward in fixed `dt` steps from the last state,
/// or restarts from `init` when time goes backwards, so scenes stay pure functions of time (and HTML
/// export, seeking and looping all still work).
pub struct Sim<S> {
    dt: f32,
    init: Box<dyn Fn() -> S>,
    step: Box<dyn Fn(&mut S, f32, f32)>,
    state: std::cell::RefCell<(f32, S)>,
}

impl<S> Sim<S> {
    /// `step(state, time, dt)` advances the state by one tick.
    pub fn new(dt: f32, init: impl Fn() -> S + 'static, step: impl Fn(&mut S, f32, f32) + 'static) -> Sim<S> {
        let s = init();
        Sim { dt, init: Box::new(init), step: Box::new(step), state: std::cell::RefCell::new((0.0, s)) }
    }
    /// The state at time `t`.
    pub fn at(&self, t: f32) -> std::cell::Ref<'_, S> {
        {
            let mut st = self.state.borrow_mut();
            if t < st.0 {
                *st = (0.0, (self.init)());
            }
            let (now, s) = &mut *st;
            while *now + self.dt <= t {
                (self.step)(s, *now, self.dt);
                *now += self.dt;
            }
        }
        std::cell::Ref::map(self.state.borrow(), |s| &s.1)
    }
}

/// A springy rope of points (verlet). Node 0 is pinned; every other node is pulled back toward its rest
/// pose, hard near the root and softly near the tip, so it sways like seaweed, hair or an anemone arm.
#[derive(Clone, Debug)]
pub struct Chain {
    pub pts: Vec<V2>,
    prev: Vec<V2>,
    rest: Vec<V2>,
    seg: f32,
    /// Spring strength toward the rest pose at the (root, tip).
    pub stiff: (f32, f32),
    /// Velocity kept each step (0..1).
    pub damping: f32,
}

impl Chain {
    /// Grow `n` nodes, `len` long in total, from `root` at `angle`, turning by `curl` radians per segment.
    pub fn new(root: V2, angle: f32, n: usize, len: f32, curl: f32) -> Chain {
        let n = n.max(2);
        let seg = len / (n - 1) as f32;
        let (mut pts, mut a) = (vec![root], angle);
        for _ in 1..n {
            let p = pts[pts.len() - 1];
            pts.push((p.0 + a.cos() * seg, p.1 + a.sin() * seg));
            a += curl;
        }
        Chain { prev: pts.clone(), rest: pts.clone(), pts, seg, stiff: (40.0, 3.0), damping: 0.97 }
    }
    /// Advance by `dt` under an acceleration field `force(point, node)` (currents, wind, gravity).
    pub fn step(&mut self, dt: f32, force: impl Fn(V2, usize) -> V2) {
        let n = self.pts.len();
        for i in 1..n {
            let k = lerp(self.stiff.0, self.stiff.1, i as f32 / (n - 1) as f32);
            let (p, q, r, f) = (self.pts[i], self.prev[i], self.rest[i], force(self.pts[i], i));
            let acc = (f.0 + k * (r.0 - p.0), f.1 + k * (r.1 - p.1));
            self.prev[i] = p;
            self.pts[i] = (p.0 + (p.0 - q.0) * self.damping + acc.0 * dt * dt, p.1 + (p.1 - q.1) * self.damping + acc.1 * dt * dt);
        }
        for _ in 0..3 {
            for i in 1..n {
                let (a, b) = (self.pts[i - 1], self.pts[i]);
                let k = self.seg / dist(a, b).max(1e-6);
                self.pts[i] = (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
            }
        }
    }
    /// Shove every node out of a circle (a swimmer, a hand, a rock).
    pub fn push_out(&mut self, c: V2, r: f32) {
        for p in self.pts.iter_mut().skip(1) {
            let d = dist(*p, c);
            if d < r && d > 1e-6 {
                *p = (c.0 + (p.0 - c.0) * r / d, c.1 + (p.1 - c.1) * r / d);
            }
        }
    }
    pub fn path(&self) -> Path {
        Path::new(self.pts.clone(), false)
    }
}

// ───────────────────────────── canvas ─────────────────────────────

#[derive(Clone, Copy, PartialEq)]
struct Cell {
    ch: char,
    fg: Rgb,
    bg: Rgb,
}

/// What every draw call writes into. Coordinates are world units (see crate docs).
pub struct Canvas {
    pub cols: usize,
    pub rows: usize,
    pub bg: Rgb,
    /// Opacity of everything drawn right now. Scenes set it for fades.
    pub alpha: f32,
    unit: f32,
    dots: Vec<u8>,
    dcol: Vec<Rgb>,
    pix: Vec<Rgb>,
    txt: Vec<(char, Rgb)>,
    zb: Vec<f32>,
}

const BRAILLE: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
const RAMP: &[u8] = b".,-~:;=!*#$@";

impl Canvas {
    pub fn new(cols: usize, rows: usize, bg: Rgb) -> Canvas {
        let n = cols * rows;
        Canvas {
            cols, rows, bg, alpha: 1.0,
            unit: rows as f32 / 2.0,
            dots: vec![0; n], dcol: vec![bg; n], pix: vec![bg; n * 2],
            txt: vec![('\0', bg); n], zb: vec![f32::NEG_INFINITY; n],
        }
    }
    pub fn clear(&mut self) {
        self.dots.fill(0);
        self.pix.fill(self.bg);
        self.txt.fill(('\0', self.bg));
        self.zb.fill(f32::NEG_INFINITY);
        self.alpha = 1.0;
    }
    /// Half-width of the visible world. (Half-height is always 4.)
    pub fn w(&self) -> f32 {
        self.cols as f32 / self.unit
    }
    pub fn h(&self) -> f32 {
        4.0
    }
    fn to_dot(&self, (x, y): V2) -> V2 {
        (self.cols as f32 + x * self.unit, self.rows as f32 * 2.0 - y * self.unit)
    }
    fn col(&self, c: Rgb) -> Rgb {
        if self.alpha >= 1.0 { c } else { self.bg.mix(c, self.alpha) }
    }

    // ── dots: hairlines ──

    fn plot(&mut self, x: i32, y: i32, c: Rgb) {
        if x < 0 || y < 0 || x >= self.cols as i32 * 2 || y >= self.rows as i32 * 4 {
            return;
        }
        let i = (y as usize / 4) * self.cols + x as usize / 2;
        self.dots[i] |= BRAILLE[x as usize % 2][y as usize % 4];
        self.dcol[i] = c;
    }
    pub fn dot(&mut self, p: V2, c: Rgb) {
        let (x, y) = self.to_dot(p);
        let c = self.col(c);
        self.plot(x.floor() as i32, y.floor() as i32, c);
    }
    fn seg(&mut self, a: V2, b: V2, t0: f32, t1: f32, ink: &impl Ink) {
        let (a, b) = (self.to_dot(a), self.to_dot(b));
        let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().clamp(1.0, 20_000.0) as usize;
        for k in 0..=n {
            let s = k as f32 / n as f32;
            let c = self.col(ink.at(lerp(t0, t1, s)));
            self.plot(lerp(a.0, b.0, s).floor() as i32, lerp(a.1, b.1, s).floor() as i32, c);
        }
    }
    pub fn line(&mut self, a: V2, b: V2, ink: impl Ink) {
        self.seg(a, b, 0.0, 1.0, &ink);
    }
    pub fn stroke(&mut self, p: &Path, ink: impl Ink) {
        let total = p.len();
        let mut acc = 0.0;
        for (a, b) in p.segs() {
            let l = dist(a, b);
            self.seg(a, b, acc / total.max(1e-9), (acc + l) / total.max(1e-9), &ink);
            acc += l;
        }
    }
    /// A stroke with a soft glow underneath. The crowd favourite.
    pub fn neon(&mut self, p: &Path, ink: impl Ink) {
        self.neon_k(p, ink, 0.4);
    }
    /// [`neon`](Canvas::neon) with a chosen glow strength: 0 = plain stroke, 0.4 = default, 1 = blinding.
    pub fn neon_k(&mut self, p: &Path, ink: impl Ink, strength: f32) {
        if strength > 0.0 {
            let n = (p.len() * 5.0) as usize + 2;
            for (i, q) in p.resample(n).pts.into_iter().enumerate() {
                self.glow(q, 0.3, ink.at(i as f32 / (n - 1) as f32).dim(strength));
            }
        }
        self.stroke(p, ink);
    }
    pub fn arrow(&mut self, a: V2, b: V2, ink: impl Ink) {
        self.seg(a, b, 0.0, 1.0, &ink);
        let l = dist(a, b);
        if l < 1e-6 {
            return;
        }
        let u = ((b.0 - a.0) / l, (b.1 - a.1) / l);
        let s = (l * 0.3).min(0.28);
        for side in [-1.0, 1.0] {
            let h = rot2(u, PI - side * 0.45);
            self.seg(b, (b.0 + h.0 * s, b.1 + h.1 * s), 1.0, 1.0, &ink);
        }
    }
    /// Faint coordinate grid, like manim's NumberPlane.
    pub fn grid(&mut self, step: f32, c: Rgb) {
        let (w, h) = (self.w(), self.h());
        let mut x = (-w / step).ceil() * step;
        while x <= w {
            self.line((x, -h), (x, h), c);
            x += step;
        }
        let mut y = (-h / step).ceil() * step;
        while y <= h {
            self.line((-w, y), (w, y), c);
            y += step;
        }
    }
    /// x and y axes with unit ticks.
    pub fn axes(&mut self, c: Rgb) {
        let (w, h) = (self.w(), self.h());
        self.arrow((-w, 0.0), (w - 0.05, 0.0), c);
        self.arrow((0.0, -h), (0.0, h - 0.05), c);
        for i in -(w as i32)..=(w as i32) {
            self.line((i as f32, -0.08), (i as f32, 0.08), c);
        }
        for i in -3..=3 {
            self.line((-0.08, i as f32), (0.08, i as f32), c);
        }
    }

    // ── pixels: full color ──

    fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if x >= 0 && y >= 0 && (x as usize) < self.cols && (y as usize) < self.rows * 2 {
            let i = y as usize * self.cols + x as usize;
            self.pix[i] = self.pix[i].mix(c, a * self.alpha);
        }
    }
    fn to_pix(&self, p: V2) -> V2 {
        let (x, y) = self.to_dot(p);
        (x / 2.0, y / 2.0)
    }
    /// Run a pixel shader over the whole screen, on every core. Return `None` to leave a pixel alone.
    pub fn shade(&mut self, f: impl Fn(f32, f32) -> Option<Rgb> + Sync) {
        let (cols, pu, a) = (self.cols, self.unit / 2.0, self.alpha);
        let (cx, cy) = (cols as f32 / 2.0, self.rows as f32);
        let f = &f;
        par(&mut self.pix, cols, move |i, px| {
            let (x, y) = ((i % cols) as f32 + 0.5, (i / cols) as f32 + 0.5);
            if let Some(c) = f((x - cx) / pu, (cy - y) / pu) {
                *px = if a >= 1.0 { c } else { px.mix(c, a) };
            }
        });
    }
    /// One full-color pixel (half a terminal cell).
    pub fn px(&mut self, p: V2, c: Rgb) {
        let (x, y) = self.to_pix(p);
        self.blend(x.floor() as i32, y.floor() as i32, c, 1.0);
    }
    fn radial(&mut self, p: V2, r: f32, c: Rgb, soft: bool) {
        let (cx, cy) = self.to_pix(p);
        let rp = (r * self.unit / 2.0).max(0.5);
        for y in (cy - rp).floor() as i32..=(cy + rp).ceil() as i32 {
            for x in (cx - rp).floor() as i32..=(cx + rp).ceil() as i32 {
                let d = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy) / rp;
                if d < 1.0 {
                    self.blend(x, y, c, if soft { (1.0 - d) * (1.0 - d) } else { 1.0 });
                } else if !soft && d < 1.0 + 1.0 / rp {
                    self.blend(x, y, c, 0.5); // anti-aliased rim
                }
            }
        }
    }
    /// Solid disc.
    pub fn disc(&mut self, p: V2, r: f32, c: Rgb) {
        self.radial(p, r, c, false);
    }
    /// Soft radial light. Stack a few for bloom.
    pub fn glow(&mut self, p: V2, r: f32, c: Rgb) {
        self.radial(p, r, c, true);
    }
    /// Size of one terminal cell in world units.
    pub fn cell(&self) -> V2 {
        (2.0 / self.unit, 4.0 / self.unit)
    }
    /// A box that covers whatever was drawn under it (dots and text too). Put it behind labels.
    pub fn panel(&mut self, center: V2, w: f32, h: f32, c: Rgb, opacity: f32) {
        let (x0, y0) = self.to_dot((center.0 - w / 2.0, center.1 + h / 2.0));
        let (x1, y1) = self.to_dot((center.0 + w / 2.0, center.1 - h / 2.0));
        let (c0, c1) = ((x0 / 2.0).floor().max(0.0) as usize, ((x1 / 2.0).ceil().max(0.0) as usize).min(self.cols));
        let (r0, r1) = ((y0 / 4.0).floor().max(0.0) as usize, ((y1 / 4.0).ceil().max(0.0) as usize).min(self.rows));
        let hide = opacity * self.alpha >= 0.5;
        for r in r0..r1 {
            for col in c0..c1 {
                let i = r * self.cols + col;
                if hide {
                    self.dots[i] = 0;
                    self.txt[i] = ('\0', self.bg);
                }
                self.blend(col as i32, 2 * r as i32, c, opacity);
                self.blend(col as i32, 2 * r as i32 + 1, c, opacity);
            }
        }
    }
    /// Fill a closed path (even-odd). The ink runs top (0) to bottom (1).
    pub fn fill(&mut self, p: &Path, ink: impl Ink, opacity: f32) {
        let pts: Vec<V2> = p.pts.iter().map(|&q| self.to_pix(q)).collect();
        if pts.len() < 3 {
            return;
        }
        let y0 = pts.iter().map(|q| q.1).fold(f32::MAX, f32::min);
        let y1 = pts.iter().map(|q| q.1).fold(f32::MIN, f32::max);
        let mut xs = Vec::new();
        for py in y0.floor().max(0.0) as i32..=y1.ceil().min(self.rows as f32 * 2.0 - 1.0) as i32 {
            let yc = py as f32 + 0.5;
            xs.clear();
            for j in 0..pts.len() {
                let (a, b) = (pts[j], pts[(j + 1) % pts.len()]);
                if (a.1 <= yc) != (b.1 <= yc) {
                    xs.push(a.0 + (yc - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(|a, b| a.total_cmp(b));
            let c = ink.at((yc - y0) / (y1 - y0).max(1e-6));
            for pair in xs.chunks_exact(2) {
                for x in (pair[0] - 0.5).ceil() as i32..=(pair[1] - 0.5).floor() as i32 {
                    self.blend(x, py, c, opacity);
                }
            }
        }
    }
    /// Big chunky pixel-font text centered at `p`, `size` units tall. The ink runs left to right.
    pub fn big(&mut self, p: V2, s: &str, size: f32, ink: impl Ink) {
        let k = (size * self.unit / 10.0).round().max(1.0) as i32;
        let n = s.chars().count() as i32;
        let (w, h) = (n * 4 * k - k, 5 * k);
        let (cx, cy) = self.to_pix(p);
        let (x0, y0) = ((cx - w as f32 / 2.0).round() as i32, (cy - h as f32 / 2.0).round() as i32);
        for (i, ch) in s.chars().enumerate() {
            for (r, row) in glyph(ch).iter().enumerate() {
                for b in 0..3 {
                    if (row >> (2 - b)) & 1 == 1 {
                        let (gx, gy) = (x0 + (i as i32 * 4 + b) * k, y0 + r as i32 * k);
                        let c = ink.at((gx - x0) as f32 / w.max(1) as f32);
                        for yy in 0..k {
                            for xx in 0..k {
                                self.blend(gx + xx, gy + yy, c, 1.0);
                            }
                        }
                    }
                }
            }
        }
    }

    // ── text ──

    /// Put one character into the cell at `col`, `row` (0, 0 is the top left). Off-screen is ignored.
    pub fn set(&mut self, col: i32, row: i32, ch: char, c: Rgb) {
        if col >= 0 && row >= 0 && (col as usize) < self.cols && (row as usize) < self.rows {
            self.txt[row as usize * self.cols + col as usize] = (ch, self.col(c));
        }
    }
    /// The cell (column, row) that world point `p` falls in.
    pub fn cell_at(&self, p: V2) -> (i32, i32) {
        let (x, y) = self.to_dot(p);
        ((x / 2.0).floor() as i32, (y / 4.0).floor() as i32)
    }
    fn put(&mut self, p: V2, s: &str, upto: usize, caret: bool, left: bool, ink: &impl Ink) {
        let n = s.chars().count();
        let (x, y) = self.to_dot(p);
        let start = if left { x / 2.0 } else { x / 2.0 - n as f32 / 2.0 };
        let (cx, cy) = (start.round() as i32, (y / 4.0).floor() as i32);
        let chars = s.chars().take(upto).chain(caret.then_some('▌'));
        for (i, ch) in chars.enumerate() {
            if ch != ' ' {
                self.set(cx + i as i32, cy, ch, ink.at(i as f32 / (n.max(2) - 1) as f32));
            }
        }
    }
    /// Text centered at `p`. The ink runs across the characters.
    pub fn text(&mut self, p: V2, s: &str, ink: impl Ink) {
        self.put(p, s, usize::MAX, false, false, &ink);
    }
    /// Text starting at `p` (left aligned). Good for HUDs and lists.
    pub fn label(&mut self, p: V2, s: &str, ink: impl Ink) {
        self.put(p, s, usize::MAX, false, true, &ink);
    }
    /// Typewriter: reveal fraction `prog` of `s` with a caret. Layout stays put while it types.
    pub fn write(&mut self, p: V2, s: &str, prog: f32, ink: impl Ink) {
        let n = s.chars().count();
        let k = (prog.clamp(0.0, 1.0) * n as f32).round() as usize;
        self.put(p, s, k, k < n && prog > 0.0, false, &ink);
    }
    /// A line drawn in characters picked by its slope (`-` `/` `|` `\`), the classic ASCII-art look.
    pub fn glyph_line(&mut self, a: V2, b: V2, ink: impl Ink) {
        let ch = slope_char(b.0 - a.0, b.1 - a.1);
        let (pa, pb) = (self.to_dot(a), self.to_dot(b));
        let (ax, ay, dx, dy) = (pa.0 / 2.0, pa.1 / 4.0, (pb.0 - pa.0) / 2.0, (pb.1 - pa.1) / 4.0);
        let n = dx.abs().max(dy.abs()).ceil().clamp(1.0, 4000.0) as usize;
        let mut last = (i32::MIN, i32::MIN);
        for k in 0..=n {
            let s = k as f32 / n as f32;
            let cell = ((ax + dx * s).floor() as i32, (ay + dy * s).floor() as i32);
            if cell != last {
                self.set(cell.0, cell.1, ch, ink.at(s));
                last = cell;
            }
        }
    }
    /// Character shader: `f(x, y)` gives a brightness (0..1) and a color for each cell, or `None` to skip it.
    /// The character comes from `ramp` (dark to bright, e.g. [`RAMP_DENSE`]) and the color is shaded by
    /// brightness. Runs on every core. This is how you draw solid, lit things out of letters.
    pub fn ascii(&mut self, ramp: &str, f: impl Fn(f32, f32) -> Option<(f32, Rgb)> + Sync) {
        let ramp: Vec<char> = ramp.chars().collect();
        let (cols, u, a, bg) = (self.cols, self.unit, self.alpha, self.bg);
        let (cx, cy) = (cols as f32, self.rows as f32 * 2.0);
        let (f, ramp) = (&f, &ramp);
        par(&mut self.txt, cols, move |i, cell| {
            let (col, row) = ((i % cols) as f32, (i / cols) as f32);
            if let Some((l, c)) = f(((col + 0.5) * 2.0 - cx) / u, (cy - (row + 0.5) * 4.0) / u) {
                let l = l.clamp(0.0, 1.0);
                let ch = ramp[((l * ramp.len() as f32) as usize).min(ramp.len() - 1)];
                if ch != ' ' {
                    let c = c.dim(0.3 + 0.7 * l);
                    *cell = (ch, if a >= 1.0 { c } else { bg.mix(c, a) });
                }
            }
        });
    }
    /// A syntax-highlighted source listing with line numbers. `p` is the top-left corner; shows `rows`
    /// lines starting at line `first` (0-based), cut at `width` cells. Try it with `include_str!(file!())`.
    pub fn code(&mut self, p: V2, src: &str, first: usize, rows: usize, width: usize) {
        let (c0, r0) = self.cell_at(p);
        for (k, line) in src.lines().skip(first).take(rows).enumerate() {
            let r = r0 + k as i32;
            for (j, ch) in format!("{:>4} ", first + k + 1).chars().enumerate() {
                self.set(c0 + j as i32, r, ch, GREY.dim(0.55));
            }
            for (j, (ch, c)) in highlight(line).into_iter().take(width.saturating_sub(5)).enumerate() {
                if ch != ' ' {
                    self.set(c0 + 5 + j as i32, r, ch, c);
                }
            }
        }
    }
    /// Mirror everything above the line `y` into the area below it, like still water. `ripple` is the
    /// sideways wobble in cells (driven by time `t`), `tint` the water color, and `fade` (0..1) how much
    /// the reflection dies away with depth. Call it after drawing the scene, then draw things in the water.
    pub fn reflect(&mut self, y: f32, t: f32, ripple: f32, tint: Rgb, fade: f32) {
        let (cols, rows) = (self.cols, self.rows);
        let rw = (self.to_dot((0.0, y)).1 / 4.0).round().clamp(0.0, rows as f32) as usize;
        for r in rw..rows.min(2 * rw) {
            let (d, src) = (r - rw, 2 * rw - 1 - r);
            let k = d as f32 / (rows - rw).max(1) as f32;
            let keep = (0.7 - fade * k).clamp(0.0, 1.0) * self.alpha;
            let shift = (ripple * (r as f32 * 1.3 + t * 2.4).sin() * (0.4 + k)).round() as i32;
            for c in 0..cols {
                let sc = (c as i32 - shift).clamp(0, cols as i32 - 1) as usize;
                let (si, di) = (src * cols + sc, r * cols + c);
                let (ch, fg) = self.txt[si];
                self.txt[di] = if ch == '\0' { ('\0', self.bg) } else { (mirror_char(ch), tint.mix(fg, keep)) };
                self.dots[di] = flip_braille(self.dots[si]);
                self.dcol[di] = tint.mix(self.dcol[si], keep);
                let (top, bot) = (self.pix[2 * src * cols + sc], self.pix[(2 * src + 1) * cols + sc]);
                self.pix[2 * r * cols + c] = tint.mix(bot, keep);
                self.pix[(2 * r + 1) * cols + c] = tint.mix(top, keep);
            }
        }
    }

    // ── 3D ──

    /// Perspective projection; the camera sits at z = +12 looking at the origin.
    pub fn proj(&self, p: V3) -> V2 {
        let k = 12.0 / (12.0 - p.2).max(0.5);
        (p.0 * k, p.1 * k)
    }
    pub fn line3(&mut self, a: V3, b: V3, ink: impl Ink) {
        let (a, b) = (self.proj(a), self.proj(b));
        self.seg(a, b, 0.0, 1.0, &ink);
    }
    /// Wireframe. The ink runs across the edges.
    pub fn mesh(&mut self, verts: &[V3], edges: &[(usize, usize)], ink: impl Ink) {
        for (k, &(i, j)) in edges.iter().enumerate() {
            let c = ink.at(k as f32 / edges.len().max(1) as f32);
            self.line3(verts[i], verts[j], c);
        }
    }
    /// Lit, z-buffered ASCII surface (the famous donut, generalised). `f(u, v)` maps 0..1 × 0..1 to a point.
    /// Normals are found numerically; the ink is sampled by brightness.
    pub fn surface(&mut self, f: impl Fn(f32, f32) -> V3, ink: impl Ink) {
        let n = (self.cols + self.rows * 2).max(60);
        let light = norm3((-0.5, 0.7, 0.6));
        let e = 1e-3;
        for i in 0..n {
            let u = i as f32 / n as f32;
            for j in 0..n {
                let v = j as f32 / n as f32;
                let p = f(u, v);
                let (a, b) = (sub3(f(u + e, v), p), sub3(f(u, v + e), p));
                let mut nr = norm3((a.1 * b.2 - a.2 * b.1, a.2 * b.0 - a.0 * b.2, a.0 * b.1 - a.1 * b.0));
                if nr.2 < 0.0 {
                    nr = (-nr.0, -nr.1, -nr.2);
                }
                let (x, y) = self.to_dot(self.proj(p));
                let (cx, cy) = ((x / 2.0).floor() as i32, (y / 4.0).floor() as i32);
                if cx < 0 || cy < 0 || cx >= self.cols as i32 || cy >= self.rows as i32 {
                    continue;
                }
                let k = cy as usize * self.cols + cx as usize;
                if p.2 > self.zb[k] {
                    self.zb[k] = p.2;
                    let l = (nr.0 * light.0 + nr.1 * light.1 + nr.2 * light.2).clamp(0.0, 1.0);
                    let c = self.col(ink.at(l).dim(0.35 + 0.65 * l));
                    self.txt[k] = (RAMP[(l * (RAMP.len() as f32 - 0.01)) as usize] as char, c);
                }
            }
        }
    }

    // ── output ──

    fn compose(&self, out: &mut Vec<Cell>) {
        out.clear();
        for r in 0..self.rows {
            for c in 0..self.cols {
                let i = r * self.cols + c;
                let (t, b) = (self.pix[2 * r * self.cols + c], self.pix[(2 * r + 1) * self.cols + c]);
                let (ch, fg) = self.txt[i];
                out.push(if ch != '\0' {
                    Cell { ch, fg, bg: t.mix(b, 0.5) }
                } else if self.dots[i] != 0 {
                    Cell { ch: char::from_u32(0x2800 + self.dots[i] as u32).unwrap(), fg: self.dcol[i], bg: t.mix(b, 0.5) }
                } else if t == b {
                    Cell { ch: ' ', fg: BLACK, bg: t }
                } else {
                    Cell { ch: '▀', fg: t, bg: b }
                });
            }
        }
    }
}

/// Run `work(index, item)` over a buffer, split by rows (`row` items each) across every core.
/// Falls back to one thread where threads can't be spawned (e.g. wasm).
fn par<T: Send>(data: &mut [T], row: usize, work: impl Fn(usize, &mut T) + Sync) {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = (data.len() / row.max(1)).div_ceil(threads).max(1) * row.max(1);
    let work = &work;
    let run = move |ci: usize, part: &mut [T]| part.iter_mut().enumerate().for_each(|(k, x)| work(ci * chunk + k, x));
    std::thread::scope(|s| {
        for (ci, part) in data.chunks_mut(chunk).enumerate() {
            if threads == 1 {
                run(ci, part);
            } else {
                std::thread::Builder::new().spawn_scoped(s, move || run(ci, part)).expect("spawn");
            }
        }
    });
}

/// Character ramps for [`Canvas::ascii`], dark to bright.
pub const RAMP_SOFT: &str = " .:-=+*#%@";
pub const RAMP_DENSE: &str = " .'`^\",:;Il!i~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";
pub const RAMP_BLOCK: &str = " ░▒▓█";
pub const RAMP_DIGITS: &str = " .:-o0O8$@";

/// The ASCII character that best draws a line going in direction (dx, dy).
pub fn slope_char(dx: f32, dy: f32) -> char {
    match dy.atan2(dx).to_degrees().rem_euclid(180.0) as u32 {
        0..=24 | 156..=180 => '-',
        25..=69 => '/',
        70..=110 => '|',
        _ => '\\',
    }
}

fn mirror_char(c: char) -> char {
    match c {
        '/' => '\\',
        '\\' => '/',
        '^' => 'v',
        'v' => '^',
        '▀' => '▄',
        '▄' => '▀',
        _ => c,
    }
}

fn flip_braille(m: u8) -> u8 {
    let mut o = 0;
    for (x, col) in BRAILLE.iter().enumerate() {
        for y in 0..4 {
            if m & col[y] != 0 {
                o |= BRAILLE[x][3 - y];
            }
        }
    }
    o
}

/// Tiny Rust-ish highlighter for [`Canvas::code`].
fn highlight(line: &str) -> Vec<(char, Rgb)> {
    const KW: &[&str] = &[
        "fn", "let", "mut", "for", "in", "if", "else", "match", "return", "use", "pub", "const", "move",
        "struct", "impl", "enum", "while", "loop", "as", "self", "Self", "true", "false", "def", "import",
    ];
    let (cs, mut out, mut i) = (line.chars().collect::<Vec<_>>(), Vec::new(), 0);
    let span = |out: &mut Vec<(char, Rgb)>, s: &[char], c: Rgb| out.extend(s.iter().map(|&ch| (ch, c)));
    while i < cs.len() {
        let c = cs[i];
        let mut j = i + 1;
        if (c == '/' && cs.get(i + 1) == Some(&'/')) || c == '#' {
            span(&mut out, &cs[i..], GREY);
            break;
        } else if c == '"' {
            while j < cs.len() && cs[j] != '"' {
                j += 1;
            }
            j = (j + 1).min(cs.len());
            span(&mut out, &cs[i..j], GREEN);
        } else if c.is_ascii_digit() {
            while j < cs.len() && (cs[j].is_ascii_alphanumeric() || cs[j] == '.' || cs[j] == '_') {
                j += 1;
            }
            span(&mut out, &cs[i..j], GOLD);
        } else if c.is_alphabetic() || c == '_' {
            while j < cs.len() && (cs[j].is_alphanumeric() || cs[j] == '_') {
                j += 1;
            }
            let w: String = cs[i..j].iter().collect();
            let col = if KW.contains(&w.as_str()) {
                PINK
            } else if cs.get(j) == Some(&'(') || cs.get(j) == Some(&'!') {
                SKY
            } else if c.is_uppercase() {
                TEAL
            } else {
                Rgb::hex(0xC9D1E6)
            };
            span(&mut out, &cs[i..j], col);
        } else {
            out.push((c, Rgb::hex(0x8A93B0)));
        }
        i = j;
    }
    out
}

/// 3×5 pixel font, rows top to bottom, 3 bits each.
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        'A' => [2, 5, 7, 5, 5], 'B' => [6, 5, 6, 5, 6], 'C' => [3, 4, 4, 4, 3], 'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7], 'F' => [7, 4, 6, 4, 4], 'G' => [3, 4, 5, 5, 3], 'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7], 'J' => [1, 1, 1, 5, 2], 'K' => [5, 5, 6, 5, 5], 'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5], 'N' => [6, 5, 5, 5, 5], 'O' => [2, 5, 5, 5, 2], 'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 6, 3], 'R' => [6, 5, 6, 5, 5], 'S' => [3, 4, 2, 1, 6], 'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7], 'V' => [5, 5, 5, 5, 2], 'W' => [5, 5, 7, 7, 5], 'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2], 'Z' => [7, 1, 2, 4, 7],
        '0' => [7, 5, 5, 5, 7], '1' => [2, 6, 2, 2, 7], '2' => [6, 1, 2, 4, 7], '3' => [6, 1, 2, 1, 6],
        '4' => [5, 5, 7, 1, 1], '5' => [7, 4, 6, 1, 6], '6' => [3, 4, 7, 5, 7], '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7], '9' => [7, 5, 7, 1, 6],
        '!' => [2, 2, 2, 0, 2], '?' => [6, 1, 2, 0, 2], '.' => [0, 0, 0, 0, 2], ',' => [0, 0, 0, 2, 4],
        ':' => [0, 2, 0, 2, 0], '-' => [0, 0, 7, 0, 0], '+' => [0, 2, 7, 2, 0], '=' => [0, 7, 0, 7, 0],
        '\'' => [2, 2, 0, 0, 0], '/' => [1, 1, 2, 4, 4], '(' => [2, 4, 4, 4, 2], ')' => [2, 1, 1, 1, 2],
        '*' => [0, 5, 2, 5, 0], '#' => [5, 7, 5, 7, 5], '<' => [1, 2, 4, 2, 1], '>' => [4, 2, 1, 2, 4],
        '_' => [0, 0, 0, 0, 7], '^' => [2, 5, 0, 0, 0],
        _ => [0; 5],
    }
}

// ───────────────────────────── scene ─────────────────────────────

/// Timing handed to every draw function.
#[derive(Clone, Copy, Debug)]
pub struct Tick {
    /// Eased progress through the clip, 0..1 (1 while holding).
    pub p: f32,
    /// Raw linear progress, 0..1.
    pub raw: f32,
    /// Seconds since this clip started (keeps counting while held).
    pub t: f32,
    /// Seconds since the scene started.
    pub time: f32,
    /// Length of the clip in seconds.
    pub dur: f32,
}

type Draw = Box<dyn Fn(&mut Canvas, Tick)>;

/// One entry on the timeline. Returned by [`Scene::play`] and friends for chaining.
pub struct Clip {
    start: f32,
    dur: f32,
    end: f32,
    ease: fn(f32) -> f32,
    fin: f32,
    fout: f32,
    draw: Draw,
}

impl Clip {
    pub fn ease(&mut self, e: fn(f32) -> f32) -> &mut Clip {
        self.ease = e;
        self
    }
    /// Keep drawing (at p = 1) until the scene ends.
    pub fn hold(&mut self) -> &mut Clip {
        self.end = f32::INFINITY;
        self
    }
    /// Keep drawing until absolute time `t`.
    pub fn until(&mut self, t: f32) -> &mut Clip {
        self.end = t;
        self
    }
    /// Keep drawing for `d` more seconds after the clip ends.
    pub fn linger(&mut self, d: f32) -> &mut Clip {
        self.end = self.start + self.dur + d;
        self
    }
    pub fn fade_in(&mut self, d: f32) -> &mut Clip {
        self.fin = d;
        self
    }
    /// Fade out over the last `d` seconds before the clip disappears.
    pub fn fade_out(&mut self, d: f32) -> &mut Clip {
        self.fout = d;
        self
    }
}

/// A timeline of draw functions.
pub struct Scene {
    clips: Vec<Clip>,
    cursor: f32,
    last: f32,
    pub fps: f32,
    pub bg: Rgb,
    /// Loop forever in the terminal (Ctrl-C to quit).
    pub looped: bool,
    /// Columns × rows for HTML export. (160, 45) is 16:9; (90, 80) is a 9:16 portrait for phones.
    pub size: (usize, usize),
}

impl Default for Scene {
    fn default() -> Self {
        Scene::new()
    }
}

impl Scene {
    pub fn new() -> Scene {
        Scene { clips: Vec::new(), cursor: 0.0, last: 0.0, fps: 60.0, bg: BG, looped: false, size: (160, 45) }
    }
    fn push(&mut self, start: f32, dur: f32, f: impl Fn(&mut Canvas, Tick) + 'static) -> &mut Clip {
        self.last = start;
        let end = start + dur;
        self.clips.push(Clip { start, dur, end, ease: ease::smooth, fin: 0.0, fout: 0.0, draw: Box::new(f) });
        self.clips.last_mut().unwrap()
    }
    /// Play after everything before it.
    pub fn play(&mut self, dur: f32, f: impl Fn(&mut Canvas, Tick) + 'static) -> &mut Clip {
        let s = self.cursor;
        self.cursor += dur;
        self.push(s, dur, f)
    }
    /// Play at the same moment as the previous clip.
    pub fn also(&mut self, dur: f32, f: impl Fn(&mut Canvas, Tick) + 'static) -> &mut Clip {
        let s = self.last;
        self.cursor = self.cursor.max(s + dur);
        self.push(s, dur, f)
    }
    /// Play at an absolute time, without moving the cursor.
    pub fn at(&mut self, start: f32, dur: f32, f: impl Fn(&mut Canvas, Tick) + 'static) -> &mut Clip {
        self.push(start, dur, f)
    }
    /// Draw for the whole scene (backgrounds, HUDs). `tick.t` is the scene time.
    pub fn always(&mut self, f: impl Fn(&mut Canvas, Tick) + 'static) -> &mut Clip {
        self.push(0.0, 0.0, f).hold()
    }
    pub fn wait(&mut self, d: f32) {
        self.cursor += d;
    }
    /// Where the next `play` starts.
    pub fn now(&self) -> f32 {
        self.cursor
    }
    pub fn duration(&self) -> f32 {
        self.clips.iter().map(|c| if c.end.is_finite() { c.end.max(c.start + c.dur) } else { 0.0 }).fold(self.cursor, f32::max)
    }
    /// Render the frame at `time` into `c`.
    pub fn frame(&self, c: &mut Canvas, time: f32) {
        c.clear();
        for k in &self.clips {
            let end = k.end.max(k.start + k.dur);
            if time < k.start || time >= end {
                continue;
            }
            let raw = if k.dur > 0.0 { ((time - k.start) / k.dur).clamp(0.0, 1.0) } else { 1.0 };
            let mut a: f32 = 1.0;
            if k.fin > 0.0 {
                a = a.min((time - k.start) / k.fin);
            }
            if k.fout > 0.0 && end.is_finite() {
                a = a.min((end - time) / k.fout);
            }
            c.alpha = a.clamp(0.0, 1.0);
            (k.draw)(c, Tick { p: (k.ease)(raw), raw, t: time - k.start, time, dur: k.dur });
        }
        c.alpha = 1.0;
    }

    /// `--html [file]` on the command line bakes an HTML player (`--size 90x80` picks its columns and
    /// rows), otherwise the scene plays in the terminal.
    pub fn show(&self) {
        let args: Vec<String> = std::env::args().collect();
        let arg = |flag: &str| args.iter().position(|a| a == flag).map(|i| args.get(i + 1).cloned().unwrap_or_default());
        let size = arg("--size").and_then(|s| {
            let (c, r) = s.split_once('x')?;
            Some((c.parse().ok()?, r.parse().ok()?))
        });
        match arg("--html") {
            Some(path) => {
                let path = if path.is_empty() || path.starts_with("--") { "maniac.html".to_string() } else { path };
                let (cols, rows) = size.unwrap_or(self.size);
                self.html(&path, cols, rows).expect("could not write html");
                eprintln!("wrote {path}");
            }
            None => self.run(),
        }
    }

    /// Play in the terminal: 24-bit color, alternate screen, only changed cells are redrawn.
    pub fn run(&self) {
        let total = self.duration().max(1e-3);
        let mut out = std::io::stdout().lock();
        sys::trap();
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |i| {
            sys::restore();
            prev_hook(i)
        }));
        let _ = out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[?7l\x1b[2J");
        let (mut cv, mut cur, mut prev) = (Canvas::new(0, 0, self.bg), Vec::new(), Vec::new());
        let (t0, dt) = (Instant::now(), 1.0 / self.fps);
        let mut buf = Vec::with_capacity(1 << 20);
        let mut frame = 0u64;
        loop {
            let (cols, rows) = sys::size();
            if (cols, rows) != (cv.cols, cv.rows) {
                cv = Canvas::new(cols, rows, self.bg);
                prev.clear();
                buf.extend_from_slice(b"\x1b[2J");
            }
            let el = t0.elapsed().as_secs_f32();
            let done = !self.looped && el >= total;
            let t = if self.looped { el % total } else { el.min(total - 1e-4) };
            self.frame(&mut cv, t);
            cv.compose(&mut cur);
            emit(&cur, &prev, cols, &mut buf);
            let _ = out.write_all(&buf).and_then(|_| out.flush());
            buf.clear();
            std::mem::swap(&mut cur, &mut prev);
            if done {
                break;
            }
            frame += 1;
            let next = t0 + Duration::from_secs_f32(frame as f32 * dt);
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else {
                frame = ((now - t0).as_secs_f32() / dt) as u64; // fell behind: drop frames, keep time
            }
        }
        std::thread::sleep(Duration::from_millis(900));
        sys::restore();
    }

    /// Bake every frame into one self-contained HTML player (JetBrains Mono, bloom, seek, pause).
    /// Frames are stored as binary deltas, so only what changed costs bytes.
    pub fn html(&self, path: &str, cols: usize, rows: usize) -> std::io::Result<()> {
        let fps = self.fps.min(30.0);
        let total = self.duration().max(1e-3);
        let n = (total * fps).ceil() as usize;
        let mut cv = Canvas::new(cols, rows, self.bg);
        let (mut cur, mut prev) = (Vec::new(), Vec::<Cell>::new());
        let (mut blob, mut offs) = (Vec::new(), Vec::with_capacity(n));
        for f in 0..n {
            self.frame(&mut cv, (f as f32 / fps).min(total - 1e-4));
            cv.compose(&mut cur);
            offs.push(blob.len());
            let (mut pos, mut ch, mut fg, mut bg) = (0, ' ', BLACK, BLACK);
            for (i, c) in cur.iter().enumerate() {
                if prev.get(i) == Some(c) {
                    continue;
                }
                let skip = i - pos;
                let flags = (skip > 0) as u8 | ((c.ch != ch) as u8) << 1 | ((c.fg != fg) as u8) << 2 | ((c.bg != bg) as u8) << 3;
                blob.push(flags);
                if skip > 0 {
                    varint(&mut blob, skip as u32);
                }
                if c.ch != ch {
                    varint(&mut blob, c.ch as u32);
                }
                if c.fg != fg {
                    blob.extend([c.fg.0, c.fg.1, c.fg.2]);
                }
                if c.bg != bg {
                    blob.extend([c.bg.0, c.bg.1, c.bg.2]);
                }
                (pos, ch, fg, bg) = (i + 1, c.ch, c.fg, c.bg);
            }
            std::mem::swap(&mut cur, &mut prev);
        }
        let offs: Vec<String> = offs.iter().map(|o| o.to_string()).collect();
        let meta = format!(r#"{{"cols":{cols},"rows":{rows},"fps":{fps},"n":{n},"offs":[{}]}}"#, offs.join(","));
        let bg = format!("#{:02x}{:02x}{:02x}", self.bg.0, self.bg.1, self.bg.2);
        let page = PLAYER.replace("__META__", &meta).replace("__BG__", &bg).replace("__DATA__", &base64(&blob));
        std::fs::write(path, page)
    }
}

fn varint(o: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        o.push(v as u8 | 0x80);
        v >>= 7;
    }
    o.push(v as u8);
}

fn base64(d: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(d.len().div_ceil(3) * 4);
    for c in d.chunks(3) {
        let b = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            s.push(if k <= c.len() { T[(b >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

// ───────────────────────────── terminal ─────────────────────────────

fn num(o: &mut Vec<u8>, n: usize) {
    if n >= 10 {
        num(o, n / 10);
    }
    o.push(b'0' + (n % 10) as u8);
}

fn sgr(o: &mut Vec<u8>, kind: &[u8], c: Rgb) {
    o.extend_from_slice(kind);
    for (k, v) in [c.0, c.1, c.2].into_iter().enumerate() {
        num(o, v as usize);
        o.push(if k < 2 { b';' } else { b'm' });
    }
}

/// Write only the cells that changed, tracking cursor and colors to skip redundant escapes.
fn emit(cur: &[Cell], prev: &[Cell], cols: usize, o: &mut Vec<u8>) {
    let (mut fg, mut bg, mut at) = (None, None, usize::MAX);
    for (i, c) in cur.iter().enumerate() {
        if prev.get(i) == Some(c) {
            continue;
        }
        if at != i {
            o.extend_from_slice(b"\x1b[");
            num(o, i / cols + 1);
            o.push(b';');
            num(o, i % cols + 1);
            o.push(b'H');
        }
        if c.ch != ' ' && fg != Some(c.fg) {
            sgr(o, b"\x1b[38;2;", c.fg);
            fg = Some(c.fg);
        }
        if bg != Some(c.bg) {
            sgr(o, b"\x1b[48;2;", c.bg);
            bg = Some(c.bg);
        }
        o.extend_from_slice(c.ch.encode_utf8(&mut [0; 4]).as_bytes());
        at = if (i + 1) % cols == 0 { usize::MAX } else { i + 1 };
    }
}

#[cfg(unix)]
mod sys {
    use std::os::raw::{c_int, c_ulong};
    extern "C" {
        fn signal(sig: c_int, handler: extern "C" fn(c_int)) -> usize;
        fn ioctl(fd: c_int, req: c_ulong, ...) -> c_int;
        fn write(fd: c_int, buf: *const std::ffi::c_void, n: usize) -> isize;
        fn _exit(code: c_int) -> !;
    }
    const RESTORE: &[u8] = b"\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l";
    #[cfg(target_os = "linux")]
    const TIOCGWINSZ: c_ulong = 0x5413;
    #[cfg(not(target_os = "linux"))]
    const TIOCGWINSZ: c_ulong = 0x4008_7468;

    pub fn restore() {
        unsafe { write(1, RESTORE.as_ptr().cast(), RESTORE.len()) };
    }
    extern "C" fn on_signal(_: c_int) {
        restore();
        unsafe { _exit(130) }
    }
    pub fn trap() {
        unsafe {
            signal(2, on_signal);
            signal(15, on_signal);
        }
    }
    pub fn size() -> (usize, usize) {
        let mut ws = [0u16; 4];
        let ok = unsafe { ioctl(1, TIOCGWINSZ, ws.as_mut_ptr()) } == 0 && ws[0] > 0 && ws[1] > 0;
        if ok { (ws[1] as usize, ws[0] as usize) } else { (100, 30) }
    }
}

#[cfg(not(unix))]
mod sys {
    pub fn restore() {
        print!("\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l");
    }
    pub fn trap() {}
    pub fn size() -> (usize, usize) {
        (120, 36)
    }
}

const PLAYER: &str = include_str!("player.html");
