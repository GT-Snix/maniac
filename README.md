# maniac

A small ASCII animation engine in Rust. It borrows manim's best idea (a timeline of
animations you play one after another) but drops the object system. Every animation
here is just a function of time:

```rust
s.play(1.5, |c, k| c.neon(&circle.partial(k.p), rainbow));
```

`c` is the canvas, `k` is the tick (`k.p` is eased progress from 0 to 1, `k.time` is
seconds since the scene began). Draw whatever you like in there. Maths, a solar
system, fireworks, a title card: anything that moves.

One file (`src/lib.rs`), no dependencies.

## Run it

```sh
cargo run --release --example showcase                      # plays in your terminal
cargo run --release --example showcase -- --html show.html  # bakes an HTML player
cargo run --release --example matrix                       # linear transformations demo
```

`showcase.html`, `matrix.html` and `hello.html` in this folder are already baked, and `matrix.mp4` is the matrix demo as a 1080p video. Open them in a
browser. Space pauses, the arrow keys seek, `b` toggles bloom, and clicking the bar
jumps.

## The smallest scene

```rust
use maniac::*;

fn main() {
    let mut s = Scene::new();
    let circle = Path::circle(2.0);
    let star = Path::star(5, 2.4, 1.0);

    let c0 = circle.clone();
    s.play(1.5, move |c, k| c.neon(&c0.partial(k.p), rainbow));              // draw it
    s.play(1.5, move |c, k| c.neon(&circle.morph(&star, k.p), SUNSET)).hold(); // morph, then stay
    s.also(1.5, |c, k| c.write((0.0, -3.2), "hello, maniac", k.p, WHITE)).hold(); // at the same time
    s.wait(1.0);

    s.show(); // terminal, or HTML if you pass --html
}
```

## How it works

The world is laid out like manim's. The origin is in the middle, y points up, and the
screen is always 8 units tall, so a circle of radius 2 looks the same at any terminal
size. `c.w()` gives you the half-width.

Each terminal cell is built from three layers:

| layer  | resolution per cell        | used by |
|--------|----------------------------|---------|
| pixels | 2 full-color (`▀` halves)  | `fill`, `shade`, `glow`, `disc`, `big` |
| dots   | 8 braille dots             | `line`, `stroke`, `neon`, `arrow`, `grid`, `axes`, `mesh` |
| text   | 1 character                | `text`, `write`, `surface` |

Text sits on top of the dots, and the dots sit on top of the pixels. Colors are 24-bit
everywhere.

### Timeline (`Scene`)

- `play(dur, f)` runs after everything before it. `also(dur, f)` starts alongside the previous clip. `at(t, dur, f)` starts at an absolute time. `always(f)` runs for the whole scene. `wait(d)` adds a pause.
- Each of those returns a clip you can chain: `.ease(ease::bounce)`, `.hold()`, `.linger(2.0)`, `.until(t)`, `.fade_in(d)`, `.fade_out(d)`. Fades apply to everything the clip draws, so you never have to handle alpha yourself.
- A clip disappears when it ends unless you `hold` it. That makes chains like create, then morph, then morph again work without bookkeeping.

### Shapes (`Path`)

`circle`, `rect`, `poly`, `star`, `heart`, `line`, `func(f, x0, x1)`, `param(f)`. You can
transform any of them with `at`, `scale`, `rot`, or `map(any fn)`. `partial(p)` gives you
manim's Create, and `morph(&other, p)` gives you Transform. Every built-in shape starts at
the top and runs clockwise, so any one of them morphs cleanly into any other.

### Color

You can pass a color to any `ink` argument as a plain `Rgb`, as a gradient array (`SUNSET`, `OCEAN`,
`NEON`, `AURORA`, `FIRE`, `ICE`, or your own `[Rgb; N]`), or as a closure from position to color,
`|u| rainbow(u + k.time)`. The helpers are `hsv`, `grad`, `rainbow`, `Rgb::mix`, `Rgb::dim`,
plus a manim-ish palette of constants.

### Extras

- `shade(|x, y| Some(color))` runs a pixel shader over the whole screen, split across all CPU cores.
- `surface(|u, v| point, ink)` draws a lit, z-buffered ASCII surface (the donut.c idea, for any shape). Normals are computed for you. `torus`, `sphere` and `cube` are included, and `rot3` and `add3` move them around.
- `panel(center, w, h, color, opacity)` draws a solid box that hides whatever is under it. Put one behind labels. `cell()` gives the size of one terminal cell in world units.
- `hash`, `noise` and `stagger` give you deterministic randomness, smooth noise and manim's LaggedStart. Because nothing is random at runtime, every frame depends only on time, so scenes can be seeked and baked to HTML.
- `ease::{smooth, linear, in_cubic, out_cubic, in_out, back, elastic, bounce, there_and_back}`.

## Speed

- The terminal player diffs each frame against the last one and only writes cells that changed. It also skips cursor moves and color codes that are already in effect, so all the escape codes for a frame go out in one write.
- The HTML export stores frames as binary deltas: a one-byte flag per changed cell, followed only by the fields that changed.
- The whole 50-second showcase (about 1,500 frames at 160×45) bakes in about 1.3 seconds, and that was single-threaded inside WebAssembly. Native should be faster.

## JetBrains Mono

A program can't choose your terminal's font. For the terminal, set JetBrains Mono in your
terminal's own settings. Any monospace font works, but braille and half blocks look best in
fonts that draw them edge to edge. The HTML player loads JetBrains Mono from Google Fonts
and uses it for every glyph. It draws the braille dots and half blocks itself, so they
stay crisp even where a font lacks them.

## Honest limits

- You need a terminal with truecolor (iTerm2, WezTerm, Kitty, Alacritty, Ghostty, and recent versions of Terminal.app all have it). On terminals with 256 colors or fewer, colors will look wrong.
- There's no keyboard input yet. Ctrl-C quits, and the signal handler restores the terminal first. Typed keys may echo over the picture.
- Native terminal sizing and signals use raw `ioctl`/`signal` calls on Unix. On other platforms it falls back to a fixed 120×36 and has no Ctrl-C cleanup.
- Full-screen shaders change every cell in every frame, so those parts of a baked HTML file are big. The showcase file is about 19 MB, mostly because of its 5-second plasma section.
- Text is placed one character per cell. Wide characters (CJK, emoji) will throw off the alignment.
- `lib.rs` is about 1,100 lines with comments. That's "short" for what it does, but it isn't tiny.
