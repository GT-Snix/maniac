//! A moonlit pond. Someone with a lantern walks through swaying grass below an old wall,
//! and the still water mirrors all of it. Portrait 9:16.
//!
//! cargo run --release --example pond -- --html pond.html
use maniac::*;

const WATER: f32 = -0.6; // the shoreline
const FIELD_TOP: f32 = 1.2; // where the grass meets the wall
const WALL_TOP: f32 = 2.5;
const MOON: V2 = (1.15, 3.25);
const WARM: Rgb = Rgb::hex(0xFFB45A);
const STONE: Rgb = Rgb::hex(0x8E95A8);
const MOSS: Rgb = Rgb::hex(0x5E8A55);
const DUSK: Rgb = Rgb::hex(0x0A1328);

/// The walker's feet and which way they face (-1 left, 1 right).
fn walker_at(t: f32) -> (V2, f32) {
    let x = 1.45 * (t * 0.22).sin();
    ((x, -0.15), if (t * 0.22).cos() >= 0.0 { 1.0 } else { -1.0 })
}

/// The lantern swings from the walker's hand.
fn lantern_at(t: f32) -> V2 {
    let (p, face) = walker_at(t);
    let swing = 0.04 * (t * 4.5).sin();
    (p.0 + (0.3 + swing) * face, p.1 + 0.42)
}

/// A hooded walker as a character shader, about one unit tall.
fn walker_px(t: f32, x: f32, y: f32) -> Option<(f32, Rgb)> {
    let (p, face) = walker_at(t);
    let (u, v) = ((x - p.0) * face / 1.15, (y - p.1) / 1.15);
    if !(0.0..0.8).contains(&v) || u.abs() > 0.3 {
        return None;
    }
    let step = (t * 4.5).sin();
    if v < 0.2 {
        for leg in [-1.0f32, 1.0] {
            if (u - leg * (0.05 + 0.05 * step)).abs() < 0.035 {
                return Some((0.45, Rgb::hex(0x59607E)));
            }
        }
        return None;
    }
    if v < 0.62 {
        let k = (v - 0.18) / 0.44;
        let half = lerp(0.19, 0.1, k) + 0.02 * (t * 3.0 + v * 10.0).sin() * (1.0 - k);
        if u.abs() < half {
            return Some((0.5 + 0.35 * u / half, Rgb::hex(0x7C88BE).mix(WARM, 0.3 * (u / half).max(0.0))));
        }
    }
    let hood = (u - 0.01).hypot(v - 0.68);
    if hood < 0.09 {
        let opening = u > 0.03 && (v - 0.67).abs() < 0.035;
        return Some(if opening { (0.15, Rgb::hex(0x141826)) } else { (0.7, Rgb::hex(0x8F9AD0)) });
    }
    None
}

/// Old wall: offset bricks, dark mortar, moss creeping up from the ground, an arched gate.
fn wall_px(x: f32, y: f32) -> Option<(f32, Rgb)> {
    let merlon = if ((x / 0.35).floor() as i32).rem_euclid(2) == 0 { 0.18 } else { 0.0 };
    if y < FIELD_TOP - 0.05 || y > WALL_TOP + merlon {
        return None;
    }
    let gate = x.abs() < 0.3 && (y < FIELD_TOP + 0.5 || x.hypot(y - FIELD_TOP - 0.5) < 0.3);
    if gate {
        return Some((0.06, Rgb::hex(0x161A26)));
    }
    let row = ((y - FIELD_TOP) / 0.3).floor();
    let bx = ((x + if row as i32 % 2 == 0 { 0.0 } else { 0.3 }) / 0.6).rem_euclid(1.0);
    let by = ((y - FIELD_TOP) / 0.3).rem_euclid(1.0);
    let n = noise(x * 7.0, y * 11.0);
    if bx < 0.09 || by < 0.3 {
        return None; // mortar: leave a gap so the bricks read
    }
    let lum = 0.35 + 0.4 * n;
    let moss = (noise(x * 3.0, y * 4.0) * 1.6 * (1.0 - (y - FIELD_TOP) / 1.4) - 0.45).clamp(0.0, 0.85);
    Some((lum, STONE.mix(MOSS, moss)))
}

