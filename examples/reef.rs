//! A clownfish weaving through an anemone, drawn entirely in characters,
//! with this very file scrolling underneath. Portrait 9:16.
//!
//! cargo run --release --example reef -- --html reef.html
use maniac::*;

const SRC: &str = include_str!("reef.rs");
const L: f32 = 2.4; // fish length in world units
const FISH_RAMP: &str = "o0O88$$"; // solid characters only: color does the shading
const ORANGE_HI: Rgb = Rgb::hex(0xFF8A1E);
const ORANGE_LO: Rgb = Rgb::hex(0xC8400E);
const STRIPE: Rgb = Rgb::hex(0xF4F1EA);
const EDGE: Rgb = Rgb::hex(0x4A2E22);
const ARM_BASE: Rgb = Rgb::hex(0x4C666B);
const ARM_TIP: Rgb = Rgb::hex(0xD6ECE8);
const ARM_DOT: Rgb = Rgb::hex(0xFF8FB0);
const CODE_TOP: f32 = -1.3;

// ── swimmer ──

/// Where the fish is, which way it faces (-1 left .. 1 right, 0 = looking at us), its pitch,
/// tail-beat frequency and speed.
#[derive(Clone, Copy)]
struct Fish {
    c: V2,
    face: f32,
    pitch: f32,
    hz: f32,
    speed: f32,
}

fn fish_at(t: f32) -> Fish {
    let pos = |t: f32| (0.8 * (t * 0.31).sin() + 0.12 * (t * 0.9).sin(), 0.95 + 0.55 * (t * 0.47 + 1.0).sin());
    let (a, b) = (pos(t - 0.01), pos(t + 0.01));
    let v = ((b.0 - a.0) / 0.02, (b.1 - a.1) / 0.02);
    let speed = v.0.hypot(v.1);
    Fish {
        c: pos(t),
        face: (v.0 / 0.1).tanh(),
        pitch: (v.1 / v.0.abs().max(0.2)).atan().clamp(-0.5, 0.5),
        hz: 0.9 + speed * 1.6,
        speed,
    }
}

/// Body half-height at `s` (0 = nose, 1 = where the tail starts).
fn half_height(s: f32) -> f32 {
    0.2 * L * (PI * s.clamp(0.0, 1.0)).sin().powf(0.65) * (1.0 - 0.35 * s) + 0.025 * L * s
}

/// The fish as a character shader: brightness and color at world point (x, y), or None outside it.
fn fish_px(f: Fish, t: f32, x: f32, y: f32) -> Option<(f32, Rgb)> {
    let sgn = if f.face < 0.0 { -1.0 } else { 1.0 };
    let (dx, dy) = rot2((x - f.c.0, y - f.c.1), -f.pitch * sgn);
    let u = dx * sgn / f.face.abs().max(0.3); // squash along the body as it turns toward us
    let s = 0.5 - u / L;
    if !(-0.02..1.35).contains(&s) || dy.abs() > 0.9 {
        return None;
    }
    let phase = TAU * f.hz * t;
    let v = dy - 0.09 * s * s * (phase - s * 3.0).sin(); // the body bends in a travelling wave

    // pectoral fin: a small paddle behind the gills, beating like an oar
    let (pu, pv) = rot2(((0.5 - 0.3) * L - u, -0.03 - v), 0.7 + 0.5 * (phase * 1.4).sin());
    if (pu / (0.11 * L)).powi(2) + (pv / (0.04 * L)).powi(2) < 1.0 {
        return Some((0.55, ORANGE_HI.mix(EDGE, 0.25)));
    }

    let hh = half_height(s);
    if s <= 1.0 && v.abs() < hh {
        let nv = v / hh;
        let nz = (1.0 - nv * nv).sqrt();
        let light = (0.45 + 0.55 * (0.7 * nz + 0.3 * nv)).clamp(0.3, 1.0);
        let eye = ((s - 0.1) * L).hypot(v - 0.035 * L);
        if eye < 0.045 * L {
            return Some(if eye < 0.014 * L { (1.0, STRIPE) } else { (0.5, EDGE) });
        }
        let se = s - 0.05 * (1.0 - nv * nv); // curve the bands around the body
        let mut col = ORANGE_LO.mix(ORANGE_HI, light);
        for (center, half) in [(0.2, 0.05), (0.52, 0.06), (0.92, 0.035)] {
            let d = (se - center).abs();
            if d < half {
                col = STRIPE;
            } else if d < half + 0.02 {
                col = EDGE;
            }
        }
        if nv.abs() > 0.92 {
            col = EDGE.mix(col, 0.45);
        }
        return Some((light, col));
    }
    // dorsal fin along the back
    if (0.22..0.78).contains(&s) && v > 0.0 && v < hh + 0.075 * L * (PI * (s - 0.22) / 0.56).sin() {
        let edge = v > hh + 0.05 * L * (PI * (s - 0.22) / 0.56).sin();
        return Some((0.55, if edge { EDGE } else { ORANGE_HI }));
    }
    // tail fin, fanning out with a notch
    if s > 0.97 {
        let ft = (s - 0.97) / 0.38;
        let half = 0.04 * L + 0.17 * L * ft;
        let notch = 0.12 * L * ft * ft;
        if v.abs() < half && v.abs() > notch * (1.0 - v.abs() / half) && ft <= 1.0 {
            let rim = ft > 0.85 || v.abs() > half - 0.02 * L;
            return Some((0.6, if rim { EDGE } else { ORANGE_HI.mix(ORANGE_LO, ft) }));
        }
    }
    None
}

