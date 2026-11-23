//! Linear transformations, told with a moving grid.
//! cargo run --release --example matrix
//! cargo run --release --example matrix -- --html matrix.html
use maniac::*;

/// A 2×2 matrix [a, b, c, d] = | a b |
///                              | c d |
/// Column 1 (a, c) is where î lands, column 2 (b, d) is where ĵ lands.
type M = [f32; 4];
const I: M = [1.0, 0.0, 0.0, 1.0];
const SHEAR: M = [1.0, 1.0, 0.0, 1.0];
const ROT90: M = [0.0, -1.0, 1.0, 0.0];
const STRETCH: M = [2.0, 1.0, 0.0, 1.5];
const FLIP: M = [1.0, 2.0, 1.5, 1.0];
const SQUASH: M = [1.0, 2.0, 0.5, 1.0];
const EIGEN: M = [3.0, 1.0, 0.0, 2.0];

const fn mul(a: M, b: M) -> M {
    [a[0] * b[0] + a[1] * b[2], a[0] * b[1] + a[1] * b[3], a[2] * b[0] + a[3] * b[2], a[2] * b[1] + a[3] * b[3]]
}
/// World units per grid step: zoom the plane a bit so the vectors read well.
const Z: f32 = 1.3;

/// Apply the matrix to a plane point and place it on screen.
fn ap(m: M, (x, y): V2) -> V2 {
    ((m[0] * x + m[1] * y) * Z, (m[2] * x + m[3] * y) * Z)
}
fn det(m: M) -> f32 {
    m[0] * m[3] - m[1] * m[2]
}
fn rot(a: f32) -> M {
    let (s, c) = a.sin_cos();
    [c, -s, s, c]
}
fn mix(a: M, b: M, t: f32) -> M {
    [0, 1, 2, 3].map(|i| lerp(a[i], b[i], t))
}

enum Kind {
    To(M),
    Spin(f32),
    Wobble,
}
/// (start, duration, move)
const MOVES: [(f32, f32, Kind); 12] = [
    (10.0, 2.0, Kind::To(SHEAR)),
    (15.5, 1.2, Kind::To(I)),
    (17.0, 2.2, Kind::Spin(PI / 2.0)),
    (21.5, 2.2, Kind::To(mul(SHEAR, ROT90))),
    (26.0, 1.2, Kind::To(I)),
    (29.0, 2.0, Kind::To(STRETCH)),
    (33.0, 2.5, Kind::To(FLIP)),
    (37.5, 2.0, Kind::To(SQUASH)),
    (41.5, 1.2, Kind::To(I)),
    (45.0, 2.5, Kind::To(EIGEN)),
    (51.5, 1.2, Kind::To(I)),
    (53.5, 1.5, Kind::Wobble),
];

fn wobble(t: f32) -> M {
    mul(rot(0.5 * (t * 0.7).sin()), [1.0 + 0.35 * (t * 1.3).sin(), 0.6 * (t * 0.9).sin(), 0.4 * (t * 1.1).cos() - 0.4, 1.0 + 0.3 * (t * 1.7).cos()])
}

/// The matrix applied to the plane at time `t`.
fn m_at(t: f32) -> M {
    let mut cur = I;
    for (at, dur, kind) in &MOVES {
        if t < *at {
            break;
        }
        let p = ease::smooth(((t - at) / dur).min(1.0));
        let (now, done) = match kind {
            Kind::To(m) => (mix(cur, *m, p), *m),
            Kind::Spin(a) => (mul(rot(a * p), cur), mul(rot(*a), cur)),
            Kind::Wobble => return mix(cur, wobble(t), p),
        };
        if p < 1.0 {
            return now;
        }
        cur = done;
    }
    cur
}

const IHAT: Rgb = Rgb::hex(0x8FD16F);
const JHAT: Rgb = Rgb::hex(0xE86A6A);

/// A vector: a soft halo, a two-dot-wide shaft and an arrowhead.
fn vector(c: &mut Canvas, from: V2, to: V2, col: Rgb) {
    c.neon_k(&Path::line(from, to), col, 0.14);
    let n = (0.03, 0.03);
    c.line((from.0 + n.0, from.1 - n.1), (to.0 + n.0, to.1 - n.1), col.dim(0.8));
    c.arrow(from, to, col);
}

