//! The smallest useful scene: draw a circle, morph it into a star, type a line.
//! cargo run --example hello          (terminal)
//! cargo run --example hello -- --html hello.html
use maniac::*;

fn main() {
    let mut s = Scene::new();
    let circle = Path::circle(2.0);
    let star = Path::star(5, 2.4, 1.0);

    let c0 = circle.clone();
    s.play(1.5, move |c, k| c.neon(&c0.partial(k.p), rainbow));
    s.play(1.5, move |c, k| c.neon(&circle.morph(&star, k.p), SUNSET)).hold();
    s.also(1.5, |c, k| c.write((0.0, -3.2), "hello, maniac", k.p, WHITE)).hold();
    s.wait(1.0);

    s.show();
}
