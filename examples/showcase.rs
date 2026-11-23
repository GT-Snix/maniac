//! A tour of everything manic does, about 50 seconds long.
//! cargo run --release --example showcase
//! cargo run --release --example showcase -- --html showcase.html
use maniac::*;

fn main() {
    let mut s = Scene::new();

    // twinkling starfield behind everything
    s.always(|c, k| {
        let (w, h) = (c.w(), c.h());
        for i in 0..180u32 {
            let p = ((hash(i) * 2.0 - 1.0) * w, (hash(i + 500) * 2.0 - 1.0) * h);
            let tw = 0.5 + 0.5 * (k.time * (0.8 + 3.0 * hash(i + 7)) + TAU * hash(i + 3)).sin();
            c.dot(p, rainbow(hash(i + 11)).dim(0.12 + 0.45 * tw));
        }
    });

    title(&mut s);
    shapes(&mut s);
    graph(&mut s);
    fourier(&mut s);
    three_d(&mut s);
    plasma(&mut s);
    solar(&mut s);
    finale(&mut s);

    s.show();
}

fn title(s: &mut Scene) {
    s.play(5.0, |c, k| {
        for (i, ch) in "MANIAC".chars().enumerate() {
            let p = ease::bounce(stagger((k.t / 1.6).min(1.0), i, 6, 0.35));
            let (x, y) = ((i as f32 - 2.5) * 2.0, lerp(6.0, 1.2, p));
            let hue = i as f32 * 0.12 + k.time * 0.25;
            let ch = ch.to_string();
            c.big((x + 0.1, y - 0.1), &ch, 2.4, BLACK);
            c.big((x, y), &ch, 2.4, move |u| hsv(hue + u * 0.15, 0.75, 1.0));
        }
        let w = c.w() * 0.7 * ease::out_cubic(((k.t - 1.4) / 0.8).clamp(0.0, 1.0));
        if w > 0.01 {
            c.neon(&Path::line((-w, -0.7), (w, -0.7)), |u| rainbow(u + k.time * 0.3));
        }
        c.write((0.0, -1.6), "a tiny, fast, ridiculously colorful ASCII animation engine", (k.t - 1.8) / 1.6, WHITE);
        c.write((0.0, -2.4), "written in rust  ·  everything is a function of time", (k.t - 3.0) / 1.2, NEON);
    })
    .ease(ease::linear)
    .fade_out(0.6);
}

fn shapes(s: &mut Scene) {
    let shapes = [Path::circle(2.2), Path::rect(3.6, 3.6), Path::star(5, 2.6, 1.05), Path::heart(2.6), Path::poly(6, 2.3)];
    let names = ["circle", "square", "star", "heart", "hexagon"];
    let inks = [OCEAN, SUNSET, NEON, [PINK, RED, GOLD], AURORA];
    s.play(7.5, move |c, k| {
        let t = k.t;
        let (path, label, ink) = if t < 1.2 {
            (shapes[0].partial(ease::smooth(t / 1.2)), names[0].to_string(), inks[0])
        } else {
            let x = (t - 1.2) / 1.5;
            let i = (x as usize).min(3);
            let f = ease::smooth(((x - i as f32) * 1.5).min(1.0));
            let ink = [0, 1, 2].map(|j| inks[i][j].mix(inks[i + 1][j], f));
            (shapes[i].morph(&shapes[i + 1], f), format!("{}  →  {}", names[i], names[i + 1]), ink)
        };
        let path = path.rot(t * 0.3);
        if t > 1.0 {
            c.fill(&path, ink, 0.25 * ((t - 1.0) / 0.5).min(1.0));
        }
        c.neon(&path, ink);
        c.text((0.0, 3.5), "Path::morph  —  any shape into any shape", GREY);
        c.text((0.0, -3.4), &label, ink);
    })
    .ease(ease::linear)
    .fade_in(0.3)
    .fade_out(0.5);
}

fn graph(s: &mut Scene) {
    let f = |x: f32| 2.0 * x.sin() * (x / 3.0).cos();
    s.play(7.0, move |c, k| {
        let w = c.w();
        let g = ((k.t - 0.2) / 0.8).clamp(0.0, 1.0);
        c.grid(1.0, BLUE.dim(0.05 + 0.15 * g));
        c.axes(GREY.dim(g));
        let p = ease::smooth(((k.t - 0.8) / 2.2).clamp(0.0, 1.0));
        c.neon(&Path::func(f, -w, w).partial(p), OCEAN);
        if k.t > 3.0 {
            let x = -w * 0.85 + ease::in_out(((k.t - 3.0) / 3.6).min(1.0)) * w * 1.7;
            let (y, d) = (f(x), (f(x + 1e-3) - f(x - 1e-3)) / 2e-3);
            let a = 1.6 / (1.0 + d * d).sqrt();
            c.line((x - a, y - a * d), (x + a, y + a * d), [MAGENTA, PINK, MAGENTA]);
            c.line((x, 0.0), (x, y), YELLOW.dim(0.5));
            c.glow((x, y), 0.6, GOLD);
            c.disc((x, y), 0.1, WHITE);
            c.text((x, y + if y > 0.0 { 0.7 } else { -0.7 }), &format!("slope {d:+.2}"), PINK);
        }
        c.write((0.0, 3.5), "f(x) = 2·sin(x)·cos(x/3)", (k.t - 0.4) / 1.0, GOLD);
    })
    .ease(ease::linear)
    .fade_in(0.4)
    .fade_out(0.5);
}

