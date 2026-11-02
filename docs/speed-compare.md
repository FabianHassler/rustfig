# Speed comparison with xfig

**Status:** 2026-11-02. Measured with `cargo xtask speed-compare` (see below); the tables between the markers are generated, everything else is written by hand. Revision history: 2026-11-02 first version (RustFig = local main 36712f0, after the speed work).

RustFig against the real xfig 3.2.9a on the same figures: open, zoom, pan, move, save, undo. This page is the evidence for a speed claim on the website; it says where xfig is faster as well.

## Setup

**What is compared**

- **xfig 3.2.9a**, built from the unmodified sources in `old/xfig-3.2.9a` with the same configure flags, dependencies (Xaw3d, Xft, libpng, jpeg-turbo, libtiff, libxpm, ghostscript, Homebrew's app-defaults directory with `Fig.international: False`) and `CFLAGS=-O2` as Homebrew's formula (`brew cat xfig`), plus timing hooks. `xtask/xfig-timing/build.sh` copies the sources, applies `xtask/xfig-timing/xfig-timing.patch` (the hooks only; `old/` is never touched) and builds. With `XFIG_TIMING=1` the hooks print `XT <name> <start ms> <duration ms>` on stderr (CLOCK_MONOTONIC); without the variable they do nothing. Default options, scratch `HOME`, on its own `Xvfb :78` (1400x1000x24, software X server, started with `-fbdir`), window `1322x818`, canvas 1128x686 pixels.
- **RustFig**: release build, `--background`, scratch settings and recovery folders, window `-geometry 1400x1000` (the display clips the height: canvas 1128x686 points, i.e. 2256x1372 pixels on a retina screen), driven through its control port. The build under test is given to `cargo xtask speed-compare --rustfig BIN`.
- Same machine, one after the other, never in parallel. The same figures, plain `.fig` copies (xfig reads `.fig.gz` too, but both get the same file). A new process for every run; each value is the **median of 3 runs**; a measurement that does not finish in 120 s is reported as "> 120 s".

**Hooks in xfig** (each span ends with `XSync`, so the X server's drawing is included):

| Span | Where | Measures |
|---|---|---|
| `main_start` | `main()` | zero point of the open time |
| `read_fig` | `load_file` around `read_figc` | reading the file |
| `redraw` | `redisplay_region` | every canvas redraw |
| `zoom` | `show_zoom` | one zoom step (indicator, rulers, grid, redraw) |
| `setup_grid` | `setup_grid` | pan by ruler (the middle button on the ruler ends in it) and part of every zoom |
| `move_step` | `w_canvas.c`, pointer motion in Move mode with an object held | one step of the rubber band |
| `place` | `w_canvas.c`, left button in Move mode with an object held | placing the object |
| `undo`, `save` | `undo`, `write_file` | Undo, writing the file |

Spans that follow each other within 250 ms form one action: its time runs from the start of the first span to the end of the last redraw. **Open** for xfig is `main()` to the end of the first redraw after the file was read (process start-up and the X connection before `main()` are not counted; reading the file and the first draw are listed separately).

**What is timed in RustFig.** The control-channel round trip costs 9-40 ms by itself and every poll is served in a new frame (60 Hz), so wall time seen from outside cannot go below one frame (16.7 ms). RustFig is therefore timed inside the app: `ui.state` reports `frame_ms` (the time of the last frame), `frame_ms_max_recent` (the slowest of the last 120 frames), `render_ms` (a background raster) and `canvas_pending` (a render is running or the raster on screen is not the current view: the "drawn" signal, added for this measurement in `crates/ui-egui/src/canvas.rs`/`control.rs`). For each action the harness first pushes 130 idle frames through, so that the window of recent frames holds nothing older; then it sends the action and polls until `canvas_pending` is false. The reported time is the larger of the sum of the frame times from the action to that poll and the slowest recent frame, plus the background render time if a background render ran. That is RustFig's own work (scene build, raster, UI, GPU submission as far as the frame function goes). It does not include presenting the frame (vsync) or the time the window server needs. The **open** time is different: from starting the process to the first `ui.state` with the drawing current and no queued input for 150 ms, measured from outside; it includes start-up of the executable, fonts and the window. xfig's open time starts at `main()`, so it favours xfig by the dynamic linking and the X connection (about 20-50 ms). **Save** is the time of the `file.save` command, as `write_file` is in xfig.

**Pictures of the same action from outside.** The tables at the end repeat each task seen from outside: xfig's canvas pixels watched in Xvfb's frame buffer file (the harness reads it every 2 ms, a change followed by 400-500 ms of no change and no CPU use means "drawn"), RustFig's wall time to the first idle poll. They agree with the hooks within a few ms for the heavy cases and show xfig's detection latency (about 30 ms) on the light ones; the published numbers are the hooks.

**Tasks.** *Zoom in/out*: one step (xfig: the `Z`/`z` key with the pointer at the canvas centre, a step of sqrt 2; RustFig: `view.zoom_in`/`zoom_out`, also sqrt 2) at zoom 1 with Fig (0,0) at the canvas corner. *Pan*: half a canvas width (500 px): xfig by the middle button dragged along the top ruler (the redraw happens when the button is released), RustFig `view.pan`. *Move*: Move mode, pick one object by its vertex or outline, 30 pointer steps over 240 x 120 pixels, then place. Figures of more than 300 objects are first glued into one compound (xfig: Glue mode, middle-button region over the whole canvas at the zoom that shows the whole figure, right button; RustFig: select all, group) and moved at that zoom; the small figures move a single object. *Save*: Alt-S in xfig, `file.save` in RustFig; *Undo*: Alt-U / `edit.undo` after the move.

**What each app draws while moving.** xfig draws the *outline* in XOR mode, straight to the window: for a compound only its bounding box (`moving_box`), for a polyline its shape, for a text its box; nothing underneath is redrawn, so a step costs well under a millisecond whatever the figure. When the object is placed (and for Undo) xfig redraws the affected region of the canvas, which is the expensive part on big figures. RustFig shows the real object moving over a cached layer of the rest of the drawing (since the speed work): a step costs the same for any figure, and placing re-renders.

**Caveats.**

- xfig runs on `Xvfb`, a software X server on the same machine; real XQuartz or a Linux X server would draw faster or slower. The hooks include the server's drawing time (`XSync`), so Xvfb's cost is in xfig's numbers, but not a GPU's.
- RustFig draws 4 times as many pixels (retina, 2x2) for the same canvas size and uses the GPU for presentation; xfig's pixels are 1:1.
- Fonts: xfig uses X fonts, RustFig the system fonts of the machine (its own start-up scans them, in the open time).
- The machine is shared with other builds and tests: the load average was between 1 and 8 during the runs. Medians of 3 runs reduce, not remove, the noise; differences below about 20 % are not meaningful.
- The two apps open a figure differently (xfig at zoom 1, RustFig fitted to the window): the open row compares what the user waits for, not equal work.
- `ui.state` polling adds frames in RustFig; the 130-frame flushes make runs of RustFig long, not slower.

**Re-running with another RustFig build**

```sh
cargo build --release -p rustfig                 # the build to measure (needs the ui.state fields of this page's branch)
xtask/xfig-timing/build.sh /some/dir             # once; builds the instrumented xfig into /some/dir/src/src/xfig
cargo run --release -p xtask -- speed-compare \
    --rustfig target/release/rustfig --label "my build" \
    --xfig-bin /some/dir/src/src/xfig --examples /path/to/examples \
    [--runs 3] [--timeout 120] [--filter pines,greece] [--out docs/speed-compare.md]
```

`--examples` is the folder with the example figures (`pines/pines.fig.gz`, ..., `gallery/*.fig`; the `examples/` folder of the main checkout). The harness starts and stops its own `Xvfb` (display `:78`, `--display` to change), and writes the generated part of this page after each figure. Run it as a release build (a debug build is too slow to watch the frame buffer). A RustFig build without `canvas_pending` in `ui.state` is measured with a fixed 600 ms wait: do not compare those numbers.

## Summary (what the numbers say)

Measured 2026-11-02 on an Apple M5 under a **busy machine** (load average 4-25 from other builds; the RustFig numbers probably suffer most). Treat the tables as upper bounds for RustFig and rerun on an idle machine before quoting exact factors.

- **Open** (xfig `main()` to drawn; RustFig process start to drawn): RustFig is 2.6-6.3x faster on 13 of 15 figures (190-550 ms against xfig's 1.1-1.4 s). xfig spends about 1.1 s in start-up before it reads the file (the reading itself is 3-115 ms, the first redraw 1-125 ms). Not faster: greece (958 ms, 1.3x: a 4 MB picture is decoded) and cube8 (4.3 s against 1.3 s; 190,000 polylines: runs of 3.6, 4.3 and 5.7 s: scene build plus the first raster).
- **Zoom, pan, redraw**: xfig redraws the canvas in 2-40 ms on every figure up to 7,600 objects (130-155 ms for 190-250k objects); RustFig needs 6-30 ms on small figures, 25-140 ms on the dense or picture figures (greece, gosper-curve) and 0.6-1.7 s on cube8, so for plain zoom and pan **xfig is the faster of the two**, mostly 1-5x on small and mid figures and about 10x on cube8. RustFig wins pan on blankmap (3.5x) and ne-countries (1.5x). (penrose-tiling: the 6-9x for RustFig comes from three xfig runs where it redrew at 100-120 ms, probably disturbed by load.)
- **Move, the drag**: both are cheap and constant per step (xfig draws an XOR outline: 0.2-1.6 ms; RustFig 0.1-1.8 ms over the cached layer). The two are even; RustFig shows the real object, xfig an outline (a compound's bounding box).
- **Move, placing**: where xfig is slow RustFig is much faster: ne-coastline 1.2 s against 28 ms (42x), ne-countries 524 ms against 31 ms (17x), wenig 118 against 11 ms (11x), greece 910 against 113 ms (8x), blankmap 90 against 21 ms (4x). On small figures xfig is faster (1.6 against 10 ms). On cube8 RustFig needs 1.5 s against xfig's 376 ms.
- **Undo** follows placing: RustFig 2-6x faster on blankmap/ne-coastline/wenig, xfig faster on the small figures (2-4 ms against 11-30 ms) and cube8. **Save**: RustFig faster on the large files (1.7-1.8x), xfig on the small ones (under 1 ms against 5-13 ms).
- **xfig could be measured everywhere** (no timeouts at 120 s: its slowest action is 1.2 s, placing on ne-coastline). **RustFig could not be measured** for clifford-attractor (250,001 objects) in the move test, nor in 1 of 3 cube8 runs: `edit.select_all` on that many objects did not return within the control channel's 60 s (observed: about 65 s for clifford in a separate probe), so the glue-then-move tasks, save and undo are `n/a` there; open, zoom and pan were measured. That is a RustFig finding (select all is far too slow beyond ~100k objects), not a measurement artefact.

## Results

<!-- results:start -->

### Setup of this run

- Machine: Apple M5, macOS 27.0.1
- xfig: `/private/tmp/claude-501/-Users-fabian-Documents-RustFig/acd7a9ae-6d37-45e5-8f64-b3e95f4ea55e/scratchpad/xfig-build/src/src/xfig` (3.2.9a), default options, scratch `HOME`, on `Xvfb :78` 1400x1000x24; window `1322x818` (canvas 1128x686 pixels)
- RustFig: `/private/tmp/claude-501/-Users-fabian-Documents-RustFig/acd7a9ae-6d37-45e5-8f64-b3e95f4ea55e/scratchpad/speed/bin/rustfig-36712f0` (rustfig 0.9.0-beta.7), release build, label: local main 36712f0 (speed work merged) + ui.state canvas_pending; `-geometry 1400x1000` (clipped to the screen), canvas 1128x686 points
- Each value is the median of 3 run(s), a new process per run; timeout 120 s per measurement ("> N s": not finished in that time; the real time is longer)
- Generated 2026-11-02 15:11 by `cargo xtask speed-compare`

### Results

xfig / RustFig is the speed-up of RustFig (above 1 = RustFig is faster).

#### Open (xfig: main() to figure drawn; RustFig: process start to drawn)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.17 s | 191 ms | 6.1x |
| feynman (small) | 4 kB, 34 obj. | 1.14 s | 181 ms | 6.3x |
| fourier (small) | 16 kB, 41 obj. | 1.41 s | 456 ms | 3.1x |
| pines | 11 kB, 31 obj. | 1.39 s | 332 ms | 4.2x |
| snowy | 173 kB, 436 obj. | 1.27 s | 387 ms | 3.3x |
| wenig | 587 kB, 85 obj. | 1.31 s | 411 ms | 3.2x |
| gosper-curve | 182 kB, 142 obj. | 1.33 s | 517 ms | 2.6x |
| penrose-tiling | 560 kB, 7615 obj. | 1.45 s | 547 ms | 2.6x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.13 s | 412 ms | 2.7x |
| blankmap | 1.0 MB, 2232 obj. | 1.28 s | 228 ms | 5.6x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.17 s | 452 ms | 2.6x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.22 s | 348 ms | 3.5x |
| greece | 3.2 MB, 2013 obj. | 1.21 s | 958 ms | 1.3x |
| clifford-attractor | 13.2 MB, 250001 obj. | 1.33 s | 1.27 s | 1.1x |
| cube8 | 14.6 MB, 190260 obj. | 1.34 s | 4.33 s | 0.3x |

#### Open, part 1: reading the file (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 18.8 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 24.3 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 60.4 ms | n/a | - |
| pines | 11 kB, 31 obj. | 1.6 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 4.8 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 13.6 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 5.3 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 12.5 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 3.1 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 22.6 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 51.8 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 64.0 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 79.8 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 105 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 93.1 ms | n/a | - |

#### Open, part 2: first full canvas redraw (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.6 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 1.1 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 2.5 ms | n/a | - |
| pines | 11 kB, 31 obj. | 5.6 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 5.1 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 20.1 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 5.5 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 43.5 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 5.3 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 11.1 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 3.4 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 10.5 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 11.4 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 120 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 130 ms | n/a | - |

#### Zoom in (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 9.5 ms | 9.6 ms | 1.0x |
| feynman (small) | 4 kB, 34 obj. | 21.7 ms | 8.2 ms | 2.6x |
| fourier (small) | 16 kB, 41 obj. | 21.4 ms | 20.1 ms | 1.1x |
| pines | 11 kB, 31 obj. | 10.0 ms | 5.8 ms | 1.7x |
| snowy | 173 kB, 436 obj. | 6.1 ms | 10.6 ms | 0.6x |
| wenig | 587 kB, 85 obj. | 24.9 ms | 28.3 ms | 0.9x |
| gosper-curve | 182 kB, 142 obj. | 7.3 ms | 107 ms | 0.1x |
| penrose-tiling | 560 kB, 7615 obj. | 38.1 ms | 31.6 ms | 1.2x |
| apollonian-gasket | 330 kB, 4221 obj. | 11.6 ms | 28.3 ms | 0.4x |
| blankmap | 1.0 MB, 2232 obj. | 19.2 ms | 17.8 ms | 1.1x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 10.6 ms | 30.8 ms | 0.3x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 11.4 ms | 26.9 ms | 0.4x |
| greece | 3.2 MB, 2013 obj. | 14.2 ms | 131 ms | 0.1x |
| clifford-attractor | 13.2 MB, 250001 obj. | 136 ms | 394 ms | 0.3x |
| cube8 | 14.6 MB, 190260 obj. | 118 ms | 1.69 s | 0.1x |

#### Zoom out (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 2.4 ms | 11.6 ms | 0.2x |
| feynman (small) | 4 kB, 34 obj. | 4.6 ms | 9.3 ms | 0.5x |
| fourier (small) | 16 kB, 41 obj. | 4.0 ms | 20.0 ms | 0.2x |
| pines | 11 kB, 31 obj. | 7.5 ms | 5.9 ms | 1.3x |
| snowy | 173 kB, 436 obj. | 7.9 ms | 10.6 ms | 0.7x |
| wenig | 587 kB, 85 obj. | 20.1 ms | 38.9 ms | 0.5x |
| gosper-curve | 182 kB, 142 obj. | 6.1 ms | 74.8 ms | 0.1x |
| penrose-tiling | 560 kB, 7615 obj. | 121 ms | 13.0 ms | 9.3x |
| apollonian-gasket | 330 kB, 4221 obj. | 12.9 ms | 22.1 ms | 0.6x |
| blankmap | 1.0 MB, 2232 obj. | 11.9 ms | 14.7 ms | 0.8x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 9.3 ms | 17.7 ms | 0.5x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 10.9 ms | 17.8 ms | 0.6x |
| greece | 3.2 MB, 2013 obj. | 8.2 ms | 142 ms | 0.1x |
| clifford-attractor | 13.2 MB, 250001 obj. | 155 ms | 205 ms | 0.8x |
| cube8 | 14.6 MB, 190260 obj. | 135 ms | 1.18 s | 0.1x |

#### Pan half a screen

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.8 ms | 11.8 ms | 0.2x |
| feynman (small) | 4 kB, 34 obj. | 2.9 ms | 10.2 ms | 0.3x |
| fourier (small) | 16 kB, 41 obj. | 28.7 ms | 21.0 ms | 1.4x |
| pines | 11 kB, 31 obj. | 2.8 ms | 9.4 ms | 0.3x |
| snowy | 173 kB, 436 obj. | 5.5 ms | 15.4 ms | 0.4x |
| wenig | 587 kB, 85 obj. | 22.3 ms | 34.8 ms | 0.6x |
| gosper-curve | 182 kB, 142 obj. | 5.3 ms | 66.4 ms | 0.1x |
| penrose-tiling | 560 kB, 7615 obj. | 108 ms | 16.9 ms | 6.4x |
| apollonian-gasket | 330 kB, 4221 obj. | 5.3 ms | 10.8 ms | 0.5x |
| blankmap | 1.0 MB, 2232 obj. | 22.1 ms | 6.3 ms | 3.5x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 12.3 ms | 10.4 ms | 1.2x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 12.7 ms | 8.8 ms | 1.5x |
| greece | 3.2 MB, 2013 obj. | 13.1 ms | 85.0 ms | 0.2x |
| clifford-attractor | 13.2 MB, 250001 obj. | 74.4 ms | 181 ms | 0.4x |
| cube8 | 14.6 MB, 190260 obj. | 113 ms | 621 ms | 0.2x |

#### Move: one step of the drag (median of 30)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 0.4 ms | 0.1 ms | 4.2x |
| feynman (small) | 4 kB, 34 obj. | 7.4 ms | 0.4 ms | 18x |
| fourier (small) | 16 kB, 41 obj. | 0.4 ms | 0.4 ms | 1.1x |
| pines | 11 kB, 31 obj. | 0.5 ms | 0.2 ms | 2.2x |
| snowy | 173 kB, 436 obj. | 0.2 ms | 0.4 ms | 0.5x |
| wenig | 587 kB, 85 obj. | 1.6 ms | 0.3 ms | 4.7x |
| gosper-curve | 182 kB, 142 obj. | 0.4 ms | 0.4 ms | 1.0x |
| penrose-tiling | 560 kB, 7615 obj. | 0.3 ms | 0.5 ms | 0.7x |
| apollonian-gasket | 330 kB, 4221 obj. | 0.6 ms | 0.5 ms | 1.3x |
| blankmap | 1.0 MB, 2232 obj. | 0.9 ms | 0.1 ms | 7.2x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.5 ms | 0.4 ms | 3.6x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 0.5 ms | 0.3 ms | 1.6x |
| greece | 3.2 MB, 2013 obj. | 0.4 ms | 0.4 ms | 1.0x |
| clifford-attractor | 13.2 MB, 250001 obj. | 0.3 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 0.3 ms | 1.5 ms | 0.2x |

#### Move: place (final redraw)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.6 ms | 10.4 ms | 0.2x |
| feynman (small) | 4 kB, 34 obj. | 49.5 ms | 15.9 ms | 3.1x |
| fourier (small) | 16 kB, 41 obj. | 22.1 ms | 23.0 ms | 1.0x |
| pines | 11 kB, 31 obj. | 4.0 ms | 7.8 ms | 0.5x |
| snowy | 173 kB, 436 obj. | 55.4 ms | 20.8 ms | 2.7x |
| wenig | 587 kB, 85 obj. | 118 ms | 10.6 ms | 11x |
| gosper-curve | 182 kB, 142 obj. | 36.8 ms | 59.7 ms | 0.6x |
| penrose-tiling | 560 kB, 7615 obj. | 83.1 ms | 26.4 ms | 3.1x |
| apollonian-gasket | 330 kB, 4221 obj. | 14.6 ms | 38.1 ms | 0.4x |
| blankmap | 1.0 MB, 2232 obj. | 90.3 ms | 21.0 ms | 4.3x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.20 s | 28.2 ms | 42x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 524 ms | 31.2 ms | 17x |
| greece | 3.2 MB, 2013 obj. | 910 ms | 113 ms | 8.1x |
| clifford-attractor | 13.2 MB, 250001 obj. | 225 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 376 ms | 1.47 s | 0.3x |

#### Save

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 0.6 ms | 5.0 ms | 0.1x |
| feynman (small) | 4 kB, 34 obj. | 1.1 ms | 12.9 ms | 0.1x |
| fourier (small) | 16 kB, 41 obj. | 5.5 ms | 10.3 ms | 0.5x |
| pines | 11 kB, 31 obj. | 0.6 ms | 9.6 ms | 0.1x |
| snowy | 173 kB, 436 obj. | 3.2 ms | 13.2 ms | 0.2x |
| wenig | 587 kB, 85 obj. | 9.8 ms | 16.1 ms | 0.6x |
| gosper-curve | 182 kB, 142 obj. | 4.5 ms | 24.8 ms | 0.2x |
| penrose-tiling | 560 kB, 7615 obj. | 11.5 ms | 14.5 ms | 0.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 4.0 ms | 10.3 ms | 0.4x |
| blankmap | 1.0 MB, 2232 obj. | 16.3 ms | 13.9 ms | 1.2x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 63.7 ms | 35.9 ms | 1.8x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 40.9 ms | 22.2 ms | 1.8x |
| greece | 3.2 MB, 2013 obj. | 41.9 ms | 25.2 ms | 1.7x |
| clifford-attractor | 13.2 MB, 250001 obj. | 108 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 110 ms | 205 ms | 0.5x |

#### Undo the move

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 3.9 ms | 11.5 ms | 0.3x |
| feynman (small) | 4 kB, 34 obj. | 3.6 ms | 15.4 ms | 0.2x |
| fourier (small) | 16 kB, 41 obj. | 1.6 ms | 32.6 ms | 0.0x |
| pines | 11 kB, 31 obj. | 7.5 ms | 12.4 ms | 0.6x |
| snowy | 173 kB, 436 obj. | 5.9 ms | 32.4 ms | 0.2x |
| wenig | 587 kB, 85 obj. | 113 ms | 22.9 ms | 4.9x |
| gosper-curve | 182 kB, 142 obj. | 51.1 ms | 64.3 ms | 0.8x |
| penrose-tiling | 560 kB, 7615 obj. | 63.8 ms | 32.2 ms | 2.0x |
| apollonian-gasket | 330 kB, 4221 obj. | 27.2 ms | 36.3 ms | 0.7x |
| blankmap | 1.0 MB, 2232 obj. | 87.1 ms | 21.3 ms | 4.1x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 210 ms | 37.5 ms | 5.6x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 69.4 ms | 39.5 ms | 1.8x |
| greece | 3.2 MB, 2013 obj. | 93.6 ms | 137 ms | 0.7x |
| clifford-attractor | 13.2 MB, 250001 obj. | 43.4 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 73.9 ms | 1.86 s | 0.0x |

#### Cross-check: the same actions timed from outside

xfig: its own hooks against watching the canvas pixels; RustFig: its frame times against the wall time until `ui.state` reports the canvas current (never below one frame, 16.7 ms).

| Figure | Task | xfig, own hooks | xfig, pixels | RustFig, frame times | RustFig, wall to idle |
|---|---|---|---|---|---|
| carnot (small) | open | 1.17 s | 1.29 s | 191 ms | 191 ms |
| carnot (small) | zoom_in | 9.5 ms | 29.8 ms | 9.6 ms | 24.0 ms |
| carnot (small) | zoom_out | 2.4 ms | 28.1 ms | 11.6 ms | 25.5 ms |
| carnot (small) | pan | 1.8 ms | 5.0 ms | 11.8 ms | 22.4 ms |
| carnot (small) | move_step | 0.4 ms | 7.2 ms | 0.1 ms | 66.5 ms |
| carnot (small) | move_place | 1.6 ms | 49.4 ms | 10.4 ms | 83.4 ms |
| carnot (small) | save | 0.6 ms | 29.5 ms | 5.0 ms | n/a |
| carnot (small) | undo | 3.9 ms | 33.6 ms | 11.5 ms | 25.1 ms |
| feynman (small) | open | 1.14 s | 1.16 s | 181 ms | 181 ms |
| feynman (small) | zoom_in | 21.7 ms | 29.8 ms | 8.2 ms | 23.0 ms |
| feynman (small) | zoom_out | 4.6 ms | 31.0 ms | 9.3 ms | 26.6 ms |
| feynman (small) | pan | 2.9 ms | 7.6 ms | 10.2 ms | 26.7 ms |
| feynman (small) | move_step | 7.4 ms | 7.4 ms | 0.4 ms | 50.1 ms |
| feynman (small) | move_place | 49.5 ms | 49.5 ms | 15.9 ms | 78.3 ms |
| feynman (small) | save | 1.1 ms | 31.1 ms | 12.9 ms | n/a |
| feynman (small) | undo | 3.6 ms | 33.0 ms | 15.4 ms | 32.9 ms |
| fourier (small) | open | 1.41 s | 1.44 s | 456 ms | 456 ms |
| fourier (small) | zoom_in | 21.4 ms | 44.8 ms | 20.1 ms | 32.3 ms |
| fourier (small) | zoom_out | 4.0 ms | 39.2 ms | 20.0 ms | 28.7 ms |
| fourier (small) | pan | 28.7 ms | 22.1 ms | 21.0 ms | 32.7 ms |
| fourier (small) | move_step | 0.4 ms | 22.7 ms | 0.4 ms | 50.1 ms |
| fourier (small) | move_place | 22.1 ms | 64.2 ms | 23.0 ms | 83.5 ms |
| fourier (small) | save | 5.5 ms | 47.8 ms | 10.3 ms | n/a |
| fourier (small) | undo | 1.6 ms | 45.9 ms | 32.6 ms | 44.8 ms |
| pines | open | 1.39 s | 1.42 s | 332 ms | 332 ms |
| pines | zoom_in | 10.0 ms | 42.8 ms | 5.8 ms | 20.6 ms |
| pines | zoom_out | 7.5 ms | 42.6 ms | 5.9 ms | 16.7 ms |
| pines | pan | 2.8 ms | 9.9 ms | 9.4 ms | 24.9 ms |
| pines | move_step | 0.5 ms | 6.4 ms | 0.2 ms | 50.0 ms |
| pines | move_place | 4.0 ms | 49.6 ms | 7.8 ms | 78.8 ms |
| pines | save | 0.6 ms | 30.0 ms | 9.6 ms | n/a |
| pines | undo | 7.5 ms | 30.4 ms | 12.4 ms | 26.1 ms |
| snowy | open | 1.27 s | 1.31 s | 387 ms | 387 ms |
| snowy | zoom_in | 6.1 ms | 45.5 ms | 10.6 ms | 21.8 ms |
| snowy | zoom_out | 7.9 ms | 29.5 ms | 10.6 ms | 33.5 ms |
| snowy | pan | 5.5 ms | 12.6 ms | 15.4 ms | 24.8 ms |
| snowy | move_step | 0.2 ms | 16.5 ms | 0.4 ms | 50.0 ms |
| snowy | move_place | 55.4 ms | 67.8 ms | 20.8 ms | 83.3 ms |
| snowy | save | 3.2 ms | 50.5 ms | 13.2 ms | n/a |
| snowy | undo | 5.9 ms | 48.9 ms | 32.4 ms | 38.5 ms |
| wenig | open | 1.31 s | 1.35 s | 411 ms | 411 ms |
| wenig | zoom_in | 24.9 ms | 32.2 ms | 28.3 ms | 50.1 ms |
| wenig | zoom_out | 20.1 ms | 48.7 ms | 38.9 ms | 63.9 ms |
| wenig | pan | 22.3 ms | 24.4 ms | 34.8 ms | 45.7 ms |
| wenig | move_step | 1.6 ms | 19.9 ms | 0.3 ms | 51.0 ms |
| wenig | move_place | 118 ms | 121 ms | 10.6 ms | 132 ms |
| wenig | save | 9.8 ms | 34.6 ms | 16.1 ms | n/a |
| wenig | undo | 113 ms | 115 ms | 22.9 ms | 91.0 ms |
| gosper-curve | open | 1.33 s | 1.37 s | 517 ms | 517 ms |
| gosper-curve | zoom_in | 7.3 ms | 33.2 ms | 107 ms | 115 ms |
| gosper-curve | zoom_out | 6.1 ms | 33.2 ms | 74.8 ms | 90.2 ms |
| gosper-curve | pan | 5.3 ms | 50.3 ms | 66.4 ms | 72.0 ms |
| gosper-curve | move_step | 0.4 ms | 22.1 ms | 0.4 ms | 52.5 ms |
| gosper-curve | move_place | 36.8 ms | 66.2 ms | 59.7 ms | 99.9 ms |
| gosper-curve | save | 4.5 ms | 44.9 ms | 24.8 ms | n/a |
| gosper-curve | undo | 51.1 ms | 66.6 ms | 64.3 ms | 101 ms |
| penrose-tiling | open | 1.45 s | 1.53 s | 547 ms | 547 ms |
| penrose-tiling | zoom_in | 38.1 ms | 70.6 ms | 31.6 ms | 38.8 ms |
| penrose-tiling | zoom_out | 121 ms | 133 ms | 13.0 ms | 40.3 ms |
| penrose-tiling | pan | 108 ms | 113 ms | 16.9 ms | 33.5 ms |
| penrose-tiling | move_step | 0.3 ms | 22.8 ms | 0.5 ms | 50.1 ms |
| penrose-tiling | move_place | 83.1 ms | 93.2 ms | 26.4 ms | 101 ms |
| penrose-tiling | save | 11.5 ms | 55.0 ms | 14.5 ms | n/a |
| penrose-tiling | undo | 63.8 ms | 109 ms | 32.2 ms | 50.2 ms |
| apollonian-gasket | open | 1.13 s | 1.15 s | 412 ms | 412 ms |
| apollonian-gasket | zoom_in | 11.6 ms | 29.5 ms | 28.3 ms | 37.7 ms |
| apollonian-gasket | zoom_out | 12.9 ms | 29.5 ms | 22.1 ms | 33.1 ms |
| apollonian-gasket | pan | 5.3 ms | 7.4 ms | 10.8 ms | 25.2 ms |
| apollonian-gasket | move_step | 0.6 ms | 5.7 ms | 0.5 ms | 54.9 ms |
| apollonian-gasket | move_place | 14.6 ms | 49.1 ms | 38.1 ms | 111 ms |
| apollonian-gasket | save | 4.0 ms | 29.6 ms | 10.3 ms | n/a |
| apollonian-gasket | undo | 27.2 ms | 28.7 ms | 36.3 ms | 60.0 ms |
| blankmap | open | 1.28 s | 1.36 s | 228 ms | 228 ms |
| blankmap | zoom_in | 19.2 ms | 27.2 ms | 17.8 ms | 35.5 ms |
| blankmap | zoom_out | 11.9 ms | 30.9 ms | 14.7 ms | 28.5 ms |
| blankmap | pan | 22.1 ms | 23.6 ms | 6.3 ms | 21.0 ms |
| blankmap | move_step | 0.9 ms | 6.2 ms | 0.1 ms | 54.3 ms |
| blankmap | move_place | 90.3 ms | 92.5 ms | 21.0 ms | 83.2 ms |
| blankmap | save | 16.3 ms | 30.8 ms | 13.9 ms | n/a |
| blankmap | undo | 87.1 ms | 88.6 ms | 21.3 ms | 38.9 ms |
| ne-coastline-10m | open | 1.17 s | 1.19 s | 452 ms | 452 ms |
| ne-coastline-10m | zoom_in | 10.6 ms | 31.6 ms | 30.8 ms | 42.7 ms |
| ne-coastline-10m | zoom_out | 9.3 ms | 27.6 ms | 17.7 ms | 62.0 ms |
| ne-coastline-10m | pan | 12.3 ms | 12.0 ms | 10.4 ms | 33.7 ms |
| ne-coastline-10m | move_step | 1.5 ms | 7.5 ms | 0.4 ms | 53.4 ms |
| ne-coastline-10m | move_place | 1.20 s | 1.20 s | 28.2 ms | 88.5 ms |
| ne-coastline-10m | save | 63.7 ms | 74.1 ms | 35.9 ms | n/a |
| ne-coastline-10m | undo | 210 ms | 220 ms | 37.5 ms | 61.9 ms |
| ne-countries-10m | open | 1.22 s | 1.23 s | 348 ms | 348 ms |
| ne-countries-10m | zoom_in | 11.4 ms | 29.7 ms | 26.9 ms | 39.1 ms |
| ne-countries-10m | zoom_out | 10.9 ms | 27.2 ms | 17.8 ms | 33.4 ms |
| ne-countries-10m | pan | 12.7 ms | 14.6 ms | 8.8 ms | 23.9 ms |
| ne-countries-10m | move_step | 0.5 ms | 4.7 ms | 0.3 ms | 50.0 ms |
| ne-countries-10m | move_place | 524 ms | 524 ms | 31.2 ms | 100 ms |
| ne-countries-10m | save | 40.9 ms | 42.4 ms | 22.2 ms | n/a |
| ne-countries-10m | undo | 69.4 ms | 72.1 ms | 39.5 ms | 54.4 ms |
| greece | open | 1.21 s | 1.22 s | 958 ms | 958 ms |
| greece | zoom_in | 14.2 ms | 29.8 ms | 131 ms | 146 ms |
| greece | zoom_out | 8.2 ms | 29.3 ms | 142 ms | 161 ms |
| greece | pan | 13.1 ms | 16.6 ms | 85.0 ms | 98.6 ms |
| greece | move_step | 0.4 ms | 22.0 ms | 0.4 ms | 50.8 ms |
| greece | move_place | 910 ms | 913 ms | 113 ms | 186 ms |
| greece | save | 41.9 ms | 44.2 ms | 25.2 ms | n/a |
| greece | undo | 93.6 ms | 105 ms | 137 ms | 150 ms |
| clifford-attractor | open | 1.33 s | 1.36 s | 1.27 s | 1.27 s |
| clifford-attractor | zoom_in | 136 ms | 138 ms | 394 ms | 283 ms |
| clifford-attractor | zoom_out | 155 ms | 156 ms | 205 ms | 187 ms |
| clifford-attractor | pan | 74.4 ms | 74.0 ms | 181 ms | 144 ms |
| clifford-attractor | move_step | 0.3 ms | 6.3 ms | n/a | n/a |
| clifford-attractor | move_place | 225 ms | 227 ms | n/a | n/a |
| clifford-attractor | save | 108 ms | 111 ms | n/a | n/a |
| clifford-attractor | undo | 43.4 ms | 45.1 ms | n/a | n/a |
| cube8 | open | 1.34 s | 1.36 s | 4.33 s | 4.33 s |
| cube8 | zoom_in | 118 ms | 121 ms | 1.69 s | 1.37 s |
| cube8 | zoom_out | 135 ms | 135 ms | 1.18 s | 966 ms |
| cube8 | pan | 113 ms | 114 ms | 621 ms | 518 ms |
| cube8 | move_step | 0.3 ms | 5.2 ms | 1.5 ms | 50.1 ms |
| cube8 | move_place | 376 ms | 377 ms | 1.47 s | 1.53 s |
| cube8 | save | 110 ms | 112 ms | 205 ms | n/a |
| cube8 | undo | 73.9 ms | 75.2 ms | 1.86 s | 1.61 s |

<details><summary>Raw numbers (every run, milliseconds)</summary>

| Figure | App | Task | Runs |
|---|---|---|---|
| carnot (small) | xfig | open | 1168.8; 1337.8; 1125.7 |
| carnot (small) | xfig | open_read | 33.1; 17.8; 18.8 |
| carnot (small) | xfig | open_draw | 1.7; 1.6; 1.5 |
| carnot (small) | xfig | zoom_in | 9.1; 9.5; 10.0 |
| carnot (small) | xfig | zoom_out | 2.6; 2.3; 2.4 |
| carnot (small) | xfig | pan | 2.0; 1.4; 1.8 |
| carnot (small) | xfig | move_step | 0.3; 0.4; 0.7 |
| carnot (small) | xfig | move_step_max | 0.5; 16.1; 1.1 |
| carnot (small) | xfig | move_place | 1.3; 1.6; 3.0 |
| carnot (small) | xfig | save | 0.6; 0.6; 2.2 |
| carnot (small) | xfig | undo | 26.5; 1.3; 3.9 |
| carnot (small) | xfig | x_open | 1294.0; 1367.6; 1140.7 |
| carnot (small) | xfig | x_zoom_in | 29.6; 29.8; 29.9 |
| carnot (small) | xfig | x_zoom_out | 29.9; 26.9; 28.1 |
| carnot (small) | xfig | x_pan | 5.2; 4.6; 5.0 |
| carnot (small) | xfig | x_move_step | 4.5; 13.2; 7.2 |
| carnot (small) | xfig | x_move_place | 49.4; 45.8; 51.5 |
| carnot (small) | xfig | x_save | 29.5; 29.5; 31.7 |
| carnot (small) | xfig | x_undo | 93.4; 29.7; 33.6 |
| carnot (small) | RustFig | open | 213.1; 190.6; 174.0 |
| carnot (small) | RustFig | open_read | -; -; - |
| carnot (small) | RustFig | open_draw | -; -; - |
| carnot (small) | RustFig | zoom_in | 8.8; 10.7; 9.6 |
| carnot (small) | RustFig | zoom_out | 11.2; 18.5; 11.6 |
| carnot (small) | RustFig | pan | 11.3; 36.5; 11.8 |
| carnot (small) | RustFig | move_step | 0.1; 0.3; 0.1 |
| carnot (small) | RustFig | move_step_max | 0.2; 0.4; 0.2 |
| carnot (small) | RustFig | move_place | 10.4; 10.3; 11.5 |
| carnot (small) | RustFig | save | 4.4; 13.3; 5.0 |
| carnot (small) | RustFig | undo | 11.5; 10.3; 11.8 |
| carnot (small) | RustFig | x_open | 213.1; 190.6; 174.0 |
| carnot (small) | RustFig | x_zoom_in | 23.8; 60.9; 24.0 |
| carnot (small) | RustFig | x_zoom_out | 25.4; 27.6; 25.5 |
| carnot (small) | RustFig | x_pan | 22.4; 34.8; 16.8 |
| carnot (small) | RustFig | x_move_step | 66.5; 50.0; 66.7 |
| carnot (small) | RustFig | x_move_place | 83.5; 83.4; 83.3 |
| carnot (small) | RustFig | x_save | -; -; - |
| carnot (small) | RustFig | x_undo | 27.6; 24.7; 25.1 |
| feynman (small) | xfig | open | 1136.6; 1128.5; 1142.5 |
| feynman (small) | xfig | open_read | 25.4; 24.3; 17.3 |
| feynman (small) | xfig | open_draw | 1.1; 1.1; 1.2 |
| feynman (small) | xfig | zoom_in | 22.3; 21.7; 13.6 |
| feynman (small) | xfig | zoom_out | 4.6; 4.8; 2.0 |
| feynman (small) | xfig | pan | 2.9; 3.7; 1.3 |
| feynman (small) | xfig | move_step | 7.5; 7.4; 4.5 |
| feynman (small) | xfig | move_step_max | 8.3; 8.7; 4.8 |
| feynman (small) | xfig | move_place | 49.7; 48.6; 49.5 |
| feynman (small) | xfig | save | 1.9; 1.1; 0.6 |
| feynman (small) | xfig | undo | 4.0; 3.6; 1.7 |
| feynman (small) | xfig | x_open | 1164.0; 1153.8; 1158.7 |
| feynman (small) | xfig | x_zoom_in | 29.8; 30.0; 29.4 |
| feynman (small) | xfig | x_zoom_out | 31.0; 31.0; 25.5 |
| feynman (small) | xfig | x_pan | 7.6; 8.6; 4.5 |
| feynman (small) | xfig | x_move_step | 7.5; 7.4; 4.5 |
| feynman (small) | xfig | x_move_place | 49.7; 48.6; 49.5 |
| feynman (small) | xfig | x_save | 34.1; 31.1; 29.5 |
| feynman (small) | xfig | x_undo | 33.0; 33.0; 29.4 |
| feynman (small) | RustFig | open | 174.2; 180.8; 383.8 |
| feynman (small) | RustFig | open_read | -; -; - |
| feynman (small) | RustFig | open_draw | -; -; - |
| feynman (small) | RustFig | zoom_in | 7.8; 8.2; 12.1 |
| feynman (small) | RustFig | zoom_out | 8.4; 9.3; 21.0 |
| feynman (small) | RustFig | pan | 5.9; 10.2; 24.0 |
| feynman (small) | RustFig | move_step | 0.1; 0.4; 0.4 |
| feynman (small) | RustFig | move_step_max | 0.2; 10.5; 10.5 |
| feynman (small) | RustFig | move_place | 7.7; 21.8; 15.9 |
| feynman (small) | RustFig | save | 12.9; 5.5; 15.9 |
| feynman (small) | RustFig | undo | 7.5; 27.8; 15.4 |
| feynman (small) | RustFig | x_open | 174.2; 180.8; 383.8 |
| feynman (small) | RustFig | x_zoom_in | 22.0; 23.0; 25.6 |
| feynman (small) | RustFig | x_zoom_out | 22.1; 26.6; 32.2 |
| feynman (small) | RustFig | x_pan | 20.0; 39.4; 26.7 |
| feynman (small) | RustFig | x_move_step | 50.2; 50.1; 50.1 |
| feynman (small) | RustFig | x_move_place | 78.3; 83.4; 78.3 |
| feynman (small) | RustFig | x_save | -; -; - |
| feynman (small) | RustFig | x_undo | 17.7; 44.6; 32.9 |
| fourier (small) | xfig | open | 1407.7; 1577.2; 1367.0 |
| fourier (small) | xfig | open_read | 60.2; 102.5; 60.4 |
| fourier (small) | xfig | open_draw | 2.5; 2.4; 16.9 |
| fourier (small) | xfig | zoom_in | 20.1; 21.4; 49.7 |
| fourier (small) | xfig | zoom_out | 3.8; 4.0; 17.6 |
| fourier (small) | xfig | pan | 3.3; 42.3; 28.7 |
| fourier (small) | xfig | move_step | 0.5; 0.4; 0.4 |
| fourier (small) | xfig | move_step_max | 7.8; 12.1; 7.6 |
| fourier (small) | xfig | move_place | 22.1; 29.4; 2.4 |
| fourier (small) | xfig | save | 5.5; 1.6; 11.2 |
| fourier (small) | xfig | undo | 1.6; 1.5; 37.5 |
| fourier (small) | xfig | x_open | 1437.0; 1619.8; 1406.9 |
| fourier (small) | xfig | x_zoom_in | 44.8; 43.3; 145.7 |
| fourier (small) | xfig | x_zoom_out | 47.2; 36.6; 39.2 |
| fourier (small) | xfig | x_pan | 22.1; 45.3; 21.1 |
| fourier (small) | xfig | x_move_step | 17.3; 23.3; 22.7 |
| fourier (small) | xfig | x_move_place | 87.6; 61.6; 64.2 |
| fourier (small) | xfig | x_save | 47.8; 42.4; 117.3 |
| fourier (small) | xfig | x_undo | 39.0; 45.9; 54.8 |
| fourier (small) | RustFig | open | 456.2; 440.7; 615.1 |
| fourier (small) | RustFig | open_read | -; -; - |
| fourier (small) | RustFig | open_draw | -; -; - |
| fourier (small) | RustFig | zoom_in | 20.1; 20.1; 20.2 |
| fourier (small) | RustFig | zoom_out | 19.6; 20.0; 20.8 |
| fourier (small) | RustFig | pan | 21.0; 29.5; 16.7 |
| fourier (small) | RustFig | move_step | 0.4; 0.4; 0.4 |
| fourier (small) | RustFig | move_step_max | 0.5; 0.7; 10.5 |
| fourier (small) | RustFig | move_place | 21.1; 23.0; 29.0 |
| fourier (small) | RustFig | save | 14.5; 10.3; 6.8 |
| fourier (small) | RustFig | undo | 32.6; 27.3; 32.9 |
| fourier (small) | RustFig | x_open | 456.2; 440.7; 615.1 |
| fourier (small) | RustFig | x_zoom_in | 28.8; 32.3; 35.1 |
| fourier (small) | RustFig | x_zoom_out | 28.7; 28.1; 29.5 |
| fourier (small) | RustFig | x_pan | 32.7; 32.9; 29.3 |
| fourier (small) | RustFig | x_move_step | 50.1; 54.3; 50.0 |
| fourier (small) | RustFig | x_move_place | 87.1; 83.4; 83.5 |
| fourier (small) | RustFig | x_save | -; -; - |
| fourier (small) | RustFig | x_undo | 44.8; 47.1; 40.5 |
| pines | xfig | open | 1567.3; 1388.8; 1122.4 |
| pines | xfig | open_read | 2.8; 1.6; 0.9 |
| pines | xfig | open_draw | 5.6; 8.6; 3.2 |
| pines | xfig | zoom_in | 11.2; 10.0; 5.8 |
| pines | xfig | zoom_out | 10.0; 7.5; 5.0 |
| pines | xfig | pan | 2.8; 3.2; 2.1 |
| pines | xfig | move_step | 0.5; 0.5; 0.4 |
| pines | xfig | move_step_max | 3.6; 4.2; 0.5 |
| pines | xfig | move_place | 7.1; 3.7; 4.0 |
| pines | xfig | save | 1.4; 0.6; 0.6 |
| pines | xfig | undo | 8.4; 3.9; 7.5 |
| pines | xfig | x_open | 1663.5; 1420.7; 1136.7 |
| pines | xfig | x_zoom_in | 42.8; 46.3; 30.2 |
| pines | xfig | x_zoom_out | 42.6; 45.1; 30.0 |
| pines | xfig | x_pan | 9.9; 10.6; 6.3 |
| pines | xfig | x_move_step | 24.7; 6.4; 4.8 |
| pines | xfig | x_move_place | 67.8; 49.6; 49.2 |
| pines | xfig | x_save | 47.0; 29.6; 30.0 |
| pines | xfig | x_undo | 36.9; 29.5; 30.4 |
| pines | RustFig | open | 207.7; 357.4; 331.7 |
| pines | RustFig | open_read | -; -; - |
| pines | RustFig | open_draw | -; -; - |
| pines | RustFig | zoom_in | 5.8; 5.1; 5.8 |
| pines | RustFig | zoom_out | 5.9; 7.0; 5.9 |
| pines | RustFig | pan | 6.9; 11.9; 9.4 |
| pines | RustFig | move_step | 0.2; 0.3; 0.2 |
| pines | RustFig | move_step_max | 0.3; 0.5; 0.4 |
| pines | RustFig | move_place | 7.8; 12.0; 7.6 |
| pines | RustFig | save | 9.6; 13.7; 5.4 |
| pines | RustFig | undo | 14.4; 12.4; 6.0 |
| pines | RustFig | x_open | 207.7; 357.4; 331.7 |
| pines | RustFig | x_zoom_in | 20.6; 16.7; 20.6 |
| pines | RustFig | x_zoom_out | 16.7; 16.8; 16.7 |
| pines | RustFig | x_pan | 22.0; 28.1; 24.9 |
| pines | RustFig | x_move_step | 50.0; 50.3; 50.0 |
| pines | RustFig | x_move_place | 106.7; 78.4; 78.8 |
| pines | RustFig | x_save | -; -; - |
| pines | RustFig | x_undo | 26.1; 35.1; 17.8 |
| snowy | xfig | open | 1117.0; 1273.9; 1297.4 |
| snowy | xfig | open_read | 4.3; 4.8; 14.8 |
| snowy | xfig | open_draw | 3.7; 5.1; 9.8 |
| snowy | xfig | zoom_in | 2.8; 6.1; 33.5 |
| snowy | xfig | zoom_out | 3.3; 9.5; 7.9 |
| snowy | xfig | pan | 7.0; 5.4; 5.5 |
| snowy | xfig | move_step | 0.3; 0.2; 0.2 |
| snowy | xfig | move_step_max | 10.7; 7.5; 0.3 |
| snowy | xfig | move_place | 55.4; 81.4; 24.3 |
| snowy | xfig | save | 12.7; 3.2; 3.2 |
| snowy | xfig | undo | 106.1; 5.8; 5.9 |
| snowy | xfig | x_open | 1142.5; 1306.8; 1328.3 |
| snowy | xfig | x_zoom_in | 30.7; 45.5; 55.0 |
| snowy | xfig | x_zoom_out | 29.5; 49.0; 28.7 |
| snowy | xfig | x_pan | 6.7; 12.6; 23.3 |
| snowy | xfig | x_move_step | 13.9; 16.5; 23.1 |
| snowy | xfig | x_move_place | 67.8; 82.6; 61.0 |
| snowy | xfig | x_save | 38.8; 51.2; 50.5 |
| snowy | xfig | x_undo | 119.8; 39.6; 48.9 |
| snowy | RustFig | open | 402.4; 376.9; 386.7 |
| snowy | RustFig | open_read | -; -; - |
| snowy | RustFig | open_draw | -; -; - |
| snowy | RustFig | zoom_in | 10.6; 10.6; 15.5 |
| snowy | RustFig | zoom_out | 10.6; 10.6; 10.6 |
| snowy | RustFig | pan | 10.6; 15.4; 15.5 |
| snowy | RustFig | move_step | 0.4; 0.4; 0.4 |
| snowy | RustFig | move_step_max | 10.5; 10.4; 10.5 |
| snowy | RustFig | move_place | 19.9; 21.6; 20.8 |
| snowy | RustFig | save | 7.4; 13.4; 13.2 |
| snowy | RustFig | undo | 20.4; 32.4; 32.8 |
| snowy | RustFig | x_open | 402.4; 376.9; 386.7 |
| snowy | RustFig | x_zoom_in | 21.8; 16.7; 32.9 |
| snowy | RustFig | x_zoom_out | 33.5; 33.5; 22.7 |
| snowy | RustFig | x_pan | 21.6; 28.2; 24.8 |
| snowy | RustFig | x_move_step | 50.0; 50.0; 50.0 |
| snowy | RustFig | x_move_place | 83.3; 73.2; 83.7 |
| snowy | RustFig | x_save | -; -; - |
| snowy | RustFig | x_undo | 27.2; 38.9; 38.5 |
| wenig | xfig | open | 1293.8; 1315.2; 1305.6 |
| wenig | xfig | open_read | 13.3; 13.7; 13.6 |
| wenig | xfig | open_draw | 19.8; 20.1; 24.2 |
| wenig | xfig | zoom_in | 24.9; 26.3; 20.7 |
| wenig | xfig | zoom_out | 19.1; 24.9; 20.1 |
| wenig | xfig | pan | 24.5; 20.4; 22.3 |
| wenig | xfig | move_step | 1.6; 1.6; 1.8 |
| wenig | xfig | move_step_max | 2.1; 2.0; 2.0 |
| wenig | xfig | move_place | 117.5; 126.8; 111.8 |
| wenig | xfig | save | 9.8; 10.3; 8.8 |
| wenig | xfig | undo | 112.0; 115.8; 113.2 |
| wenig | xfig | x_open | 1326.2; 1347.8; 1348.3 |
| wenig | xfig | x_zoom_in | 32.2; 32.0; 47.6 |
| wenig | xfig | x_zoom_out | 53.9; 48.1; 48.7 |
| wenig | xfig | x_pan | 25.1; 24.4; 23.6 |
| wenig | xfig | x_move_step | 19.0; 20.9; 19.9 |
| wenig | xfig | x_move_place | 120.6; 129.3; 112.2 |
| wenig | xfig | x_save | 45.5; 34.6; 34.6 |
| wenig | xfig | x_undo | 112.9; 115.8; 115.0 |
| wenig | RustFig | open | 410.7; 259.6; 541.1 |
| wenig | RustFig | open_read | -; -; - |
| wenig | RustFig | open_draw | -; -; - |
| wenig | RustFig | zoom_in | 28.3; 21.9; 32.5 |
| wenig | RustFig | zoom_out | 38.9; 23.2; 45.3 |
| wenig | RustFig | pan | 22.2; 34.8; 50.0 |
| wenig | RustFig | move_step | 0.2; 0.4; 0.3 |
| wenig | RustFig | move_step_max | 0.4; 38.0; 10.4 |
| wenig | RustFig | move_place | 1.2; 10.6; 10.6 |
| wenig | RustFig | save | 13.2; 16.1; 20.5 |
| wenig | RustFig | undo | 22.9; 12.5; 48.1 |
| wenig | RustFig | x_open | 410.7; 259.6; 541.1 |
| wenig | RustFig | x_zoom_in | 50.1; 49.8; 65.5 |
| wenig | RustFig | x_zoom_out | 68.3; 50.1; 63.9 |
| wenig | RustFig | x_pan | 45.7; 38.0; 60.0 |
| wenig | RustFig | x_move_step | 50.0; 58.3; 51.0 |
| wenig | RustFig | x_move_place | 83.3; 131.8; 132.1 |
| wenig | RustFig | x_save | -; -; - |
| wenig | RustFig | x_undo | 48.9; 91.0; 95.7 |
| gosper-curve | xfig | open | 1357.8; 1227.9; 1327.8 |
| gosper-curve | xfig | open_read | 5.7; 5.1; 5.3 |
| gosper-curve | xfig | open_draw | 6.1; 5.5; 4.7 |
| gosper-curve | xfig | zoom_in | 7.3; 7.1; 7.5 |
| gosper-curve | xfig | zoom_out | 6.6; 4.5; 6.1 |
| gosper-curve | xfig | pan | 3.2; 5.3; 28.1 |
| gosper-curve | xfig | move_step | 0.4; 0.4; 0.4 |
| gosper-curve | xfig | move_step_max | 5.6; 7.3; 9.6 |
| gosper-curve | xfig | move_place | 52.9; 19.4; 36.8 |
| gosper-curve | xfig | save | 3.4; 4.5; 5.3 |
| gosper-curve | xfig | undo | 6.9; 53.1; 51.1 |
| gosper-curve | xfig | x_open | 1391.1; 1251.9; 1368.7 |
| gosper-curve | xfig | x_zoom_in | 33.2; 26.4; 67.0 |
| gosper-curve | xfig | x_zoom_out | 33.2; 30.6; 77.4 |
| gosper-curve | xfig | x_pan | 50.3; 6.1; 50.7 |
| gosper-curve | xfig | x_move_step | 22.1; 6.3; 23.6 |
| gosper-curve | xfig | x_move_place | 66.2; 49.9; 71.5 |
| gosper-curve | xfig | x_save | 43.6; 44.9; 45.9 |
| gosper-curve | xfig | x_undo | 32.3; 66.6; 76.5 |
| gosper-curve | RustFig | open | 510.2; 517.0; 530.9 |
| gosper-curve | RustFig | open_read | -; -; - |
| gosper-curve | RustFig | open_draw | -; -; - |
| gosper-curve | RustFig | zoom_in | 80.9; 106.9; 112.7 |
| gosper-curve | RustFig | zoom_out | 74.8; 97.2; 68.2 |
| gosper-curve | RustFig | pan | 66.4; 68.5; 62.7 |
| gosper-curve | RustFig | move_step | 0.4; 0.4; 0.4 |
| gosper-curve | RustFig | move_step_max | 53.1; 53.2; 51.0 |
| gosper-curve | RustFig | move_place | 93.2; 59.7; 10.7 |
| gosper-curve | RustFig | save | 19.0; 28.2; 24.8 |
| gosper-curve | RustFig | undo | 73.9; 63.2; 64.3 |
| gosper-curve | RustFig | x_open | 510.2; 517.0; 530.9 |
| gosper-curve | RustFig | x_zoom_in | 90.2; 115.4; 142.1 |
| gosper-curve | RustFig | x_zoom_out | 90.2; 112.7; 82.6 |
| gosper-curve | RustFig | x_pan | 72.0; 71.9; 72.1 |
| gosper-curve | RustFig | x_move_step | 52.5; 61.6; 52.4 |
| gosper-curve | RustFig | x_move_place | 129.9; 99.5; 99.9 |
| gosper-curve | RustFig | x_save | -; -; - |
| gosper-curve | RustFig | x_undo | 110.7; 101.2; 80.3 |
| penrose-tiling | xfig | open | 1602.9; 1444.1; 1449.3 |
| penrose-tiling | xfig | open_read | 12.3; 12.5; 21.4 |
| penrose-tiling | xfig | open_draw | 36.9; 43.5; 46.8 |
| penrose-tiling | xfig | zoom_in | 38.8; 32.3; 38.1 |
| penrose-tiling | xfig | zoom_out | 44.8; 120.6; 122.4 |
| penrose-tiling | xfig | pan | 107.6; 114.9; 37.8 |
| penrose-tiling | xfig | move_step | 0.3; 0.4; 0.3 |
| penrose-tiling | xfig | move_step_max | 10.5; 16.2; 27.0 |
| penrose-tiling | xfig | move_place | 107.2; 83.1; 54.8 |
| penrose-tiling | xfig | save | 11.5; 30.0; 10.7 |
| penrose-tiling | xfig | undo | 101.9; 63.8; 9.0 |
| penrose-tiling | xfig | x_open | 1708.9; 1490.5; 1527.2 |
| penrose-tiling | xfig | x_zoom_in | 70.6; 100.9; 39.8 |
| penrose-tiling | xfig | x_zoom_out | 67.2; 142.8; 133.4 |
| penrose-tiling | xfig | x_pan | 112.9; 154.2; 41.5 |
| penrose-tiling | xfig | x_move_step | 22.8; 24.3; 22.3 |
| penrose-tiling | xfig | x_move_place | 118.2; 93.2; 58.1 |
| penrose-tiling | xfig | x_save | 55.0; 84.1; 38.3 |
| penrose-tiling | xfig | x_undo | 123.6; 108.6; 43.3 |
| penrose-tiling | RustFig | open | 582.3; 547.2; 508.2 |
| penrose-tiling | RustFig | open_read | -; -; - |
| penrose-tiling | RustFig | open_draw | -; -; - |
| penrose-tiling | RustFig | zoom_in | 43.8; 31.6; 23.2 |
| penrose-tiling | RustFig | zoom_out | 10.4; 25.4; 13.0 |
| penrose-tiling | RustFig | pan | 16.9; 21.9; 10.4 |
| penrose-tiling | RustFig | move_step | 0.6; 0.5; 0.2 |
| penrose-tiling | RustFig | move_step_max | 0.7; 0.7; 0.5 |
| penrose-tiling | RustFig | move_place | 26.4; 36.1; 12.4 |
| penrose-tiling | RustFig | save | 14.5; 16.6; 6.7 |
| penrose-tiling | RustFig | undo | 32.2; 76.1; 14.4 |
| penrose-tiling | RustFig | x_open | 582.3; 547.2; 508.2 |
| penrose-tiling | RustFig | x_zoom_in | 49.9; 38.8; 38.2 |
| penrose-tiling | RustFig | x_zoom_out | 53.8; 40.3; 34.0 |
| penrose-tiling | RustFig | x_pan | 33.5; 44.3; 24.0 |
| penrose-tiling | RustFig | x_move_step | 55.7; 50.1; 50.1 |
| penrose-tiling | RustFig | x_move_place | 108.2; 101.4; 66.8 |
| penrose-tiling | RustFig | x_save | -; -; - |
| penrose-tiling | RustFig | x_undo | 50.2; 89.1; 29.6 |
| apollonian-gasket | xfig | open | 1153.3; 1126.0; 1124.8 |
| apollonian-gasket | xfig | open_read | 3.1; 2.7; 7.9 |
| apollonian-gasket | xfig | open_draw | 5.3; 4.4; 12.2 |
| apollonian-gasket | xfig | zoom_in | 5.0; 11.6; 15.4 |
| apollonian-gasket | xfig | zoom_out | 5.5; 14.0; 12.9 |
| apollonian-gasket | xfig | pan | 5.3; 16.0; 5.2 |
| apollonian-gasket | xfig | move_step | 0.6; 0.9; 0.6 |
| apollonian-gasket | xfig | move_step_max | 1.3; 1.2; 1.6 |
| apollonian-gasket | xfig | move_place | 14.6; 25.2; 12.1 |
| apollonian-gasket | xfig | save | 2.9; 6.5; 4.0 |
| apollonian-gasket | xfig | undo | 15.3; 27.2; 28.0 |
| apollonian-gasket | xfig | x_open | 1170.3; 1143.0; 1150.0 |
| apollonian-gasket | xfig | x_zoom_in | 29.5; 26.0; 30.5 |
| apollonian-gasket | xfig | x_zoom_out | 29.5; 28.1; 30.8 |
| apollonian-gasket | xfig | x_pan | 7.4; 17.9; 4.9 |
| apollonian-gasket | xfig | x_move_step | 4.9; 6.0; 5.7 |
| apollonian-gasket | xfig | x_move_place | 47.8; 49.1; 50.4 |
| apollonian-gasket | xfig | x_save | 25.7; 33.9; 29.6 |
| apollonian-gasket | xfig | x_undo | 25.4; 30.0; 28.7 |
| apollonian-gasket | RustFig | open | 225.1; 486.6; 412.1 |
| apollonian-gasket | RustFig | open_read | -; -; - |
| apollonian-gasket | RustFig | open_draw | -; -; - |
| apollonian-gasket | RustFig | zoom_in | 11.9; 28.3; 46.8 |
| apollonian-gasket | RustFig | zoom_out | 13.0; 22.1; 22.7 |
| apollonian-gasket | RustFig | pan | 4.8; 21.2; 10.8 |
| apollonian-gasket | RustFig | move_step | 0.3; 0.5; 0.5 |
| apollonian-gasket | RustFig | move_step_max | 0.6; 10.6; 10.6 |
| apollonian-gasket | RustFig | move_place | 44.6; 38.1; 24.2 |
| apollonian-gasket | RustFig | save | 10.3; 10.3; 10.9 |
| apollonian-gasket | RustFig | undo | 36.3; 47.1; 26.1 |
| apollonian-gasket | RustFig | x_open | 225.1; 486.6; 412.1 |
| apollonian-gasket | RustFig | x_zoom_in | 27.2; 37.7; 76.1 |
| apollonian-gasket | RustFig | x_zoom_out | 28.4; 40.0; 33.1 |
| apollonian-gasket | RustFig | x_pan | 16.8; 26.6; 25.2 |
| apollonian-gasket | RustFig | x_move_step | 50.3; 54.9; 57.0 |
| apollonian-gasket | RustFig | x_move_place | 111.3; 119.5; 101.6 |
| apollonian-gasket | RustFig | x_save | -; -; - |
| apollonian-gasket | RustFig | x_undo | 60.0; 72.3; 39.4 |
| blankmap | xfig | open | 1362.2; 1282.7; 1130.9 |
| blankmap | xfig | open_read | 23.1; 19.8; 22.6 |
| blankmap | xfig | open_draw | 18.4; 11.1; 5.5 |
| blankmap | xfig | zoom_in | 82.6; 14.2; 19.2 |
| blankmap | xfig | zoom_out | 14.2; 11.9; 10.1 |
| blankmap | xfig | pan | 22.1; 16.9; 22.4 |
| blankmap | xfig | move_step | 0.9; 0.9; 1.0 |
| blankmap | xfig | move_step_max | 7.6; 2.0; 1.7 |
| blankmap | xfig | move_place | 281.8; 90.3; 86.2 |
| blankmap | xfig | save | 17.4; 16.3; 15.1 |
| blankmap | xfig | undo | 230.5; 87.1; 86.8 |
| blankmap | xfig | x_open | 1397.9; 1359.7; 1154.3 |
| blankmap | xfig | x_zoom_in | 115.9; 26.4; 27.2 |
| blankmap | xfig | x_zoom_out | 40.2; 30.9; 29.6 |
| blankmap | xfig | x_pan | 23.6; 19.1; 24.0 |
| blankmap | xfig | x_move_step | 19.4; 5.7; 6.2 |
| blankmap | xfig | x_move_place | 287.4; 92.5; 86.7 |
| blankmap | xfig | x_save | 50.8; 30.8; 28.2 |
| blankmap | xfig | x_undo | 244.9; 88.6; 87.9 |
| blankmap | RustFig | open | 228.2; 247.5; 211.6 |
| blankmap | RustFig | open_read | -; -; - |
| blankmap | RustFig | open_draw | -; -; - |
| blankmap | RustFig | zoom_in | 17.3; 22.0; 17.8 |
| blankmap | RustFig | zoom_out | 14.7; 17.9; 14.2 |
| blankmap | RustFig | pan | 6.3; 6.9; 6.2 |
| blankmap | RustFig | move_step | 0.1; 0.2; 0.1 |
| blankmap | RustFig | move_step_max | 0.3; 0.2; 0.2 |
| blankmap | RustFig | move_place | 22.4; 21.0; 20.4 |
| blankmap | RustFig | save | 15.5; 7.8; 13.9 |
| blankmap | RustFig | undo | 24.5; 21.3; 21.2 |
| blankmap | RustFig | x_open | 228.2; 247.5; 211.6 |
| blankmap | RustFig | x_zoom_in | 38.1; 35.5; 24.0 |
| blankmap | RustFig | x_zoom_out | 28.5; 37.5; 21.0 |
| blankmap | RustFig | x_pan | 21.0; 21.6; 16.8 |
| blankmap | RustFig | x_move_step | 54.3; 50.1; 54.7 |
| blankmap | RustFig | x_move_place | 83.2; 78.8; 100.2 |
| blankmap | RustFig | x_save | -; -; - |
| blankmap | RustFig | x_undo | 38.9; 45.2; 28.9 |
| ne-coastline-10m | xfig | open | 1158.7; 1166.7; 1755.7 |
| ne-coastline-10m | xfig | open_read | 51.8; 51.8; 126.6 |
| ne-coastline-10m | xfig | open_draw | 2.9; 3.4; 8.6 |
| ne-coastline-10m | xfig | zoom_in | 10.6; 9.8; 116.0 |
| ne-coastline-10m | xfig | zoom_out | 9.3; 9.0; 31.3 |
| ne-coastline-10m | xfig | pan | 12.3; 10.0; 87.3 |
| ne-coastline-10m | xfig | move_step | 1.7; 1.5; 1.3 |
| ne-coastline-10m | xfig | move_step_max | 2.3; 36.0; 40.4 |
| ne-coastline-10m | xfig | move_place | 387.7; 1384.4; 1197.5 |
| ne-coastline-10m | xfig | save | 37.3; 69.4; 63.7 |
| ne-coastline-10m | xfig | undo | 52.5; 251.0; 209.7 |
| ne-coastline-10m | xfig | x_open | 1183.6; 1191.4; 1814.6 |
| ne-coastline-10m | xfig | x_zoom_in | 31.6; 31.2; 123.2 |
| ne-coastline-10m | xfig | x_zoom_out | 26.9; 27.6; 89.8 |
| ne-coastline-10m | xfig | x_pan | 12.0; 9.4; 95.9 |
| ne-coastline-10m | xfig | x_move_step | 6.3; 7.5; 26.3 |
| ne-coastline-10m | xfig | x_move_place | 387.8; 1392.3; 1198.8 |
| ne-coastline-10m | xfig | x_save | 40.1; 78.8; 74.1 |
| ne-coastline-10m | xfig | x_undo | 53.7; 268.6; 219.7 |
| ne-coastline-10m | RustFig | open | 525.7; 452.1; 447.0 |
| ne-coastline-10m | RustFig | open_read | -; -; - |
| ne-coastline-10m | RustFig | open_draw | -; -; - |
| ne-coastline-10m | RustFig | zoom_in | 27.3; 30.8; 64.1 |
| ne-coastline-10m | RustFig | zoom_out | 25.7; 17.7; 0.8 |
| ne-coastline-10m | RustFig | pan | 10.7; 10.4; 10.3 |
| ne-coastline-10m | RustFig | move_step | 0.5; 0.4; 0.4 |
| ne-coastline-10m | RustFig | move_step_max | 10.6; 6.1; 0.5 |
| ne-coastline-10m | RustFig | move_place | 37.6; 27.2; 28.2 |
| ne-coastline-10m | RustFig | save | 41.4; 35.9; 33.3 |
| ne-coastline-10m | RustFig | undo | 73.5; 37.5; 36.9 |
| ne-coastline-10m | RustFig | x_open | 525.7; 452.1; 447.0 |
| ne-coastline-10m | RustFig | x_zoom_in | 42.7; 40.2; 89.7 |
| ne-coastline-10m | RustFig | x_zoom_out | 62.0; 33.0; 70.0 |
| ne-coastline-10m | RustFig | x_pan | 57.7; 21.5; 33.7 |
| ne-coastline-10m | RustFig | x_move_step | 53.4; 54.2; 51.1 |
| ne-coastline-10m | RustFig | x_move_place | 100.0; 83.3; 88.5 |
| ne-coastline-10m | RustFig | x_save | -; -; - |
| ne-coastline-10m | RustFig | x_undo | 107.4; 59.7; 61.9 |
| ne-countries-10m | xfig | open | 1216.8; 1214.6; 1229.6 |
| ne-countries-10m | xfig | open_read | 56.7; 64.0; 66.5 |
| ne-countries-10m | xfig | open_draw | 9.7; 10.6; 10.5 |
| ne-countries-10m | xfig | zoom_in | 24.0; 11.4; 11.0 |
| ne-countries-10m | xfig | zoom_out | 50.2; 10.3; 10.9 |
| ne-countries-10m | xfig | pan | 16.9; 11.8; 12.7 |
| ne-countries-10m | xfig | move_step | 0.5; 0.5; 0.5 |
| ne-countries-10m | xfig | move_step_max | 0.6; 0.6; 0.8 |
| ne-countries-10m | xfig | move_place | 524.2; 517.0; 623.9 |
| ne-countries-10m | xfig | save | 39.9; 40.9; 48.2 |
| ne-countries-10m | xfig | undo | 69.4; 68.2; 82.7 |
| ne-countries-10m | xfig | x_open | 1234.2; 1230.4; 1245.7 |
| ne-countries-10m | xfig | x_zoom_in | 58.5; 28.5; 29.7 |
| ne-countries-10m | xfig | x_zoom_out | 51.1; 26.7; 27.2 |
| ne-countries-10m | xfig | x_pan | 19.7; 11.3; 14.6 |
| ne-countries-10m | xfig | x_move_step | 4.7; 4.7; 4.9 |
| ne-countries-10m | xfig | x_move_place | 524.3; 518.9; 624.4 |
| ne-countries-10m | xfig | x_save | 42.2; 42.4; 48.7 |
| ne-countries-10m | xfig | x_undo | 72.1; 70.2; 83.5 |
| ne-countries-10m | RustFig | open | 343.5; 348.0; 409.0 |
| ne-countries-10m | RustFig | open_read | -; -; - |
| ne-countries-10m | RustFig | open_draw | -; -; - |
| ne-countries-10m | RustFig | zoom_in | 31.3; 26.9; 26.1 |
| ne-countries-10m | RustFig | zoom_out | 22.8; 17.8; 15.4 |
| ne-countries-10m | RustFig | pan | 0.8; 10.0; 8.8 |
| ne-countries-10m | RustFig | move_step | 0.3; 0.3; 0.3 |
| ne-countries-10m | RustFig | move_step_max | 0.4; 0.5; 0.5 |
| ne-countries-10m | RustFig | move_place | 34.6; 31.0; 31.2 |
| ne-countries-10m | RustFig | save | 22.2; 20.6; 28.4 |
| ne-countries-10m | RustFig | undo | 39.5; 40.5; 35.2 |
| ne-countries-10m | RustFig | x_open | 343.5; 348.0; 409.0 |
| ne-countries-10m | RustFig | x_zoom_in | 39.1; 37.6; 41.6 |
| ne-countries-10m | RustFig | x_zoom_out | 44.2; 33.4; 33.4 |
| ne-countries-10m | RustFig | x_pan | 21.6; 33.4; 23.9 |
| ne-countries-10m | RustFig | x_move_step | 50.0; 50.0; 50.0 |
| ne-countries-10m | RustFig | x_move_place | 100.1; 100.0; 100.0 |
| ne-countries-10m | RustFig | x_save | -; -; - |
| ne-countries-10m | RustFig | x_undo | 54.4; 56.2; 54.1 |
| greece | xfig | open | 1207.5; 1182.9; 1632.1 |
| greece | xfig | open_read | 79.8; 61.8; 142.6 |
| greece | xfig | open_draw | 11.4; 7.5; 23.6 |
| greece | xfig | zoom_in | 6.5; 14.5; 14.2 |
| greece | xfig | zoom_out | 8.2; 7.0; 19.7 |
| greece | xfig | pan | 6.3; 14.9; 13.1 |
| greece | xfig | move_step | 0.6; 0.4; 0.4 |
| greece | xfig | move_step_max | 0.7; 9.8; 11.1 |
| greece | xfig | move_place | 228.1; 924.6; 909.9 |
| greece | xfig | save | 29.8; 41.9; 62.6 |
| greece | xfig | undo | 41.8; 93.6; 161.1 |
| greece | xfig | x_open | 1223.3; 1196.7; 1684.8 |
| greece | xfig | x_zoom_in | 29.8; 26.7; 32.1 |
| greece | xfig | x_zoom_out | 25.1; 29.3; 48.8 |
| greece | xfig | x_pan | 8.2; 16.6; 36.6 |
| greece | xfig | x_move_step | 7.2; 22.0; 22.0 |
| greece | xfig | x_move_place | 230.5; 927.5; 912.6 |
| greece | xfig | x_save | 32.4; 44.2; 96.4 |
| greece | xfig | x_undo | 43.0; 105.3; 167.9 |
| greece | RustFig | open | 1083.5; 958.0; 944.8 |
| greece | RustFig | open_read | -; -; - |
| greece | RustFig | open_draw | -; -; - |
| greece | RustFig | zoom_in | 131.1; 124.1; 144.4 |
| greece | RustFig | zoom_out | 142.2; 150.1; 131.2 |
| greece | RustFig | pan | 80.8; 121.1; 85.0 |
| greece | RustFig | move_step | 0.4; 0.4; 0.4 |
| greece | RustFig | move_step_max | 0.5; 1.0; 56.3 |
| greece | RustFig | move_place | 108.5; 114.2; 112.6 |
| greece | RustFig | save | 29.5; 25.2; 18.5 |
| greece | RustFig | undo | 123.1; 137.1; 154.6 |
| greece | RustFig | x_open | 1083.5; 958.0; 944.8 |
| greece | RustFig | x_zoom_in | 146.0; 133.4; 155.0 |
| greece | RustFig | x_zoom_out | 160.9; 165.0; 139.7 |
| greece | RustFig | x_pan | 88.5; 144.3; 98.6 |
| greece | RustFig | x_move_step | 50.8; 50.7; 56.1 |
| greece | RustFig | x_move_place | 166.6; 188.7; 185.9 |
| greece | RustFig | x_save | -; -; - |
| greece | RustFig | x_undo | 133.7; 150.4; 158.6 |
| clifford-attractor | xfig | open | 1888.4; 1330.8; 1329.8 |
| clifford-attractor | xfig | open_read | 204.4; 105.1; 101.8 |
| clifford-attractor | xfig | open_draw | 300.4; 119.6; 119.6 |
| clifford-attractor | xfig | zoom_in | 435.6; 136.2; 113.3 |
| clifford-attractor | xfig | zoom_out | 172.5; 154.5; 131.5 |
| clifford-attractor | xfig | pan | 91.6; 74.4; 71.5 |
| clifford-attractor | xfig | move_step | 0.3; 0.4; 0.3 |
| clifford-attractor | xfig | move_step_max | 0.6; 0.5; 0.6 |
| clifford-attractor | xfig | move_place | 221.9; 224.7; 228.9 |
| clifford-attractor | xfig | save | 107.2; 110.8; 108.4 |
| clifford-attractor | xfig | undo | 43.4; 36.8; 44.7 |
| clifford-attractor | xfig | x_open | 1920.0; 1355.6; 1349.5 |
| clifford-attractor | xfig | x_zoom_in | 472.5; 137.7; 115.6 |
| clifford-attractor | xfig | x_zoom_out | 172.9; 156.2; 134.4 |
| clifford-attractor | xfig | x_pan | 93.9; 74.0; 71.5 |
| clifford-attractor | xfig | x_move_step | 6.3; 6.4; 6.1 |
| clifford-attractor | xfig | x_move_place | 222.2; 227.0; 230.1 |
| clifford-attractor | xfig | x_save | 110.7; 113.1; 111.2 |
| clifford-attractor | xfig | x_undo | 45.1; 38.9; 47.1 |
| clifford-attractor | RustFig | open | 513.5; 1283.0; 1266.2 |
| clifford-attractor | RustFig | open_read | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":544}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":603}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":606}) |
| clifford-attractor | RustFig | open_draw | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":544}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":603}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":606}) |
| clifford-attractor | RustFig | zoom_in | 123.0; 427.8; 393.7 |
| clifford-attractor | RustFig | zoom_out | 75.2; 254.1; 204.7 |
| clifford-attractor | RustFig | pan | 67.9; 189.6; 181.1 |
| clifford-attractor | RustFig | move_step | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":544}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":603}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":606}) |
| clifford-attractor | RustFig | move_step_max | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":544}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":603}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":606}) |
| clifford-attractor | RustFig | move_place | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":544}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":603}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":606}) |
| clifford-attractor | RustFig | save | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":544}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":603}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":606}) |
| clifford-attractor | RustFig | undo | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":544}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":603}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":606}) |
| clifford-attractor | RustFig | x_open | 513.5; 1283.0; 1266.2 |
| clifford-attractor | RustFig | x_zoom_in | 116.7; 318.2; 283.4 |
| clifford-attractor | RustFig | x_zoom_out | 72.2; 217.5; 186.5 |
| clifford-attractor | RustFig | x_pan | 72.6; 153.1; 144.3 |
| clifford-attractor | RustFig | x_move_step | -; -; - |
| clifford-attractor | RustFig | x_move_place | -; -; - |
| clifford-attractor | RustFig | x_save | -; -; - |
| clifford-attractor | RustFig | x_undo | -; -; - |
| cube8 | xfig | open | 1635.0; 1325.0; 1342.5 |
| cube8 | xfig | open_read | 117.5; 81.3; 93.1 |
| cube8 | xfig | open_draw | 221.8; 126.3; 129.9 |
| cube8 | xfig | zoom_in | 118.3; 135.7; 117.5 |
| cube8 | xfig | zoom_out | 133.9; 135.5; 134.6 |
| cube8 | xfig | pan | 107.8; 124.0; 113.3 |
| cube8 | xfig | move_step | 0.3; 0.5; 0.2 |
| cube8 | xfig | move_step_max | 0.7; 0.7; 0.6 |
| cube8 | xfig | move_place | 323.5; 403.8; 376.2 |
| cube8 | xfig | save | 82.4; 110.2; 119.8 |
| cube8 | xfig | undo | 48.2; 73.9; 149.7 |
| cube8 | xfig | x_open | 1691.3; 1342.7; 1363.5 |
| cube8 | xfig | x_zoom_in | 120.6; 137.0; 119.8 |
| cube8 | xfig | x_zoom_out | 133.7; 135.1; 135.7 |
| cube8 | xfig | x_pan | 110.2; 124.6; 113.7 |
| cube8 | xfig | x_move_step | 4.9; 7.1; 5.2 |
| cube8 | xfig | x_move_place | 324.5; 406.4; 377.2 |
| cube8 | xfig | x_save | 83.1; 111.8; 120.9 |
| cube8 | xfig | x_undo | 49.5; 75.2; 171.6 |
| cube8 | RustFig | open | 3598.9; 4332.7; 5735.3 |
| cube8 | RustFig | open_read | -; n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":1503}); - |
| cube8 | RustFig | open_draw | -; n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":1503}); - |
| cube8 | RustFig | zoom_in | 974.0; 1685.8; 2111.5 |
| cube8 | RustFig | zoom_out | 817.1; 1184.2; 1545.0 |
| cube8 | RustFig | pan | 266.0; 620.7; 811.6 |
| cube8 | RustFig | move_step | 1.3; n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":1503}); 1.7 |
| cube8 | RustFig | move_step_max | 1.5; n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":1503}); 2.0 |
| cube8 | RustFig | move_place | 1271.9; n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":1503}); 1658.2 |
| cube8 | RustFig | save | 165.5; n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":1503}); 245.1 |
| cube8 | RustFig | undo | 1568.9; n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":1503}); 2142.2 |
| cube8 | RustFig | x_open | 3598.9; 4332.7; 5735.3 |
| cube8 | RustFig | x_zoom_in | 818.0; 1367.4; 1637.2 |
| cube8 | RustFig | x_zoom_out | 683.4; 966.0; 1201.8 |
| cube8 | RustFig | x_pan | 250.3; 517.6; 652.8 |
| cube8 | RustFig | x_move_step | 50.1; -; 50.0 |
| cube8 | RustFig | x_move_place | 1325.9; -; 1733.5 |
| cube8 | RustFig | x_save | -; -; - |
| cube8 | RustFig | x_undo | 1397.5; -; 1826.5 |

</details>

Move test: 30 steps over 240 x 120 pixels. Figures with more than 300 objects (snowy, penrose-tiling, apollonian-gasket, blankmap, ne-coastline-10m, ne-countries-10m, greece, clifford-attractor, cube8) were glued into one compound first (xfig: Glue, middle-button region, right button; RustFig: select all, group), smaller ones move a single object.

<!-- results:end -->