// ── anemone ──

/// 40 arms rooted on a mound. Every fifth one stands in front of the fish.
fn anemone() -> Vec<(Chain, bool)> {
    (0..40u32)
        .map(|i| {
            let h = |k: u32| hash(i * 13 + k);
            let x = -2.3 + 4.6 * (i as f32 + h(1)) / 40.0;
            let root = (x, -0.95 + 0.25 * (1.0 - (x / 2.3).powi(2)));
            let angle = PI / 2.0 - x * 0.22 + (h(2) - 0.5) * 0.5;
            let mut arm = Chain::new(root, angle, 12, 1.8 + h(3) * 1.2, (h(4) - 0.5) * 0.08);
            arm.stiff = (40.0, 5.0 + h(5) * 3.0);
            (arm, i % 5 == 0)
        })
        .collect()
}

/// One step of the reef: the current sways every arm, and the fish shoves them aside.
fn sway(arms: &mut Vec<(Chain, bool)>, t: f32, dt: f32) {
    let f = fish_at(t);
    let along = rot2((0.42 * L * f.face, 0.0), f.pitch * f.face.signum());
    for (arm, _) in arms.iter_mut() {
        arm.step(dt, |p, i| {
            let k = i as f32 / 11.0;
            let surge = 1.1 * (t * 0.8 + p.1 * 1.3).sin() + 0.5 * (t * 2.1 + p.0 * 2.0).sin();
            (k * surge, k * 0.4 * (t * 1.3 + p.0).cos())
        });
        for b in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            arm.push_out((f.c.0 + along.0 * b, f.c.1 + along.1 * b), 0.5 - 0.15 * b * b);
        }
    }
}

fn draw_arm(c: &mut Canvas, arm: &Chain, dim: f32) {
    let n = arm.pts.len();
    for k in 0..n - 1 {
        let (a, b) = (arm.pts[k], arm.pts[k + 1]);
        let s = k as f32 / (n - 1) as f32;
        let w = 0.085 * (1.0 - 0.55 * s);
        let l = dist(a, b).max(1e-6);
        let nrm = (-(b.1 - a.1) / l * w, (b.0 - a.0) / l * w);
        let col = ARM_BASE.mix(ARM_TIP, s).dim(dim);
        let off = |p: V2, k: f32| (p.0 + nrm.0 * k, p.1 + nrm.1 * k);
        c.char_line(a, b, if s > 0.75 { 'o' } else { '0' }, col.dim(0.85));
        c.char_line(off(a, 0.5), off(b, 0.5), '8', col.dim(0.75));
        c.char_line(off(a, -0.5), off(b, -0.5), '8', col.dim(0.75));
        c.glyph_line(off(a, 1.0), off(b, 1.0), col);
        c.glyph_line(off(a, -1.0), off(b, -1.0), col.dim(0.6));
    }
    let (x, y) = c.cell_at(arm.pts[n - 1]);
    c.set(x, y, 'o', ARM_DOT.dim(dim));
}