/// Sum of rotating vectors (freq, re, im) at time t.
fn epicycle(coef: &[(f32, f32, f32)], t: f32) -> V2 {
    coef.iter().fold((0.0, 0.0), |p, &(f, re, im)| {
        let (s, c) = (TAU * f * t).sin_cos();
        (p.0 + re * c - im * s, p.1 + re * s + im * c)
    })
}

fn fourier(s: &mut Scene) {
    let n = 256;
    let pts = Path::heart(2.9).resample(n).pts;
    let mut coef: Vec<(f32, f32, f32)> = (-(n as i32) / 2..n as i32 / 2)
        .map(|k| {
            let (mut re, mut im) = (0.0, 0.0);
            for (j, &(x, y)) in pts.iter().enumerate() {
                let (s, c) = (-TAU * k as f32 * j as f32 / n as f32).sin_cos();
                re += x * c - y * s;
                im += x * s + y * c;
            }
            (k as f32, re / n as f32, im / n as f32)
        })
        .collect();
    coef.sort_by(|a, b| b.1.hypot(b.2).total_cmp(&a.1.hypot(a.2)));
    coef.truncate(48);

    s.play(8.0, move |c, k| {
        let steps = (k.p * 500.0) as usize;
        let trace = Path::new((0..=steps).map(|i| epicycle(&coef, i as f32 / 500.0)).collect(), false);
        c.neon(&trace, |u| rainbow(u * 0.8 + k.time * 0.1));
        let mut p = (0.0, 0.0);
        for (i, &(f, re, im)) in coef.iter().enumerate() {
            let (s, co) = (TAU * f * k.p).sin_cos();
            let q = (p.0 + re * co - im * s, p.1 + re * s + im * co);
            let col = hsv(0.5 + i as f32 / 60.0, 0.6, 1.0);
            let r = re.hypot(im);
            if r > 0.04 {
                c.stroke(&Path::circle(r).at(p.0, p.1), col.dim(0.3));
            }
            c.line(p, q, col.dim(0.9));
            p = q;
        }
        c.glow(p, 0.5, WHITE);
        c.disc(p, 0.08, WHITE);
        c.text((0.0, -3.6), "48 spinning circles draw a heart  ·  Fourier series", GREY);
    })
    .fade_in(0.4)
    .fade_out(0.6);
}

fn three_d(s: &mut Scene) {
    let (verts, edges) = cube();
    s.play(6.0, move |c, k| {
        let (a, b) = (k.time * 1.1, k.time * 0.7);
        let w = c.w();
        c.surface(|u, v| add3(rot3(torus(u, v, 1.6, 0.7), a, b, 0.0), (-w * 0.45, 0.0, 0.0)), FIRE);
        let vs: Vec<V3> = verts.iter().map(|&p| add3(rot3((p.0 * 1.4, p.1 * 1.4, p.2 * 1.4), b, a, 0.3), (w * 0.45, 0.0, 0.0))).collect();
        for (i, &p) in vs.iter().enumerate() {
            c.glow(c.proj(p), 0.45, rainbow(i as f32 / 8.0 + k.time * 0.2).dim(0.7));
        }
        c.mesh(&vs, &edges, |u| rainbow(u + k.time * 0.2));
        c.text((-w * 0.45, -3.5), "Canvas::surface  —  lit, z-buffered", FIRE);
        c.text((w * 0.45, -3.5), "Canvas::mesh  —  wireframes", NEON);
    })
    .fade_in(0.5)
    .fade_out(0.6);
}

fn plasma(s: &mut Scene) {
    s.play(5.0, |c, k| {
        let t = k.time;
        c.shade(move |x, y| {
            let v = (x * 1.1 + t).sin()
                + (y * 1.6 - t * 1.3).sin()
                + ((x * x + y * y).sqrt() * 1.8 - t * 2.0).sin()
                + ((x * 0.6 - y * 0.8) * 1.4 + t * 0.7).sin();
            Some(hsv(v * 0.12 + t * 0.08, 0.85, 0.5 + 0.1 * v.cos()))
        });
        let pop = ease::back(((k.t - 0.4) / 0.7).clamp(0.0, 1.0));
        if pop > 0.05 {
            c.big((0.12, 1.08), "ANY", 2.6 * pop, BLACK);
            c.big((0.0, 1.2), "ANY", 2.6 * pop, WHITE);
        }
        let pop = ease::back(((k.t - 0.9) / 0.7).clamp(0.0, 1.0));
        if pop > 0.05 {
            c.big((0.08, -1.78), "ANIMATION", 1.4 * pop, BLACK);
            c.big((0.0, -1.7), "ANIMATION", 1.4 * pop, WHITE);
        }
        c.text((0.0, -3.5), "Canvas::shade  —  a pixel shader on every core", WHITE);
    })
    .fade_in(0.5)
    .fade_out(0.6);
}