/// The transformed grid, the two basis vectors and the matrix readout.
fn plane(c: &mut Canvas, m: M) {
    for i in -12..=12 {
        let f = i as f32;
        let (v, h) = if i == 0 {
            (WHITE.dim(0.8), WHITE.dim(0.8))
        } else {
            (hsv(0.55 + f * 0.012, 0.55, 0.75), hsv(0.62 - f * 0.012, 0.55, 0.75))
        };
        c.line(ap(m, (f, -12.0)), ap(m, (f, 12.0)), v);
        c.line(ap(m, (-12.0, f)), ap(m, (12.0, f)), h);
    }
    let (i, j) = (ap(m, (1.0, 0.0)), ap(m, (0.0, 1.0)));
    vector(c, (0.0, 0.0), i, IHAT);
    vector(c, (0.0, 0.0), j, JHAT);
    c.text((i.0 * 1.12 + 0.25, i.1 * 1.12 - 0.25), "î", IHAT);
    c.text((j.0 * 1.12 - 0.25, j.1 * 1.12 + 0.25), "ĵ", JHAT);

    // matrix readout, top left
    let (cw, ch) = c.cell();
    let pos = (-c.w() + 13.0 * cw, 4.0 - 4.0 * ch);
    c.panel(pos, 24.0 * cw, 7.0 * ch, BG, 0.9);
    let rows = [
        "┌              ┐".to_string(),
        format!("│ {:5.2}  {:5.2} │", m[0], m[1]),
        format!("│ {:5.2}  {:5.2} │", m[2], m[3]),
        "└              ┘".to_string(),
    ];
    for (k, row) in rows.iter().enumerate() {
        let n = row.chars().count() as f32 - 1.0;
        let ink = |u: f32| match (u * n).round() as usize {
            2..=6 => IHAT,
            9..=13 => JHAT,
            _ => GREY,
        };
        c.text((pos.0 + 2.0 * cw, pos.1 + (1.5 - k as f32) * ch), row, ink);
    }
    c.text((pos.0 - 8.5 * cw, pos.1), "A =", WHITE);
    let d = det(m);
    let col = if d < -0.01 { PINK } else if d < 0.01 { GREY } else { GOLD };
    c.text((pos.0 + 2.0 * cw, pos.1 - 3.0 * ch), &format!("det = {d:+.2}"), col);
}