fn main() {
    let mut s = Scene::new();
    s.size = (90, 80);
    s.fps = 30.0;
    s.bg = Rgb::hex(0x05080D);

    // water: a dark gradient with slow light shafts, drifting specks and a sandy mound
    s.always(|c, k| {
        let t = k.time;
        c.shade(move |x, y| {
            if y < CODE_TOP {
                return None;
            }
            let g = ((y - CODE_TOP) / 5.3).clamp(0.0, 1.0);
            let ray = (0.5 + 0.5 * (x * 1.7 + y * 0.6 - t * 0.15).sin()).powi(8) * g * 0.35;
            Some(Rgb::hex(0x04070C).mix(Rgb::hex(0x0C2536), g * 0.8 + ray))
        });
        let w = c.w();
        for i in 0..70u32 {
            let y = -0.9 + (hash(i + 99) * 5.0 - t * (0.05 + 0.1 * hash(i + 7))).rem_euclid(4.9);
            let x = (hash(i) * 2.0 - 1.0) * w + 0.08 * (t * 0.5 + i as f32).sin();
            c.dot((x, y), Rgb::hex(0x9FB8C8).dim(0.25 + 0.4 * hash(i + 3)));
        }
        c.ascii(" .,:;", |x, y| {
            let top = -0.95 + 0.25 * (1.0 - (x / 2.3).powi(2));
            (y < top && y > CODE_TOP).then(|| (0.25 + 0.6 * noise(x * 6.0, y * 9.0), Rgb::hex(0x7A6650)))
        });
    });

    // the reef itself
    let reef = Sim::new(1.0 / 60.0, anemone, sway);
    s.play(24.0, move |c, k| {
        let (t, f) = (k.time, fish_at(k.time));
        let arms = reef.at(t);
        for (arm, front) in arms.iter() {
            if !front {
                draw_arm(c, arm, 0.55);
            }
        }
        c.ascii(FISH_RAMP, move |x, y| fish_px(f, t, x, y));
        for (arm, front) in arms.iter() {
            if *front {
                draw_arm(c, arm, 1.0);
            }
        }
        // a few bubbles rising from the anemone
        for b in 0..5u32 {
            let age = (t + hash(b) * 6.0) % 6.0;
            let x = (hash(b + 20) - 0.5) * 3.5 + 0.08 * (age * 3.0).sin();
            let (cx, cy) = c.cell_at((x, -0.4 + age * 0.75));
            c.set(cx, cy, if age < 2.0 { '°' } else { 'o' }, Rgb::hex(0xBFE3F0).dim(1.0 - age / 6.0));
        }
    })
    .ease(ease::linear)
    .fade_in(1.0)
    .fade_out(1.0);

    // stats bar, and this file scrolling section by section underneath
    let sections: Vec<usize> = SRC
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with("fn ") || l.starts_with("// ──"))
        .map(|(i, _)| i)
        .collect();
    s.always(move |c, k| {
        let f = fish_at(k.time);
        let (w, (cw, ch)) = (c.w(), c.cell());
        let hud = format!("beat {:.1} Hz   speed {:.2}   arms 40", f.hz, f.speed);
        c.label((-w + 2.0 * cw, 4.0 - 1.5 * ch), &hud, GREY);
        c.label((w - 15.0 * cw, 4.0 - 1.5 * ch), "maniac / reef", GREY.dim(0.7));
        c.panel((0.0, (CODE_TOP - 4.0) / 2.0), 2.0 * w, 4.0 + CODE_TOP, Rgb::hex(0x090C13), 1.0);
        c.line((-w, CODE_TOP), (w, CODE_TOP), GREY.dim(0.3));
        let first = sections[(k.time / 4.0) as usize % sections.len()];
        c.code((-w + cw, CODE_TOP - 0.6 * ch), SRC, first, 25, 86);
    });

    s.show();
}