fn solar(s: &mut Scene) {
    const TILT: f32 = 0.38;
    let planets = [(1.3, 1.9, 0.10, GREY), (1.9, 1.4, 0.16, GOLD), (2.6, 1.0, 0.18, BLUE), (3.4, 0.75, 0.14, RED), (4.7, 0.42, 0.34, ORANGE), (6.1, 0.3, 0.26, TEAL)];
    s.play(7.0, move |c, k| {
        let t = k.time * 0.8;
        let orbit = |r: f32, a: f32| (r * a.cos(), TILT * r * a.sin());
        for &(r, ..) in &planets {
            c.stroke(&Path::circle(r).map(|(x, y)| (x, y * TILT)), GREY.dim(0.25));
        }
        for front in [false, true] {
            for (i, &(r, sp, size, col)) in planets.iter().enumerate() {
                let a = sp * t + i as f32 * 1.7;
                if (a.sin() < 0.0) != front {
                    continue;
                }
                c.stroke(&Path::param(|u| orbit(r, a - 0.9 * (1.0 - u))), |u: f32| col.dim(u));
                let p = orbit(r, a);
                c.glow(p, size * 3.0, col.dim(0.5));
                c.disc(p, size, col);
                if i == 2 {
                    c.disc((p.0 + 0.35 * (t * 4.0).cos(), p.1 + 0.15 * (t * 4.0).sin()), 0.05, WHITE);
                }
                if i == 4 {
                    c.stroke(&Path::circle(size * 1.9).map(|(x, y)| (x + p.0, y * 0.3 + p.1)).rot(0.0), GOLD.dim(0.8));
                }
            }
            if !front {
                c.glow((0.0, 0.0), 1.8, ORANGE.dim(0.55));
                c.glow((0.0, 0.0), 0.9, GOLD);
                c.disc((0.0, 0.0), 0.45, YELLOW);
            }
        }
        // a comet with a particle tail
        let w = c.w();
        let head = lerp2((-w - 1.0, 3.4), (w + 1.0, 1.2), k.raw);
        for j in 0..80u32 {
            let back = hash(j) * 2.2;
            let wob = (hash(j + 99) - 0.5) * 0.4 * back;
            c.dot((head.0 - back * 0.97, head.1 + back * 0.24 + wob), grad(&ICE, 1.0 - back / 2.2));
        }
        c.glow(head, 0.35, SKY);
        c.text((0.0, -3.5), "not just maths  —  anything that moves", SUNSET);
    })
    .ease(ease::linear)
    .fade_in(0.5)
    .fade_out(0.6);
}

fn finale(s: &mut Scene) {
    s.play(7.0, |c, k| {
        for b in 0..10u32 {
            let age = k.t - b as f32 * 0.45;
            if age < 0.0 {
                continue;
            }
            let o = ((hash(b * 7 + 1) * 2.0 - 1.0) * c.w() * 0.75, 0.6 + hash(b * 7 + 2) * 2.6);
            let hue = hash(b * 7 + 3);
            if age < 0.5 {
                c.glow((o.0, lerp(-4.0, o.1, ease::out_cubic(age / 0.5))), 0.22, GOLD);
                continue;
            }
            let age = age - 0.5;
            if age > 2.2 {
                continue;
            }
            let fade = 1.0 - age / 2.2;
            let at = |j: u32, t: f32| {
                let ang = TAU * j as f32 / 70.0 + hash(b * 131 + j) * 0.1;
                let sp = 1.4 + hash(b * 977 + j) * 1.6;
                (o.0 + ang.cos() * sp * t, o.1 + ang.sin() * sp * t - 1.3 * t * t)
            };
            for j in 0..70u32 {
                let col = hsv(hue + hash(j) * 0.1, 0.7, 1.0).dim(fade);
                c.line(at(j, (age - 0.12).max(0.0)), at(j, age), col);
                if j % 6 == 0 {
                    c.glow(at(j, age), 0.18, col.dim(0.7));
                }
            }
            c.glow(o, 1.3 * (1.0 - age / 0.5).max(0.0), WHITE);
        }
        let a = ((k.t - 1.5) / 1.0).clamp(0.0, 1.0);
        if a > 0.0 {
            let x = k.time;
            c.big((0.1, -0.6), "THE END", 1.6, BLACK.mix(c.bg, 1.0 - a));
            c.big((0.0, -0.5), "THE END", 1.6, move |u: f32| hsv(u * 0.6 + x * 0.2, 0.7, 1.0).dim(a));
        }
        c.write((0.0, -2.6), "made with maniac", (k.t - 2.6) / 1.0, GREY);
    })
    .ease(ease::linear)
    .fade_out(1.0);
}