/// One grass blade: (x in -1..1 of the width, base y, height, hue jitter). Far ones first.
fn meadow() -> Vec<(f32, f32, f32, f32)> {
    let mut b: Vec<_> = (0..650u32)
        .map(|i| {
            let y = WATER + 0.05 + hash(i * 4 + 1).powf(0.8) * (FIELD_TOP - WATER - 0.05);
            let near = (FIELD_TOP - y) / (FIELD_TOP - WATER);
            (hash(i * 4) * 2.0 - 1.0, y, (0.08 + 0.14 * hash(i * 4 + 2)) * (0.5 + 0.7 * near), hash(i * 4 + 3))
        })
        .collect();
    b.sort_by(|a, b| b.1.total_cmp(&a.1));
    b
}

fn blade(c: &mut Canvas, (fx, y, h, hue): (f32, f32, f32, f32), t: f32) {
    let x = fx * c.w();
    let (feet, lamp) = (walker_at(t).0, lantern_at(t));
    let wind = 0.5 * (t * 1.1 + x * 1.3 + y * 2.0).sin() + 0.3 * noise(x * 1.5 + t * 0.6, y * 3.0) - 0.15;
    let d = dist((x, y), feet);
    let push = if d < 0.45 { (x - feet.0).signum() * (0.45 - d) * 3.0 } else { 0.0 };
    let lean = 0.35 + 0.5 * (hue - 0.5); // every blade has its own resting slant
    let ang = PI / 2.0 - (lean + wind + push).clamp(-1.3, 1.3);
    let near = (FIELD_TOP - y) / (FIELD_TOP - WATER);
    let glow = (1.0 - dist((x, y), lamp) / 1.3).max(0.0);
    let col = hsv(0.28 + 0.08 * hue, 0.6, 0.3 + 0.55 * near * (0.6 + 0.4 * hue)).mix(WARM, glow * 0.6);
    c.glyph_line((x, y), (x + ang.cos() * h, y + ang.sin() * h), col);
}