fn main() {
    let mut s = Scene::new();

    // faint ghost of the original grid, so you can always see what moved
    s.always(|c, _| c.grid(Z, BLUE.dim(0.13)));

    // ── title ──
    s.at(0.0, 5.0, |c, k| {
        let p = ease::back((k.t / 1.0).min(1.0));
        if p > 0.05 {
            c.big((0.12, 1.38), "MATRIX", 2.4 * p, BLACK);
            c.big((0.0, 1.5), "MATRIX", 2.4 * p, |u| hsv(0.25 + u * 0.7 + k.time * 0.15, 0.7, 1.0));
        }
        let q = ease::back(((k.t - 0.5) / 1.0).clamp(0.0, 1.0));
        if q > 0.05 {
            c.big((0.08, -0.82), "TRANSFORMATIONS", 1.0 * q, BLACK);
            c.big((0.0, -0.75), "TRANSFORMATIONS", 1.0 * q, NEON);
        }
        c.write((0.0, -2.4), "a matrix is just a note of where î and ĵ land", (k.t - 1.6) / 1.6, WHITE);
        c.write((0.0, -3.2), "made with maniac", (k.t - 3.0) / 0.8, GREY);
    })
    .ease(ease::linear)
    .fade_out(0.6);

    // ── the plane itself, alive from 5s to the end ──
    s.at(5.0, 57.0, |c, k| plane(c, m_at(k.time))).fade_in(1.0).fade_out(1.0);

    // ── v = 2î + 1ĵ rides along with the grid ──
    s.at(6.5, 9.0, |c, k| {
        let m = m_at(k.time);
        let (i2, v) = (ap(m, (2.0, 0.0)), ap(m, (2.0, 1.0)));
        let grow = ease::smooth((k.t / 1.0).min(1.0));
        c.line((0.0, 0.0), lerp2((0.0, 0.0), i2, grow), IHAT.dim(0.7));
        c.line(i2, lerp2(i2, v, grow), JHAT.dim(0.7));
        vector(c, (0.0, 0.0), lerp2((0.0, 0.0), v, grow), GOLD);
        c.text((v.0 + 0.6, v.1 + 0.35), if k.time < 10.0 { "v = 2î + 1ĵ" } else { "Av = 2(Aî) + 1(Aĵ)" }, GOLD);
    })
    .fade_in(0.3)
    .fade_out(0.5);

    // ── determinant: the unit square's area ──
    s.at(27.5, 14.0, |c, k| {
        let m = m_at(k.time);
        let sq = Path::new(vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], true).map(|p| ap(m, p));
        let d = det(m);
        let ink = if d < 0.0 { [PINK, MAGENTA] } else { [GOLD, ORANGE] };
        c.fill(&sq, ink, 0.3);
        c.stroke(&sq, ink);
        let mid = ap(m, (0.5, 0.5));
        c.text(mid, &format!("area {:.2}", d.abs()), WHITE);
    })
    .fade_in(0.5)
    .fade_out(0.5);

    // ── eigenvectors: a fan of arrows, two of which never leave their line ──
    s.at(43.0, 9.5, |c, k| {
        let m = m_at(k.time);
        let show = ((k.t - 0.5) / 1.0).clamp(0.0, 1.0);
        c.neon_k(&Path::line(ap(I, (-9.0, 0.0)), ap(I, (9.0, 0.0))), GOLD.dim(0.6 * show), 0.2);
        c.neon_k(&Path::line(ap(I, (-6.0, 6.0)), ap(I, (6.0, -6.0))), TEAL.dim(0.6 * show), 0.2);
        for n in 0..16 {
            let a = TAU * n as f32 / 16.0;
            let tip = ap(m, (a.cos() * 1.2, a.sin() * 1.2));
            let col = match n {
                0 | 8 => GOLD,
                6 | 14 => TEAL,
                _ => rainbow(n as f32 / 16.0).dim(0.75),
            };
            c.arrow((0.0, 0.0), tip, col);
        }
        if k.time > 47.5 {
            c.text(ap(m, (1.35, 0.35)), "λ = 3", GOLD);
            let d = ap(m, (-0.9, 0.9));
            c.text((d.0 + 1.1, d.1 + 0.3), "λ = 2", TEAL);
        }
    })
    .fade_in(0.5)
    .fade_out(0.5);

    // ── finale: any shape rides along ──
    s.at(53.5, 8.0, |c, k| {
        let m = m_at(k.time);
        let heart = Path::heart(1.2).at(-2.2, 1.2).map(|p| ap(m, p));
        c.fill(&heart, [PINK, RED], 0.25);
        c.neon_k(&heart, |u| hsv(0.95 + u * 0.12, 0.6, 0.95), 0.2);
        let star = Path::star(5, 0.9, 0.4).at(2.3, -1.2).map(|p| ap(m, p));
        c.fill(&star, [GOLD, ORANGE], 0.25);
        c.neon_k(&star, SUNSET, 0.2);
    })
    .fade_in(0.8)
    .fade_out(1.0);

    // ── captions ──
    let captions: [(f32, f32, &str, [Rgb; 3]); 13] = [
        (5.5, 4.4, "this is the plane: every point is some mix of î (green) and ĵ (red)", OCEAN),
        (10.0, 5.4, "shear: î stays put, ĵ leans over to (1, 1)", AURORA),
        (15.5, 1.4, "back to normal", [GREY, GREY, GREY]),
        (17.0, 4.4, "rotate 90°: î goes to (0, 1), ĵ goes to (-1, 0)", NEON),
        (21.5, 4.4, "now shear the rotated plane: two moves = one matrix, S·R", SUNSET),
        (26.0, 3.0, "grid lines stay parallel and evenly spaced. that's what 'linear' means", OCEAN),
        (29.0, 4.0, "the determinant says how much area grows. here every area is ×3", [GOLD, ORANGE, GOLD]),
        (33.0, 4.5, "negative determinant: the plane flips over, î and ĵ swap sides", [PINK, MAGENTA, PINK]),
        (37.5, 4.0, "determinant 0: the whole plane gets squashed onto one line", [GREY, WHITE, GREY]),
        (41.5, 3.5, "most vectors get knocked off their line when you transform...", AURORA),
        (45.0, 6.5, "...but eigenvectors only stretch. along x by 3, along the diagonal by 2", [GOLD, WHITE, TEAL]),
        (51.5, 2.0, "back to normal", [GREY, GREY, GREY]),
        (53.5, 8.0, "any shape drawn on the plane just rides along with it", NEON),
    ];
    for (at, dur, text, ink) in captions {
        s.at(at, dur, move |c, k| {
            let (cw, ch) = c.cell();
            let n = text.chars().count() as f32;
            c.panel((0.0, -3.55), (n + 4.0) * cw, 1.2 * ch, BG, 0.9);
            c.write((0.0, -3.55), text, k.t / 1.2, ink);
        })
        .fade_in(0.2)
        .fade_out(0.4);
    }

    s.show();
}