fn main() {
    let mut s = Scene::new();
    s.size = (90, 80);
    s.fps = 30.0;
    s.bg = Rgb::hex(0x05060C);
    let blades = meadow();

    s.play(26.0, move |c, k| {
        let t = k.time;
        let w = c.w();

        // sky and field
        c.shade(|_x, y| {
            Some(if y > FIELD_TOP {
                Rgb::hex(0x121B36).mix(Rgb::hex(0x04050B), ((y - FIELD_TOP) / 2.8).clamp(0.0, 1.0))
            } else {
                Rgb::hex(0x060D09).mix(Rgb::hex(0x0B1A10), ((FIELD_TOP - y) / 1.8).clamp(0.0, 1.0))
            })
        });
        for i in 0..90u32 {
            let p = ((hash(i) * 2.0 - 1.0) * w, WALL_TOP + 0.35 + hash(i + 300) * 1.3);
            let tw = 0.5 + 0.5 * (t * (1.0 + 2.0 * hash(i + 5)) + TAU * hash(i + 9)).sin();
            c.dot(p, Rgb::hex(0xCFD8FF).dim(0.2 + 0.6 * tw));
        }

        // moon: a soft halo, then a cratered disc of characters
        c.glow(MOON, 1.1, Rgb::hex(0x2A3155));
        c.glow(MOON, 0.55, Rgb::hex(0x6B6A5A));
        c.ascii(RAMP_DENSE, |x, y| {
            let d = dist((x, y), MOON) / 0.33;
            (d < 1.0).then(|| (0.55 + 0.4 * noise(x * 9.0, y * 9.0) - 0.25 * d * d, Rgb::hex(0xF3EFD9)))
        });

        // the wall, with two torches
        c.ascii(RAMP_DENSE, wall_px);
        for tx in [-1.35f32, 1.35] {
            let flick = 0.75 + 0.25 * noise(t * 6.0 + tx, 0.0);
            c.glow((tx, 1.95), 0.5 * flick, WARM.dim(0.5));
            let (cx, cy) = c.cell_at((tx, 1.95));
            c.set(cx, cy, if flick > 0.85 { '*' } else { '+' }, WARM);
        }

        // grass behind the walker, the walker and lantern, then the grass in front
        let feet = walker_at(t).0.1;
        for &b in blades.iter().filter(|b| b.1 > feet) {
            blade(c, b, t);
        }
        c.ascii("o0O8@", move |x, y| walker_px(t, x, y));
        let lamp = lantern_at(t);
        c.glow(lamp, 0.9, WARM.dim(0.35));
        c.glow(lamp, 0.25, WARM);
        let (lx, ly) = c.cell_at(lamp);
        c.set(lx, ly, '0', Rgb::hex(0xFFE2A8));
        for &b in blades.iter().filter(|b| b.1 <= feet) {
            blade(c, b, t);
        }

        // fireflies drifting over the meadow
        for i in 0..16u32 {
            let p = (
                w * 0.9 * (t * (0.05 + 0.05 * hash(i)) + TAU * hash(i + 40)).sin(),
                0.2 + 0.8 * hash(i + 60) + 0.15 * (t * 0.7 + i as f32).sin(),
            );
            let blink = (0.5 + 0.5 * (t * 2.0 + i as f32 * 3.0).sin()).powi(3);
            c.glow(p, 0.07, Rgb::hex(0xC8FF6B).dim(blink * 0.6));
            c.dot(p, Rgb::hex(0xEFFFB0).dim(blink));
        }

        // the water: mirror everything above the shore, then add its own life
        c.reflect(WATER, t, 1.2, DUSK, 0.3);
        let (cols, rows) = (c.cols as i32, c.rows as i32);
        let shore = c.cell_at((0.0, WATER)).1;
        for r in shore..rows {
            let depth = (r - shore) as f32 / (rows - shore) as f32;
            for k in 0..5u32 {
                let speed = 2.0 + 3.0 * hash(r as u32 * 7 + k);
                let col = ((hash(r as u32 * 31 + k) * cols as f32 + t * speed) as i32).rem_euclid(cols);
                c.set(col, r, if k % 2 == 0 { '~' } else { '-' }, Rgb::hex(0x4A6FA8).dim(0.35 + 0.3 * depth));
            }
            // a glitter path under the moon
            let wob = (3.0 * (t * 1.7 + r as f32 * 0.9).sin() * (0.3 + depth)) as i32;
            let mx = c.cell_at(MOON).0 + wob;
            for dx in -2..=2 {
                if hash((r * 97 + dx) as u32 ^ (t * 6.0) as u32) < 0.55 - depth * 0.3 {
                    c.set(mx + dx, r, if dx == 0 { '=' } else { '-' }, Rgb::hex(0xF3EFD9).dim(0.75 - 0.4 * depth));
                }
            }
        }
        // rings spreading out where something touched the water
        for i in 0..4u32 {
            let period = 3.5;
            let age = (t + i as f32 * 0.9) % period;
            let n = ((t + i as f32 * 0.9) / period) as u32;
            let o = ((hash(i * 50 + n) * 2.0 - 1.0) * w * 0.8, WATER - 0.4 - hash(i * 50 + n + 7) * 2.6);
            let r = 0.12 + age * 0.55;
            let ring = Path::circle(r).map(|(x, y)| (o.0 + x, o.1 + y * 0.32));
            let col = Rgb::hex(0x9DB7E8).dim(1.0 - age / period);
            for (a, b) in ring.pts.iter().zip(ring.pts.iter().skip(4)).step_by(4) {
                c.glyph_line(*a, *b, col);
            }
        }
        // lily pads with a flower or two
        for (i, pad) in [(-1.3, -1.4), (0.6, -2.3), (-0.4, -3.2)].into_iter().enumerate() {
            let bob = 0.02 * (t * 1.2 + i as f32).sin();
            c.ascii("oO0@", move |x, y| {
                let (u, v) = ((x - pad.0) / 0.32, (y - pad.1 - bob) / 0.09);
                let notch = u > 0.0 && v.abs() < u * 0.35;
                (u * u + v * v < 1.0 && !notch).then(|| (0.4 + 0.4 * (1.0 - v.abs()), Rgb::hex(0x2F6B3B)))
            });
            if i != 1 {
                let (fx, fy) = c.cell_at((pad.0 - 0.08, pad.1 + bob + 0.05));
                c.set(fx, fy, '*', PINK);
            }
        }

        // reeds at the shore corners
        for i in 0..14u32 {
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let x = side * (w - 0.05 - hash(i) * 0.5);
            let h = 0.5 + hash(i + 3) * 0.6;
            let lean = 0.15 * (t * 0.9 + i as f32).sin();
            c.glyph_line((x, WATER - 0.05), (x + lean - side * 0.1, WATER - 0.05 + h), Rgb::hex(0x55704A));
        }

        let ch = c.cell().1;
        c.label((-w + 0.1, 4.0 - 1.5 * ch), "a night by the pond", Rgb::hex(0xCFD8FF).dim(0.8));
        c.label((w - 0.75, 4.0 - 1.5 * ch), "maniac / pond", GREY.dim(0.7));
    })
    .ease(ease::linear)
    .fade_in(1.2)
    .fade_out(1.2);

    s.show();
}
