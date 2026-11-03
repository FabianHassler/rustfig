# Speed comparison with xfig

**Status:** 2026-11-03. Measured with `cargo xtask speed-compare` (see below); the tables inside the result blocks are generated, everything else is written by hand. Revision history: 2026-11-02 first version (RustFig = local main e2c3c50, after the speed work, measured on a quiet Mac mini M4 at equal resolution); 2026-11-03 re-measured with main 2112440 (after P8: linear selection commands, exact curves in the raster, scene built on several threads), display kept awake (`caffeinate -d -u`).

RustFig against the real xfig 3.2.9a on the same figures: open, zoom, pan, move, save, undo. This page is the evidence for a speed claim on the website; it says where xfig is faster as well.

## Setup

**What is compared**

- **xfig 3.2.9a**, built from the unmodified sources in `old/xfig-3.2.9a` with the same configure flags, dependencies (Xaw3d, Xft, libpng, jpeg-turbo, libtiff, libxpm, ghostscript, Homebrew's app-defaults directory with `Fig.international: False`) and `CFLAGS=-O2` as Homebrew's formula (`brew cat xfig`), plus timing hooks. `xtask/xfig-timing/build.sh` copies the sources, applies `xtask/xfig-timing/xfig-timing.patch` (the hooks only; `old/` is never touched) and builds. With `XFIG_TIMING=1` the hooks print `XT <name> <start ms> <duration ms>` on stderr (CLOCK_MONOTONIC); without the variable they do nothing. Default options, scratch `HOME`, on its own `Xvfb :78` (1400x1000x24, software X server, started with `-fbdir`), window `1322x818`, canvas 1128x686 pixels. The build needs `-std=gnu17` with a current compiler (xfig's C is not C23-clean); otherwise the flags are Homebrew's.
- **RustFig**: release build, `--background`, scratch settings and recovery folders, driven through its control port. **Main table (same resolution)**: `RUSTFIG_PIXELS_PER_POINT=1` (a testing option added with this page, read at start-up: one device pixel per point on any display, and `-geometry` is then the window size in device pixels) and `-geometry 1398x803`, which gives a canvas of 1128x686 device pixels, the same as xfig's (checked in `ui.state` and printed in each result block). **Second table (Retina 2x, for information)**: without the option, the display's own density (this display: 2x), `-geometry 1398x803` points, so the canvas is 1128x686 points = 2256x1372 pixels, four times the pixels of xfig's. The build under test is given to `cargo xtask speed-compare --rustfig BIN`.
- Machine: **Mac mini (M4, 10 cores: 4 performance + 6 efficiency, 32 GB), macOS 27.2**, 4K display (2x scaling), a user logged in; load average 1.3-4.4 during the 2026-11-02 runs (some other activity), 1.1-1.8 during the 2026-11-03 runs (at night, display kept awake). Earlier runs on a busy MacBook (M5, load 4-25, RustFig built before the Gosper fix) are not reported: they were too noisy. xfig and RustFig ran one after the other, never in parallel. The same figures, plain `.fig` copies (xfig reads `.fig.gz` too, but both get the same file). A new process for every run; each value is the **median of 3 runs**; a measurement that does not finish in 120 s is reported as "> 120 s".

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

**Tasks.** *Zoom in/out*: one step (xfig: the `Z`/`z` key with the pointer at the canvas center, a step of sqrt 2; RustFig: `view.zoom_in`/`zoom_out`, also sqrt 2) at zoom 1 with Fig (0,0) at the canvas corner. *Pan*: half a canvas width (500 px): xfig by the middle button dragged along the top ruler (the redraw happens when the button is released), RustFig `view.pan`. *Move*: Move mode, pick one object by its vertex or outline, 30 pointer steps over 240 x 120 pixels, then place. Figures of more than 300 objects are first glued into one compound (xfig: Glue mode, middle-button region over the whole canvas at the zoom that shows the whole figure, right button; RustFig: select all, group) and moved at that zoom; the small figures move a single object. *Save*: Alt-S in xfig, `file.save` in RustFig; *Undo*: Alt-U / `edit.undo` after the move.

**What each app draws while moving.** xfig draws the *outline* in XOR mode, straight to the window: for a compound only its bounding box (`moving_box`), for a polyline its shape, for a text its box; nothing underneath is redrawn, so a step costs well under a millisecond whatever the figure. When the object is placed (and for Undo) xfig redraws the affected region of the canvas, which is the expensive part on big figures. RustFig shows the real object moving over a cached layer of the rest of the drawing (since the speed work): a step costs the same for any figure, and placing re-renders.

**Caveats.**

- xfig runs on `Xvfb`, a software X server on the same machine; real XQuartz or a Linux X server would draw faster or slower. The hooks include the server's drawing time (`XSync`), so Xvfb's cost is in xfig's numbers, but not a GPU's.
- In the main table both draw a 1128x686 pixel canvas. RustFig still uses the GPU for presentation (not in its numbers); xfig's X server is a software server.
- Fonts: xfig uses X fonts, RustFig the system fonts of the machine (its own start-up scans them, in the open time).
- Medians of 3 runs; differences below about 20 % (and below 2 ms) are not meaningful.
- The two apps open a figure differently (xfig at zoom 1, RustFig fitted to the window): the open row compares what the user waits for, not equal work.
- `ui.state` polling adds frames in RustFig; the 130-frame flushes make runs of RustFig long, not slower.

**Re-running with another RustFig build**

```sh
cargo build --release -p rustfig                 # the build to measure (needs the ui.state fields and RUSTFIG_PIXELS_PER_POINT of this page's branch)
xtask/xfig-timing/build.sh /some/dir             # once; builds the instrumented xfig into /some/dir/src/src/xfig
cargo run --release -p xtask -- speed-compare \
    --rustfig target/release/rustfig --label "my build" \
    --xfig-bin /some/dir/src/src/xfig --examples /path/to/examples \
    --rf-ppp 1 --rf-geometry 1398x803 \
    [--runs 3] [--timeout 120] [--filter pines,greece] [--out docs/speed-compare.md]
```

`--examples` is the folder with the example figures (`pines/pines.fig.gz`, ..., `gallery/*.fig`; the `examples/` folder of the main checkout). The harness starts and stops its own `Xvfb` (display `:78`, `--display` to change), and writes the generated part of this page after each figure. Run it as a release build (a debug build is too slow to watch the frame buffer). Without `--rf-ppp` RustFig runs at the display's density (the Retina table). The `-geometry` that gives a 1128x686 canvas depends on the machine's fonts and UI chrome (1400x826 on the MacBook, 1398x803 on the Mac mini): check the canvas line in the result block. A RustFig build without `canvas_pending` in `ui.state` is measured with a fixed 600 ms wait: do not compare those numbers.

## Summary (what the numbers say)

Same resolution, Mac mini M4 (the main table below), RustFig at main 2112440. Time of RustFig's own work against xfig's own hooks; above 1x = RustFig faster.

- **Open** (xfig `main()` to drawn; RustFig process start to drawn): RustFig is **4-6.5x faster on 13 of 15 figures** (176-302 ms against xfig's 1.1-1.3 s), 2.8x on clifford-attractor (250,000 objects: 476 ms against 1.31 s), **equal on cube8** (1.39 against 1.33 s; 190,000 circles). xfig spends about 1.1 s in start-up before it reads the file.
- **Zoom and pan** (one step, half a screen): RustFig is **faster on 13 of 15 figures**, 1.4-11x (zoom in on carnot 4.2 against 28 ms, blankmap pan 1.8 against 18 ms); about equal on greece and on clifford-attractor's pan; **cube8 is the exception: xfig 3x faster** (zoom 67 against 229 ms, pan 68 against 182 ms).
- **Move, the drag**: both draw a step in about 1 ms or less on any figure (xfig an XOR outline, a compound's bounding box; RustFig the real object over a cached layer); RustFig's steps are 0.1-0.2 ms.
- **Move, placing** (the final redraw): RustFig **faster on 13 of 15** (ne-coastline 14 against 130 ms, 9x; ne-countries 21 against 160 ms; greece 19 against 101 ms); xfig faster on carnot (5.2 against 7.6 ms) and cube8 (141 against 341 ms).
- **Save**: RustFig 1.4-8x faster, equal on cube8.
- **Undo**: RustFig faster or equal on 12 of 15 (blankmap 5.3 against 50 ms, wenig 5.3 against 40 ms); slower on carnot (6.8 against 4.4 ms), clifford-attractor (86 against 33 ms) and cube8 (417 against 39 ms): undo on very large drawings redraws everything, where xfig redraws only the changed region.
- **Select all** (not a task of the table, xfig has no such command): 190,000-250,000 objects now take about 50 ms (before P8: 37-65 s).
- **Retina 2x** (second table, for information): RustFig draws four times the pixels of xfig's canvas. Opening, the drag, placing on the maps and saving stay faster than xfig; zoom and pan on the large figures become slower than xfig (greece zoom 29 against 14 ms, cube8 453 against 66 ms), and small figures take a few ms more (carnot undo 11 against 3 ms).

## Results at the same resolution (RustFig 1128x686 device pixels, as xfig)

<!-- results:start -->

### Setup of this run

- Machine: Apple M4, macOS 27.2
- xfig: `/Users/fabian/rustfig-speed/xfig/src/src/xfig` (3.2.9a), default options, scratch `HOME`, on `Xvfb :78` 1400x1000x24; window `1322x818` (canvas 1128x686 pixels)
- RustFig: `/Users/fabian/rustfig-speed/bin/rustfig` (rustfig 0.9.0-beta.7), release build, label: main 2112440 (after P8), same resolution; `-geometry 1398x803`, `RUSTFIG_PIXELS_PER_POINT=1` (window size in device pixels)
- Each value is the median of 3 run(s), a new process per run; timeout 120 s per measurement ("> N s": not finished in that time; the real time is longer)
- Generated 2026-11-03 00:17 by `cargo xtask speed-compare`

- RustFig canvas as reported by `ui.state` (points): 1128x686; density 1 device pixel per point (xfig's canvas: 1128x686 pixels)
### Results

xfig / RustFig is the speed-up of RustFig (above 1 = RustFig is faster).

#### Open (xfig: main() to figure drawn; RustFig: process start to drawn)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.14 s | 192 ms | 6.0x |
| feynman (small) | 4 kB, 34 obj. | 1.14 s | 187 ms | 6.1x |
| fourier (small) | 16 kB, 41 obj. | 1.13 s | 193 ms | 5.9x |
| pines | 11 kB, 31 obj. | 1.11 s | 176 ms | 6.3x |
| snowy | 173 kB, 436 obj. | 1.12 s | 193 ms | 5.8x |
| wenig | 587 kB, 85 obj. | 1.13 s | 184 ms | 6.2x |
| gosper-curve | 182 kB, 142 obj. | 1.16 s | 178 ms | 6.5x |
| penrose-tiling | 560 kB, 7615 obj. | 1.19 s | 191 ms | 6.3x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.17 s | 198 ms | 5.9x |
| blankmap | 1.0 MB, 2232 obj. | 1.20 s | 198 ms | 6.1x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.26 s | 242 ms | 5.2x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.25 s | 264 ms | 4.7x |
| greece | 3.2 MB, 2013 obj. | 1.25 s | 302 ms | 4.1x |
| clifford-attractor | 13.2 MB, 250001 obj. | 1.31 s | 476 ms | 2.8x |
| cube8 | 14.6 MB, 190260 obj. | 1.33 s | 1.39 s | 1.0x |

#### Open, part 1: reading the file (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 42.8 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 42.6 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 41.0 ms | n/a | - |
| pines | 11 kB, 31 obj. | 2.7 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 7.9 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 23.0 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 10.9 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 22.0 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 10.7 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 30.4 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 78.3 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 88.9 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 84.6 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 129 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 122 ms | n/a | - |

#### Open, part 2: first full canvas redraw (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.9 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 1.7 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 1.2 ms | n/a | - |
| pines | 11 kB, 31 obj. | 7.2 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 5.6 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 10.1 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 4.5 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 12.3 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 6.6 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 4.7 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 3.0 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 5.4 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 4.8 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 33.4 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 45.4 ms | n/a | - |

#### Zoom in (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 28.3 ms | 4.2 ms | 6.7x |
| feynman (small) | 4 kB, 34 obj. | 33.0 ms | 2.9 ms | 11x |
| fourier (small) | 16 kB, 41 obj. | 28.8 ms | 3.3 ms | 8.6x |
| pines | 11 kB, 31 obj. | 8.4 ms | 1.6 ms | 5.2x |
| snowy | 173 kB, 436 obj. | 8.6 ms | 1.0 ms | 8.7x |
| wenig | 587 kB, 85 obj. | 20.6 ms | 3.0 ms | 6.8x |
| gosper-curve | 182 kB, 142 obj. | 8.7 ms | 5.0 ms | 1.7x |
| penrose-tiling | 560 kB, 7615 obj. | 21.9 ms | 4.5 ms | 4.9x |
| apollonian-gasket | 330 kB, 4221 obj. | 10.7 ms | 2.4 ms | 4.4x |
| blankmap | 1.0 MB, 2232 obj. | 14.7 ms | 2.4 ms | 6.2x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 12.1 ms | 4.7 ms | 2.6x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 19.7 ms | 7.3 ms | 2.7x |
| greece | 3.2 MB, 2013 obj. | 14.2 ms | 14.5 ms | 1.0x |
| clifford-attractor | 13.2 MB, 250001 obj. | 61.4 ms | 39.9 ms | 1.5x |
| cube8 | 14.6 MB, 190260 obj. | 66.6 ms | 229 ms | 0.3x |

#### Zoom out (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 8.0 ms | 4.2 ms | 1.9x |
| feynman (small) | 4 kB, 34 obj. | 7.1 ms | 3.4 ms | 2.1x |
| fourier (small) | 16 kB, 41 obj. | 6.1 ms | 3.7 ms | 1.7x |
| pines | 11 kB, 31 obj. | 9.1 ms | 1.7 ms | 5.3x |
| snowy | 173 kB, 436 obj. | 8.9 ms | 1.1 ms | 8.2x |
| wenig | 587 kB, 85 obj. | 18.7 ms | 3.2 ms | 5.9x |
| gosper-curve | 182 kB, 142 obj. | 8.7 ms | 1.7 ms | 5.1x |
| penrose-tiling | 560 kB, 7615 obj. | 24.3 ms | 5.0 ms | 4.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 10.9 ms | 2.5 ms | 4.3x |
| blankmap | 1.0 MB, 2232 obj. | 14.9 ms | 2.8 ms | 5.4x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 13.1 ms | 5.8 ms | 2.2x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 19.0 ms | 8.7 ms | 2.2x |
| greece | 3.2 MB, 2013 obj. | 15.9 ms | 15.1 ms | 1.1x |
| clifford-attractor | 13.2 MB, 250001 obj. | 60.9 ms | 51.2 ms | 1.2x |
| cube8 | 14.6 MB, 190260 obj. | 79.7 ms | 261 ms | 0.3x |

#### Pan half a screen

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 4.8 ms | 2.8 ms | 1.7x |
| feynman (small) | 4 kB, 34 obj. | 3.9 ms | 2.9 ms | 1.4x |
| fourier (small) | 16 kB, 41 obj. | 4.7 ms | 3.4 ms | 1.4x |
| pines | 11 kB, 31 obj. | 4.3 ms | 1.6 ms | 2.6x |
| snowy | 173 kB, 436 obj. | 6.8 ms | 0.9 ms | 7.2x |
| wenig | 587 kB, 85 obj. | 17.7 ms | 2.7 ms | 6.6x |
| gosper-curve | 182 kB, 142 obj. | 4.9 ms | 1.7 ms | 2.8x |
| penrose-tiling | 560 kB, 7615 obj. | 15.7 ms | 4.1 ms | 3.9x |
| apollonian-gasket | 330 kB, 4221 obj. | 12.0 ms | 1.9 ms | 6.5x |
| blankmap | 1.0 MB, 2232 obj. | 17.9 ms | 1.8 ms | 10.0x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 12.1 ms | 3.3 ms | 3.7x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 21.2 ms | 5.1 ms | 4.2x |
| greece | 3.2 MB, 2013 obj. | 11.8 ms | 13.6 ms | 0.9x |
| clifford-attractor | 13.2 MB, 250001 obj. | 41.5 ms | 44.4 ms | 0.9x |
| cube8 | 14.6 MB, 190260 obj. | 67.6 ms | 182 ms | 0.4x |

#### Move: one step of the drag (median of 30)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 0.8 ms | 0.1 ms | 7.4x |
| feynman (small) | 4 kB, 34 obj. | 9.1 ms | 0.1 ms | 81x |
| fourier (small) | 16 kB, 41 obj. | 0.8 ms | 0.1 ms | 7.1x |
| pines | 11 kB, 31 obj. | 1.0 ms | 0.1 ms | 8.9x |
| snowy | 173 kB, 436 obj. | 0.5 ms | 0.1 ms | 4.5x |
| wenig | 587 kB, 85 obj. | 2.2 ms | 0.1 ms | 20x |
| gosper-curve | 182 kB, 142 obj. | 0.6 ms | 0.1 ms | 5.8x |
| penrose-tiling | 560 kB, 7615 obj. | 0.5 ms | 0.1 ms | 4.5x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.3 ms | 0.1 ms | 9.3x |
| blankmap | 1.0 MB, 2232 obj. | 1.3 ms | 0.1 ms | 13x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.6 ms | 0.1 ms | 15x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.0 ms | 0.1 ms | 9.3x |
| greece | 3.2 MB, 2013 obj. | 0.5 ms | 0.1 ms | 4.5x |
| clifford-attractor | 13.2 MB, 250001 obj. | 0.5 ms | 0.1 ms | 3.5x |
| cube8 | 14.6 MB, 190260 obj. | 0.5 ms | 0.2 ms | 2.3x |

#### Move: place (final redraw)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 5.2 ms | 7.6 ms | 0.7x |
| feynman (small) | 4 kB, 34 obj. | 51.6 ms | 4.6 ms | 11x |
| fourier (small) | 16 kB, 41 obj. | 4.7 ms | 3.3 ms | 1.4x |
| pines | 11 kB, 31 obj. | 8.0 ms | 1.7 ms | 4.8x |
| snowy | 173 kB, 436 obj. | 15.4 ms | 2.3 ms | 6.6x |
| wenig | 587 kB, 85 obj. | 38.6 ms | 5.3 ms | 7.2x |
| gosper-curve | 182 kB, 142 obj. | 18.9 ms | 2.0 ms | 9.3x |
| penrose-tiling | 560 kB, 7615 obj. | 23.6 ms | 5.3 ms | 4.5x |
| apollonian-gasket | 330 kB, 4221 obj. | 18.7 ms | 6.1 ms | 3.1x |
| blankmap | 1.0 MB, 2232 obj. | 50.3 ms | 5.8 ms | 8.7x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 130 ms | 14.3 ms | 9.1x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 160 ms | 20.6 ms | 7.8x |
| greece | 3.2 MB, 2013 obj. | 101 ms | 19.0 ms | 5.3x |
| clifford-attractor | 13.2 MB, 250001 obj. | 99.1 ms | 71.4 ms | 1.4x |
| cube8 | 14.6 MB, 190260 obj. | 141 ms | 341 ms | 0.4x |

#### Save

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 3.2 ms | 1.4 ms | 2.3x |
| feynman (small) | 4 kB, 34 obj. | 2.3 ms | 1.2 ms | 1.9x |
| fourier (small) | 16 kB, 41 obj. | 3.0 ms | 1.5 ms | 2.0x |
| pines | 11 kB, 31 obj. | 2.7 ms | 1.3 ms | 2.1x |
| snowy | 173 kB, 436 obj. | 8.2 ms | 1.7 ms | 5.0x |
| wenig | 587 kB, 85 obj. | 18.1 ms | 10.5 ms | 1.7x |
| gosper-curve | 182 kB, 142 obj. | 9.0 ms | 6.4 ms | 1.4x |
| penrose-tiling | 560 kB, 7615 obj. | 21.3 ms | 3.2 ms | 6.6x |
| apollonian-gasket | 330 kB, 4221 obj. | 12.4 ms | 3.3 ms | 3.8x |
| blankmap | 1.0 MB, 2232 obj. | 29.0 ms | 3.6 ms | 8.2x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 59.6 ms | 9.9 ms | 6.0x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 73.6 ms | 14.0 ms | 5.2x |
| greece | 3.2 MB, 2013 obj. | 46.2 ms | 7.4 ms | 6.3x |
| clifford-attractor | 13.2 MB, 250001 obj. | 134 ms | 57.2 ms | 2.3x |
| cube8 | 14.6 MB, 190260 obj. | 112 ms | 116 ms | 1.0x |

#### Undo the move

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 4.4 ms | 6.8 ms | 0.7x |
| feynman (small) | 4 kB, 34 obj. | 6.0 ms | 4.4 ms | 1.4x |
| fourier (small) | 16 kB, 41 obj. | 2.8 ms | 2.9 ms | 1.0x |
| pines | 11 kB, 31 obj. | 7.4 ms | 1.7 ms | 4.3x |
| snowy | 173 kB, 436 obj. | 4.5 ms | 2.4 ms | 1.8x |
| wenig | 587 kB, 85 obj. | 39.8 ms | 5.3 ms | 7.4x |
| gosper-curve | 182 kB, 142 obj. | 5.0 ms | 2.4 ms | 2.1x |
| penrose-tiling | 560 kB, 7615 obj. | 6.1 ms | 5.6 ms | 1.1x |
| apollonian-gasket | 330 kB, 4221 obj. | 21.2 ms | 6.3 ms | 3.3x |
| blankmap | 1.0 MB, 2232 obj. | 50.2 ms | 5.3 ms | 9.5x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 39.5 ms | 16.7 ms | 2.4x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 44.5 ms | 22.9 ms | 1.9x |
| greece | 3.2 MB, 2013 obj. | 30.1 ms | 19.1 ms | 1.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 33.3 ms | 85.6 ms | 0.4x |
| cube8 | 14.6 MB, 190260 obj. | 38.7 ms | 417 ms | 0.1x |

#### Cross-check: the same actions timed from outside

xfig: its own hooks against watching the canvas pixels; RustFig: its frame times against the wall time until `ui.state` reports the canvas current (never below one frame, 16.7 ms).

| Figure | Task | xfig, own hooks | xfig, pixels | RustFig, frame times | RustFig, wall to idle |
|---|---|---|---|---|---|
| carnot (small) | open | 1.14 s | 1.18 s | 192 ms | 192 ms |
| carnot (small) | zoom_in | 28.3 ms | 29.3 ms | 4.2 ms | 5.3 ms |
| carnot (small) | zoom_out | 8.0 ms | 34.4 ms | 4.2 ms | 6.1 ms |
| carnot (small) | pan | 4.8 ms | 10.1 ms | 2.8 ms | 4.2 ms |
| carnot (small) | move_step | 0.8 ms | 9.6 ms | 0.1 ms | 3.6 ms |
| carnot (small) | move_place | 5.2 ms | 55.4 ms | 7.6 ms | 13.6 ms |
| carnot (small) | save | 3.2 ms | 34.4 ms | 1.4 ms | n/a |
| carnot (small) | undo | 4.4 ms | 35.6 ms | 6.8 ms | 10.4 ms |
| feynman (small) | open | 1.14 s | 1.17 s | 187 ms | 187 ms |
| feynman (small) | zoom_in | 33.0 ms | 34.8 ms | 2.9 ms | 7.3 ms |
| feynman (small) | zoom_out | 7.1 ms | 35.3 ms | 3.4 ms | 4.7 ms |
| feynman (small) | pan | 3.9 ms | 10.2 ms | 2.9 ms | 4.2 ms |
| feynman (small) | move_step | 9.1 ms | 9.1 ms | 0.1 ms | 3.7 ms |
| feynman (small) | move_place | 51.6 ms | 51.6 ms | 4.6 ms | 12.1 ms |
| feynman (small) | save | 2.3 ms | 34.0 ms | 1.2 ms | n/a |
| feynman (small) | undo | 6.0 ms | 35.2 ms | 4.4 ms | 7.2 ms |
| fourier (small) | open | 1.13 s | 1.17 s | 193 ms | 193 ms |
| fourier (small) | zoom_in | 28.8 ms | 29.5 ms | 3.3 ms | 6.1 ms |
| fourier (small) | zoom_out | 6.1 ms | 35.3 ms | 3.7 ms | 8.3 ms |
| fourier (small) | pan | 4.7 ms | 9.8 ms | 3.4 ms | 4.2 ms |
| fourier (small) | move_step | 0.8 ms | 9.4 ms | 0.1 ms | 4.7 ms |
| fourier (small) | move_place | 4.7 ms | 55.3 ms | 3.3 ms | 9.5 ms |
| fourier (small) | save | 3.0 ms | 34.4 ms | 1.5 ms | n/a |
| fourier (small) | undo | 2.8 ms | 33.9 ms | 2.9 ms | 5.8 ms |
| pines | open | 1.11 s | 1.15 s | 176 ms | 176 ms |
| pines | zoom_in | 8.4 ms | 33.4 ms | 1.6 ms | 6.3 ms |
| pines | zoom_out | 9.1 ms | 32.0 ms | 1.7 ms | 3.1 ms |
| pines | pan | 4.3 ms | 10.5 ms | 1.6 ms | 2.9 ms |
| pines | move_step | 1.0 ms | 9.3 ms | 0.1 ms | 4.9 ms |
| pines | move_place | 8.0 ms | 54.1 ms | 1.7 ms | 8.1 ms |
| pines | save | 2.7 ms | 35.1 ms | 1.3 ms | n/a |
| pines | undo | 7.4 ms | 34.9 ms | 1.7 ms | 4.8 ms |
| snowy | open | 1.12 s | 1.15 s | 193 ms | 193 ms |
| snowy | zoom_in | 8.6 ms | 31.3 ms | 1.0 ms | 4.0 ms |
| snowy | zoom_out | 8.9 ms | 34.0 ms | 1.1 ms | 2.4 ms |
| snowy | pan | 6.8 ms | 10.6 ms | 0.9 ms | 2.2 ms |
| snowy | move_step | 0.5 ms | 9.5 ms | 0.1 ms | 4.6 ms |
| snowy | move_place | 15.4 ms | 51.2 ms | 2.3 ms | 7.1 ms |
| snowy | save | 8.2 ms | 32.8 ms | 1.7 ms | n/a |
| snowy | undo | 4.5 ms | 34.4 ms | 2.4 ms | 5.3 ms |
| wenig | open | 1.13 s | 1.17 s | 184 ms | 184 ms |
| wenig | zoom_in | 20.6 ms | 29.8 ms | 3.0 ms | 4.5 ms |
| wenig | zoom_out | 18.7 ms | 29.5 ms | 3.2 ms | 4.5 ms |
| wenig | pan | 17.7 ms | 19.6 ms | 2.7 ms | 3.9 ms |
| wenig | move_step | 2.2 ms | 9.5 ms | 0.1 ms | 3.3 ms |
| wenig | move_place | 38.6 ms | 49.6 ms | 5.3 ms | 44.2 ms |
| wenig | save | 18.1 ms | 32.8 ms | 10.5 ms | n/a |
| wenig | undo | 39.8 ms | 43.1 ms | 5.3 ms | 25.0 ms |
| gosper-curve | open | 1.16 s | 1.19 s | 178 ms | 178 ms |
| gosper-curve | zoom_in | 8.7 ms | 36.5 ms | 5.0 ms | 6.3 ms |
| gosper-curve | zoom_out | 8.7 ms | 41.1 ms | 1.7 ms | 2.9 ms |
| gosper-curve | pan | 4.9 ms | 10.2 ms | 1.7 ms | 3.1 ms |
| gosper-curve | move_step | 0.6 ms | 9.7 ms | 0.1 ms | 3.8 ms |
| gosper-curve | move_place | 18.9 ms | 60.1 ms | 2.0 ms | 24.9 ms |
| gosper-curve | save | 9.0 ms | 36.3 ms | 6.4 ms | n/a |
| gosper-curve | undo | 5.0 ms | 40.6 ms | 2.4 ms | 10.7 ms |
| penrose-tiling | open | 1.19 s | 1.23 s | 191 ms | 191 ms |
| penrose-tiling | zoom_in | 21.9 ms | 36.3 ms | 4.5 ms | 6.5 ms |
| penrose-tiling | zoom_out | 24.3 ms | 32.7 ms | 5.0 ms | 6.3 ms |
| penrose-tiling | pan | 15.7 ms | 15.3 ms | 4.1 ms | 5.5 ms |
| penrose-tiling | move_step | 0.5 ms | 9.9 ms | 0.1 ms | 5.7 ms |
| penrose-tiling | move_place | 23.6 ms | 51.4 ms | 5.3 ms | 12.9 ms |
| penrose-tiling | save | 21.3 ms | 38.7 ms | 3.2 ms | n/a |
| penrose-tiling | undo | 6.1 ms | 40.9 ms | 5.6 ms | 8.5 ms |
| apollonian-gasket | open | 1.17 s | 1.20 s | 198 ms | 198 ms |
| apollonian-gasket | zoom_in | 10.7 ms | 40.3 ms | 2.4 ms | 3.9 ms |
| apollonian-gasket | zoom_out | 10.9 ms | 31.8 ms | 2.5 ms | 3.9 ms |
| apollonian-gasket | pan | 12.0 ms | 10.9 ms | 1.9 ms | 3.3 ms |
| apollonian-gasket | move_step | 1.3 ms | 9.8 ms | 0.1 ms | 3.2 ms |
| apollonian-gasket | move_place | 18.7 ms | 58.9 ms | 6.1 ms | 10.9 ms |
| apollonian-gasket | save | 12.4 ms | 31.4 ms | 3.3 ms | n/a |
| apollonian-gasket | undo | 21.2 ms | 38.1 ms | 6.3 ms | 9.5 ms |
| blankmap | open | 1.20 s | 1.24 s | 198 ms | 198 ms |
| blankmap | zoom_in | 14.7 ms | 36.8 ms | 2.4 ms | 7.3 ms |
| blankmap | zoom_out | 14.9 ms | 37.7 ms | 2.8 ms | 4.1 ms |
| blankmap | pan | 17.9 ms | 15.7 ms | 1.8 ms | 3.1 ms |
| blankmap | move_step | 1.3 ms | 10.1 ms | 0.1 ms | 4.2 ms |
| blankmap | move_place | 50.3 ms | 51.1 ms | 5.8 ms | 11.2 ms |
| blankmap | save | 29.0 ms | 34.7 ms | 3.6 ms | n/a |
| blankmap | undo | 50.2 ms | 50.4 ms | 5.3 ms | 8.2 ms |
| ne-coastline-10m | open | 1.26 s | 1.29 s | 242 ms | 242 ms |
| ne-coastline-10m | zoom_in | 12.1 ms | 37.9 ms | 4.7 ms | 6.2 ms |
| ne-coastline-10m | zoom_out | 13.1 ms | 36.4 ms | 5.8 ms | 7.5 ms |
| ne-coastline-10m | pan | 12.1 ms | 11.1 ms | 3.3 ms | 7.0 ms |
| ne-coastline-10m | move_step | 1.6 ms | 9.9 ms | 0.1 ms | 4.7 ms |
| ne-coastline-10m | move_place | 130 ms | 132 ms | 14.3 ms | 19.5 ms |
| ne-coastline-10m | save | 59.6 ms | 61.9 ms | 9.9 ms | n/a |
| ne-coastline-10m | undo | 39.5 ms | 41.3 ms | 16.7 ms | 19.7 ms |
| ne-countries-10m | open | 1.25 s | 1.29 s | 264 ms | 264 ms |
| ne-countries-10m | zoom_in | 19.7 ms | 32.9 ms | 7.3 ms | 9.1 ms |
| ne-countries-10m | zoom_out | 19.0 ms | 29.5 ms | 8.7 ms | 10.1 ms |
| ne-countries-10m | pan | 21.2 ms | 21.9 ms | 5.1 ms | 6.6 ms |
| ne-countries-10m | move_step | 1.0 ms | 9.9 ms | 0.1 ms | 3.1 ms |
| ne-countries-10m | move_place | 160 ms | 163 ms | 20.6 ms | 26.2 ms |
| ne-countries-10m | save | 73.6 ms | 74.8 ms | 14.0 ms | n/a |
| ne-countries-10m | undo | 44.5 ms | 46.3 ms | 22.9 ms | 27.8 ms |
| greece | open | 1.25 s | 1.28 s | 302 ms | 302 ms |
| greece | zoom_in | 14.2 ms | 29.5 ms | 14.5 ms | 16.1 ms |
| greece | zoom_out | 15.9 ms | 29.7 ms | 15.1 ms | 16.7 ms |
| greece | pan | 11.8 ms | 14.4 ms | 13.6 ms | 15.2 ms |
| greece | move_step | 0.5 ms | 9.9 ms | 0.1 ms | 3.0 ms |
| greece | move_place | 101 ms | 102 ms | 19.0 ms | 28.9 ms |
| greece | save | 46.2 ms | 48.2 ms | 7.4 ms | n/a |
| greece | undo | 30.1 ms | 35.0 ms | 19.1 ms | 22.1 ms |
| clifford-attractor | open | 1.31 s | 1.35 s | 476 ms | 476 ms |
| clifford-attractor | zoom_in | 61.4 ms | 64.1 ms | 39.9 ms | 47.9 ms |
| clifford-attractor | zoom_out | 60.9 ms | 61.6 ms | 51.2 ms | 51.7 ms |
| clifford-attractor | pan | 41.5 ms | 42.0 ms | 44.4 ms | 46.8 ms |
| clifford-attractor | move_step | 0.5 ms | 9.7 ms | 0.1 ms | 5.2 ms |
| clifford-attractor | move_place | 99.1 ms | 102 ms | 71.4 ms | 89.9 ms |
| clifford-attractor | save | 134 ms | 136 ms | 57.2 ms | n/a |
| clifford-attractor | undo | 33.3 ms | 35.3 ms | 85.6 ms | 89.0 ms |
| cube8 | open | 1.33 s | 1.37 s | 1.39 s | 1.39 s |
| cube8 | zoom_in | 66.6 ms | 66.4 ms | 229 ms | 232 ms |
| cube8 | zoom_out | 79.7 ms | 80.2 ms | 261 ms | 254 ms |
| cube8 | pan | 67.6 ms | 68.6 ms | 182 ms | 188 ms |
| cube8 | move_step | 0.5 ms | 9.7 ms | 0.2 ms | 8.8 ms |
| cube8 | move_place | 141 ms | 142 ms | 341 ms | 361 ms |
| cube8 | save | 112 ms | 115 ms | 116 ms | n/a |
| cube8 | undo | 38.7 ms | 39.1 ms | 417 ms | 417 ms |

<details><summary>Raw numbers (every run, milliseconds)</summary>

| Figure | App | Task | Runs |
|---|---|---|---|
| carnot (small) | xfig | open | 1143.4; 1142.9; 1155.8 |
| carnot (small) | xfig | open_read | 37.2; 44.6; 42.8 |
| carnot (small) | xfig | open_draw | 1.9; 1.8; 2.2 |
| carnot (small) | xfig | zoom_in | 13.9; 29.0; 28.3 |
| carnot (small) | xfig | zoom_out | 8.0; 6.8; 8.2 |
| carnot (small) | xfig | pan | 4.8; 4.8; 5.0 |
| carnot (small) | xfig | move_step | 0.8; 0.8; 0.5 |
| carnot (small) | xfig | move_step_max | 1.0; 1.0; 0.9 |
| carnot (small) | xfig | move_place | 5.2; 5.2; 6.6 |
| carnot (small) | xfig | save | 3.2; 2.6; 3.7 |
| carnot (small) | xfig | undo | 4.2; 4.4; 5.0 |
| carnot (small) | xfig | x_open | 1181.0; 1179.0; 1212.4 |
| carnot (small) | xfig | x_zoom_in | 29.3; 31.8; 29.0 |
| carnot (small) | xfig | x_zoom_out | 31.5; 34.4; 35.9 |
| carnot (small) | xfig | x_pan | 10.1; 9.8; 11.9 |
| carnot (small) | xfig | x_move_step | 9.2; 9.6; 10.7 |
| carnot (small) | xfig | x_move_place | 52.1; 55.4; 57.2 |
| carnot (small) | xfig | x_save | 34.4; 29.1; 36.4 |
| carnot (small) | xfig | x_undo | 35.6; 34.3; 36.5 |
| carnot (small) | RustFig | open | 285.7; 180.7; 191.9 |
| carnot (small) | RustFig | open_read | -; -; - |
| carnot (small) | RustFig | open_draw | -; -; - |
| carnot (small) | RustFig | zoom_in | 4.7; 4.2; 4.1 |
| carnot (small) | RustFig | zoom_out | 4.2; 4.0; 4.6 |
| carnot (small) | RustFig | pan | 2.8; 2.8; 2.8 |
| carnot (small) | RustFig | move_step | 0.1; 0.1; 0.1 |
| carnot (small) | RustFig | move_step_max | 0.2; 7.5; 0.2 |
| carnot (small) | RustFig | move_place | 7.7; 7.1; 7.6 |
| carnot (small) | RustFig | save | 1.6; 1.2; 1.4 |
| carnot (small) | RustFig | undo | 6.7; 7.0; 6.8 |
| carnot (small) | RustFig | x_open | 285.7; 180.7; 191.9 |
| carnot (small) | RustFig | x_zoom_in | 5.3; 5.1; 5.3 |
| carnot (small) | RustFig | x_zoom_out | 9.1; 5.4; 6.1 |
| carnot (small) | RustFig | x_pan | 4.2; 8.5; 4.2 |
| carnot (small) | RustFig | x_move_step | 3.6; 4.0; 3.4 |
| carnot (small) | RustFig | x_move_place | 13.6; 12.6; 17.4 |
| carnot (small) | RustFig | x_save | -; -; - |
| carnot (small) | RustFig | x_undo | 9.7; 10.4; 11.3 |
| feynman (small) | xfig | open | 1149.9; 1136.2; 1133.8 |
| feynman (small) | xfig | open_read | 37.4; 42.6; 44.5 |
| feynman (small) | xfig | open_draw | 2.0; 1.7; 1.4 |
| feynman (small) | xfig | zoom_in | 34.2; 33.0; 32.7 |
| feynman (small) | xfig | zoom_out | 7.3; 6.2; 7.1 |
| feynman (small) | xfig | pan | 4.5; 3.9; 3.3 |
| feynman (small) | xfig | move_step | 9.1; 9.0; 9.4 |
| feynman (small) | xfig | move_step_max | 10.4; 10.9; 11.0 |
| feynman (small) | xfig | move_place | 57.1; 50.8; 51.6 |
| feynman (small) | xfig | save | 2.3; 2.5; 2.1 |
| feynman (small) | xfig | undo | 5.6; 6.0; 6.1 |
| feynman (small) | xfig | x_open | 1186.7; 1172.1; 1172.7 |
| feynman (small) | xfig | x_zoom_in | 36.2; 34.8; 32.3 |
| feynman (small) | xfig | x_zoom_out | 35.8; 35.3; 32.3 |
| feynman (small) | xfig | x_pan | 11.7; 10.2; 8.8 |
| feynman (small) | xfig | x_move_step | 9.1; 9.0; 9.4 |
| feynman (small) | xfig | x_move_place | 57.1; 50.8; 51.6 |
| feynman (small) | xfig | x_save | 34.0; 35.5; 31.4 |
| feynman (small) | xfig | x_undo | 35.2; 31.1; 35.6 |
| feynman (small) | RustFig | open | 214.5; 183.0; 186.9 |
| feynman (small) | RustFig | open_read | -; -; - |
| feynman (small) | RustFig | open_draw | -; -; - |
| feynman (small) | RustFig | zoom_in | 3.0; 2.8; 2.9 |
| feynman (small) | RustFig | zoom_out | 3.4; 2.9; 3.4 |
| feynman (small) | RustFig | pan | 2.9; 1.9; 2.9 |
| feynman (small) | RustFig | move_step | 0.1; 0.1; 0.1 |
| feynman (small) | RustFig | move_step_max | 5.2; 0.2; 0.2 |
| feynman (small) | RustFig | move_place | 4.4; 4.6; 4.6 |
| feynman (small) | RustFig | save | 1.2; 1.2; 1.2 |
| feynman (small) | RustFig | undo | 4.4; 4.3; 4.4 |
| feynman (small) | RustFig | x_open | 214.5; 183.0; 186.9 |
| feynman (small) | RustFig | x_zoom_in | 7.4; 4.0; 7.3 |
| feynman (small) | RustFig | x_zoom_out | 4.7; 4.2; 4.8 |
| feynman (small) | RustFig | x_pan | 4.1; 7.1; 4.2 |
| feynman (small) | RustFig | x_move_step | 3.1; 3.7; 4.1 |
| feynman (small) | RustFig | x_move_place | 12.1; 11.1; 14.9 |
| feynman (small) | RustFig | x_save | -; -; - |
| feynman (small) | RustFig | x_undo | 7.2; 7.2; 9.5 |
| fourier (small) | xfig | open | 1141.7; 1126.9; 1134.1 |
| fourier (small) | xfig | open_read | 38.2; 41.0; 41.8 |
| fourier (small) | xfig | open_draw | 1.1; 1.5; 1.2 |
| fourier (small) | xfig | zoom_in | 28.8; 29.0; 25.6 |
| fourier (small) | xfig | zoom_out | 6.1; 6.1; 6.0 |
| fourier (small) | xfig | pan | 4.7; 4.0; 4.7 |
| fourier (small) | xfig | move_step | 0.8; 0.8; 0.8 |
| fourier (small) | xfig | move_step_max | 0.9; 1.0; 1.0 |
| fourier (small) | xfig | move_place | 5.3; 4.7; 4.5 |
| fourier (small) | xfig | save | 2.7; 3.1; 3.0 |
| fourier (small) | xfig | undo | 2.9; 2.8; 2.8 |
| fourier (small) | xfig | x_open | 1178.0; 1163.9; 1173.7 |
| fourier (small) | xfig | x_zoom_in | 31.8; 29.5; 26.0 |
| fourier (small) | xfig | x_zoom_out | 35.3; 35.1; 35.5 |
| fourier (small) | xfig | x_pan | 9.8; 9.3; 10.7 |
| fourier (small) | xfig | x_move_step | 9.5; 9.4; 9.2 |
| fourier (small) | xfig | x_move_place | 55.2; 56.1; 55.3 |
| fourier (small) | xfig | x_save | 34.4; 33.1; 35.1 |
| fourier (small) | xfig | x_undo | 34.3; 31.1; 33.9 |
| fourier (small) | RustFig | open | 200.1; 192.3; 193.0 |
| fourier (small) | RustFig | open_read | -; -; - |
| fourier (small) | RustFig | open_draw | -; -; - |
| fourier (small) | RustFig | zoom_in | 3.3; 3.3; 3.7 |
| fourier (small) | RustFig | zoom_out | 3.7; 3.0; 3.7 |
| fourier (small) | RustFig | pan | 3.8; 3.4; 3.2 |
| fourier (small) | RustFig | move_step | 0.1; 0.1; 0.1 |
| fourier (small) | RustFig | move_step_max | 0.2; 0.2; 0.2 |
| fourier (small) | RustFig | move_place | 2.9; 3.3; 3.4 |
| fourier (small) | RustFig | save | 1.5; 1.5; 1.5 |
| fourier (small) | RustFig | undo | 2.9; 2.9; 3.0 |
| fourier (small) | RustFig | x_open | 200.1; 192.3; 193.0 |
| fourier (small) | RustFig | x_zoom_in | 6.1; 7.3; 5.3 |
| fourier (small) | RustFig | x_zoom_out | 8.3; 6.3; 8.4 |
| fourier (small) | RustFig | x_pan | 4.1; 4.2; 4.3 |
| fourier (small) | RustFig | x_move_step | 4.7; 3.2; 4.7 |
| fourier (small) | RustFig | x_move_place | 9.3; 9.5; 9.6 |
| fourier (small) | RustFig | x_save | -; -; - |
| fourier (small) | RustFig | x_undo | 5.2; 8.3; 5.8 |
| pines | xfig | open | 1103.8; 1113.9; 1114.8 |
| pines | xfig | open_read | 2.4; 2.8; 2.7 |
| pines | xfig | open_draw | 7.4; 6.7; 7.2 |
| pines | xfig | zoom_in | 6.4; 8.4; 9.2 |
| pines | xfig | zoom_out | 10.3; 8.1; 9.1 |
| pines | xfig | pan | 4.3; 4.5; 4.3 |
| pines | xfig | move_step | 1.0; 1.0; 0.8 |
| pines | xfig | move_step_max | 1.2; 1.1; 1.1 |
| pines | xfig | move_place | 8.0; 8.3; 6.7 |
| pines | xfig | save | 3.0; 2.7; 2.5 |
| pines | xfig | undo | 7.4; 8.5; 7.0 |
| pines | xfig | x_open | 1148.0; 1150.2; 1156.4 |
| pines | xfig | x_zoom_in | 33.4; 30.7; 33.9 |
| pines | xfig | x_zoom_out | 32.0; 31.7; 34.1 |
| pines | xfig | x_pan | 10.0; 10.7; 10.5 |
| pines | xfig | x_move_step | 9.3; 9.4; 8.9 |
| pines | xfig | x_move_place | 55.1; 54.1; 53.7 |
| pines | xfig | x_save | 35.1; 35.5; 33.3 |
| pines | xfig | x_undo | 30.4; 35.0; 34.9 |
| pines | RustFig | open | 200.5; 176.4; 175.1 |
| pines | RustFig | open_read | -; -; - |
| pines | RustFig | open_draw | -; -; - |
| pines | RustFig | zoom_in | 1.5; 1.6; 1.6 |
| pines | RustFig | zoom_out | 1.7; 1.7; 1.7 |
| pines | RustFig | pan | 1.7; 1.6; 1.6 |
| pines | RustFig | move_step | 0.1; 0.1; 0.1 |
| pines | RustFig | move_step_max | 0.2; 0.1; 0.1 |
| pines | RustFig | move_place | 1.6; 1.9; 1.7 |
| pines | RustFig | save | 1.3; 306.8; 1.3 |
| pines | RustFig | undo | 1.7; 2.0; 1.7 |
| pines | RustFig | x_open | 200.5; 176.4; 175.1 |
| pines | RustFig | x_zoom_in | 6.3; 6.3; 2.9 |
| pines | RustFig | x_zoom_out | 3.1; 2.9; 6.5 |
| pines | RustFig | x_pan | 2.9; 2.9; 2.9 |
| pines | RustFig | x_move_step | 5.1; 4.9; 4.6 |
| pines | RustFig | x_move_place | 8.0; 8.3; 8.1 |
| pines | RustFig | x_save | -; -; - |
| pines | RustFig | x_undo | 4.8; 5.3; 4.8 |
| snowy | xfig | open | 1120.3; 1114.3; 1115.3 |
| snowy | xfig | open_read | 7.9; 7.9; 8.6 |
| snowy | xfig | open_draw | 4.8; 5.6; 6.5 |
| snowy | xfig | zoom_in | 8.6; 8.6; 8.0 |
| snowy | xfig | zoom_out | 8.9; 9.1; 8.9 |
| snowy | xfig | pan | 6.9; 6.8; 6.2 |
| snowy | xfig | move_step | 0.5; 0.5; 0.5 |
| snowy | xfig | move_step_max | 0.6; 0.6; 0.6 |
| snowy | xfig | move_place | 17.7; 15.4; 14.8 |
| snowy | xfig | save | 8.2; 7.9; 8.6 |
| snowy | xfig | undo | 2.5; 4.9; 4.5 |
| snowy | xfig | x_open | 1159.4; 1152.7; 1150.3 |
| snowy | xfig | x_zoom_in | 31.1; 31.6; 31.3 |
| snowy | xfig | x_zoom_out | 34.7; 34.0; 31.5 |
| snowy | xfig | x_pan | 10.6; 10.7; 9.2 |
| snowy | xfig | x_move_step | 9.6; 9.5; 9.3 |
| snowy | xfig | x_move_place | 51.1; 52.3; 51.2 |
| snowy | xfig | x_save | 32.8; 35.8; 31.0 |
| snowy | xfig | x_undo | 34.4; 35.3; 32.6 |
| snowy | RustFig | open | 193.0; 192.7; 179.5 |
| snowy | RustFig | open_read | -; -; - |
| snowy | RustFig | open_draw | -; -; - |
| snowy | RustFig | zoom_in | 1.0; 1.0; 1.0 |
| snowy | RustFig | zoom_out | 1.1; 1.1; 1.1 |
| snowy | RustFig | pan | 0.9; 1.0; 0.9 |
| snowy | RustFig | move_step | 0.1; 0.1; 0.1 |
| snowy | RustFig | move_step_max | 0.2; 0.5; 0.1 |
| snowy | RustFig | move_place | 2.3; 2.8; 2.3 |
| snowy | RustFig | save | 1.7; 1.6; 1.7 |
| snowy | RustFig | undo | 2.4; 2.4; 2.4 |
| snowy | RustFig | x_open | 193.0; 192.7; 179.5 |
| snowy | RustFig | x_zoom_in | 5.7; 2.6; 4.0 |
| snowy | RustFig | x_zoom_out | 2.3; 2.4; 6.7 |
| snowy | RustFig | x_pan | 2.2; 5.2; 2.2 |
| snowy | RustFig | x_move_step | 4.6; 3.6; 6.3 |
| snowy | RustFig | x_move_place | 9.5; 7.1; 6.2 |
| snowy | RustFig | x_save | -; -; - |
| snowy | RustFig | x_undo | 6.7; 5.2; 5.3 |
| wenig | xfig | open | 1134.0; 1125.5; 1191.8 |
| wenig | xfig | open_read | 17.8; 23.0; 24.3 |
| wenig | xfig | open_draw | 8.0; 10.9; 10.1 |
| wenig | xfig | zoom_in | 21.0; 19.8; 20.6 |
| wenig | xfig | zoom_out | 18.9; 18.7; 18.0 |
| wenig | xfig | pan | 17.7; 17.5; 19.3 |
| wenig | xfig | move_step | 2.1; 2.2; 2.2 |
| wenig | xfig | move_step_max | 2.3; 2.9; 2.5 |
| wenig | xfig | move_place | 37.7; 38.6; 40.2 |
| wenig | xfig | save | 22.0; 18.1; 16.9 |
| wenig | xfig | undo | 39.8; 39.8; 42.9 |
| wenig | xfig | x_open | 1171.0; 1170.5; 1225.1 |
| wenig | xfig | x_zoom_in | 26.3; 30.9; 29.8 |
| wenig | xfig | x_zoom_out | 32.1; 28.0; 29.5 |
| wenig | xfig | x_pan | 19.5; 19.6; 22.1 |
| wenig | xfig | x_move_step | 8.9; 9.5; 10.0 |
| wenig | xfig | x_move_place | 47.8; 49.6; 54.8 |
| wenig | xfig | x_save | 32.8; 32.9; 31.8 |
| wenig | xfig | x_undo | 40.8; 43.1; 43.1 |
| wenig | RustFig | open | 211.6; 183.8; 175.8 |
| wenig | RustFig | open_read | -; -; - |
| wenig | RustFig | open_draw | -; -; - |
| wenig | RustFig | zoom_in | 3.0; 3.0; 3.2 |
| wenig | RustFig | zoom_out | 3.2; 3.8; 3.1 |
| wenig | RustFig | pan | 2.6; 2.7; 2.7 |
| wenig | RustFig | move_step | 0.1; 0.1; 0.1 |
| wenig | RustFig | move_step_max | 0.1; 0.2; 0.2 |
| wenig | RustFig | move_place | 5.3; 5.0; 5.5 |
| wenig | RustFig | save | 10.5; 10.2; 11.1 |
| wenig | RustFig | undo | 5.6; 5.2; 5.3 |
| wenig | RustFig | x_open | 211.6; 183.8; 175.8 |
| wenig | RustFig | x_zoom_in | 4.4; 7.2; 4.5 |
| wenig | RustFig | x_zoom_out | 4.5; 5.2; 4.5 |
| wenig | RustFig | x_pan | 3.9; 3.9; 4.0 |
| wenig | RustFig | x_move_step | 2.9; 4.3; 3.3 |
| wenig | RustFig | x_move_place | 44.2; 44.2; 43.7 |
| wenig | RustFig | x_save | -; -; - |
| wenig | RustFig | x_undo | 25.5; 25.0; 25.0 |
| gosper-curve | xfig | open | 1159.6; 1149.3; 1160.3 |
| gosper-curve | xfig | open_read | 10.9; 12.1; 8.5 |
| gosper-curve | xfig | open_draw | 4.5; 4.3; 5.2 |
| gosper-curve | xfig | zoom_in | 8.1; 8.8; 8.7 |
| gosper-curve | xfig | zoom_out | 8.7; 7.7; 8.7 |
| gosper-curve | xfig | pan | 5.6; 4.8; 4.9 |
| gosper-curve | xfig | move_step | 0.6; 0.7; 0.6 |
| gosper-curve | xfig | move_step_max | 0.8; 0.8; 0.8 |
| gosper-curve | xfig | move_place | 18.9; 18.6; 19.7 |
| gosper-curve | xfig | save | 9.0; 5.1; 9.1 |
| gosper-curve | xfig | undo | 5.0; 5.7; 4.3 |
| gosper-curve | xfig | x_open | 1198.5; 1181.2; 1192.8 |
| gosper-curve | xfig | x_zoom_in | 40.2; 36.5; 33.4 |
| gosper-curve | xfig | x_zoom_out | 40.3; 41.1; 41.4 |
| gosper-curve | xfig | x_pan | 11.8; 10.2; 10.0 |
| gosper-curve | xfig | x_move_step | 9.8; 9.5; 9.7 |
| gosper-curve | xfig | x_move_place | 60.3; 60.1; 59.9 |
| gosper-curve | xfig | x_save | 36.3; 40.4; 34.9 |
| gosper-curve | xfig | x_undo | 41.2; 40.6; 39.2 |
| gosper-curve | RustFig | open | 195.7; 178.0; 177.7 |
| gosper-curve | RustFig | open_read | -; -; - |
| gosper-curve | RustFig | open_draw | -; -; - |
| gosper-curve | RustFig | zoom_in | 6.0; 4.9; 5.0 |
| gosper-curve | RustFig | zoom_out | 1.7; 1.7; 1.7 |
| gosper-curve | RustFig | pan | 1.7; 1.7; 1.6 |
| gosper-curve | RustFig | move_step | 0.1; 0.1; 0.1 |
| gosper-curve | RustFig | move_step_max | 0.2; 0.1; 0.2 |
| gosper-curve | RustFig | move_place | 1.9; 2.0; 2.1 |
| gosper-curve | RustFig | save | 6.4; 8.8; 4.4 |
| gosper-curve | RustFig | undo | 2.4; 2.5; 2.3 |
| gosper-curve | RustFig | x_open | 195.7; 178.0; 177.7 |
| gosper-curve | RustFig | x_zoom_in | 7.3; 6.3; 6.3 |
| gosper-curve | RustFig | x_zoom_out | 2.9; 2.9; 3.0 |
| gosper-curve | RustFig | x_pan | 3.1; 3.2; 2.8 |
| gosper-curve | RustFig | x_move_step | 4.8; 3.8; 2.8 |
| gosper-curve | RustFig | x_move_place | 21.5; 24.9; 27.3 |
| gosper-curve | RustFig | x_save | -; -; - |
| gosper-curve | RustFig | x_undo | 10.0; 15.6; 10.7 |
| penrose-tiling | xfig | open | 1174.4; 1194.0; 1192.4 |
| penrose-tiling | xfig | open_read | 16.0; 22.0; 23.7 |
| penrose-tiling | xfig | open_draw | 12.9; 12.3; 11.9 |
| penrose-tiling | xfig | zoom_in | 21.9; 22.1; 20.7 |
| penrose-tiling | xfig | zoom_out | 24.9; 24.3; 23.7 |
| penrose-tiling | xfig | pan | 12.7; 20.9; 15.7 |
| penrose-tiling | xfig | move_step | 0.5; 0.5; 0.6 |
| penrose-tiling | xfig | move_step_max | 0.6; 0.6; 0.9 |
| penrose-tiling | xfig | move_place | 23.4; 24.4; 23.6 |
| penrose-tiling | xfig | save | 21.3; 18.9; 21.9 |
| penrose-tiling | xfig | undo | 4.6; 6.1; 6.3 |
| penrose-tiling | xfig | x_open | 1211.1; 1232.2; 1228.7 |
| penrose-tiling | xfig | x_zoom_in | 29.5; 36.3; 38.2 |
| penrose-tiling | xfig | x_zoom_out | 36.8; 32.7; 28.8 |
| penrose-tiling | xfig | x_pan | 13.6; 15.3; 16.1 |
| penrose-tiling | xfig | x_move_step | 9.9; 9.8; 10.0 |
| penrose-tiling | xfig | x_move_place | 58.5; 51.4; 51.0 |
| penrose-tiling | xfig | x_save | 38.9; 38.7; 28.9 |
| penrose-tiling | xfig | x_undo | 40.9; 41.1; 36.8 |
| penrose-tiling | RustFig | open | 223.4; 190.6; 185.1 |
| penrose-tiling | RustFig | open_read | -; -; - |
| penrose-tiling | RustFig | open_draw | -; -; - |
| penrose-tiling | RustFig | zoom_in | 4.3; 4.9; 4.5 |
| penrose-tiling | RustFig | zoom_out | 5.0; 5.4; 5.0 |
| penrose-tiling | RustFig | pan | 4.1; 3.9; 4.1 |
| penrose-tiling | RustFig | move_step | 0.1; 0.1; 0.1 |
| penrose-tiling | RustFig | move_step_max | 0.2; 0.1; 0.2 |
| penrose-tiling | RustFig | move_place | 5.3; 5.2; 5.3 |
| penrose-tiling | RustFig | save | 3.2; 3.1; 3.2 |
| penrose-tiling | RustFig | undo | 5.5; 5.8; 5.6 |
| penrose-tiling | RustFig | x_open | 223.4; 190.6; 185.1 |
| penrose-tiling | RustFig | x_zoom_in | 5.6; 6.5; 10.3 |
| penrose-tiling | RustFig | x_zoom_out | 6.3; 6.8; 6.3 |
| penrose-tiling | RustFig | x_pan | 5.4; 10.4; 5.5 |
| penrose-tiling | RustFig | x_move_step | 6.3; 3.0; 5.7 |
| penrose-tiling | RustFig | x_move_place | 13.4; 10.6; 12.9 |
| penrose-tiling | RustFig | x_save | -; -; - |
| penrose-tiling | RustFig | x_undo | 8.5; 8.8; 8.5 |
| apollonian-gasket | xfig | open | 1184.7; 1152.3; 1169.7 |
| apollonian-gasket | xfig | open_read | 10.7; 7.6; 11.5 |
| apollonian-gasket | xfig | open_draw | 8.9; 6.6; 6.1 |
| apollonian-gasket | xfig | zoom_in | 10.5; 10.8; 10.7 |
| apollonian-gasket | xfig | zoom_out | 10.9; 10.8; 12.5 |
| apollonian-gasket | xfig | pan | 12.6; 10.6; 12.0 |
| apollonian-gasket | xfig | move_step | 1.3; 1.3; 1.0 |
| apollonian-gasket | xfig | move_step_max | 1.6; 1.5; 1.5 |
| apollonian-gasket | xfig | move_place | 20.1; 18.7; 18.3 |
| apollonian-gasket | xfig | save | 12.4; 12.4; 11.9 |
| apollonian-gasket | xfig | undo | 21.2; 20.5; 22.0 |
| apollonian-gasket | xfig | x_open | 1220.5; 1183.8; 1204.7 |
| apollonian-gasket | xfig | x_zoom_in | 34.2; 40.9; 40.3 |
| apollonian-gasket | xfig | x_zoom_out | 34.0; 30.5; 31.8 |
| apollonian-gasket | xfig | x_pan | 10.5; 11.6; 10.9 |
| apollonian-gasket | xfig | x_move_step | 10.0; 9.8; 9.5 |
| apollonian-gasket | xfig | x_move_place | 60.4; 58.9; 49.9 |
| apollonian-gasket | xfig | x_save | 37.9; 31.4; 29.8 |
| apollonian-gasket | xfig | x_undo | 33.1; 38.1; 38.4 |
| apollonian-gasket | RustFig | open | 200.8; 198.0; 185.8 |
| apollonian-gasket | RustFig | open_read | -; -; - |
| apollonian-gasket | RustFig | open_draw | -; -; - |
| apollonian-gasket | RustFig | zoom_in | 2.4; 2.4; 2.4 |
| apollonian-gasket | RustFig | zoom_out | 2.5; 2.6; 2.5 |
| apollonian-gasket | RustFig | pan | 1.9; 1.8; 1.9 |
| apollonian-gasket | RustFig | move_step | 0.1; 0.1; 0.1 |
| apollonian-gasket | RustFig | move_step_max | 0.2; 0.2; 0.2 |
| apollonian-gasket | RustFig | move_place | 6.3; 6.1; 6.1 |
| apollonian-gasket | RustFig | save | 3.2; 3.3; 4.1 |
| apollonian-gasket | RustFig | undo | 6.3; 6.3; 6.6 |
| apollonian-gasket | RustFig | x_open | 200.8; 198.0; 185.8 |
| apollonian-gasket | RustFig | x_zoom_in | 3.8; 3.9; 3.9 |
| apollonian-gasket | RustFig | x_zoom_out | 3.9; 4.0; 3.9 |
| apollonian-gasket | RustFig | x_pan | 3.3; 3.3; 3.3 |
| apollonian-gasket | RustFig | x_move_step | 3.0; 3.4; 3.2 |
| apollonian-gasket | RustFig | x_move_place | 11.0; 10.9; 10.7 |
| apollonian-gasket | RustFig | x_save | -; -; - |
| apollonian-gasket | RustFig | x_undo | 9.3; 9.5; 9.6 |
| blankmap | xfig | open | 1190.4; 1206.7; 1200.8 |
| blankmap | xfig | open_read | 30.3; 32.2; 30.4 |
| blankmap | xfig | open_draw | 4.1; 5.8; 4.7 |
| blankmap | xfig | zoom_in | 11.7; 14.7; 14.9 |
| blankmap | xfig | zoom_out | 13.1; 15.0; 14.9 |
| blankmap | xfig | pan | 17.9; 18.0; 15.2 |
| blankmap | xfig | move_step | 1.3; 1.3; 1.2 |
| blankmap | xfig | move_step_max | 1.5; 1.6; 1.4 |
| blankmap | xfig | move_place | 52.5; 50.3; 48.0 |
| blankmap | xfig | save | 27.7; 29.4; 29.0 |
| blankmap | xfig | undo | 50.7; 50.2; 49.5 |
| blankmap | xfig | x_open | 1230.0; 1247.1; 1235.5 |
| blankmap | xfig | x_zoom_in | 36.8; 39.4; 36.8 |
| blankmap | xfig | x_zoom_out | 38.3; 37.7; 29.4 |
| blankmap | xfig | x_pan | 15.7; 16.1; 14.3 |
| blankmap | xfig | x_move_step | 10.2; 10.1; 9.8 |
| blankmap | xfig | x_move_place | 54.9; 51.1; 49.6 |
| blankmap | xfig | x_save | 33.3; 35.3; 34.7 |
| blankmap | xfig | x_undo | 53.5; 50.4; 50.4 |
| blankmap | RustFig | open | 197.6; 185.6; 197.9 |
| blankmap | RustFig | open_read | -; -; - |
| blankmap | RustFig | open_draw | -; -; - |
| blankmap | RustFig | zoom_in | 2.4; 2.3; 2.5 |
| blankmap | RustFig | zoom_out | 3.0; 2.8; 2.7 |
| blankmap | RustFig | pan | 1.8; 1.8; 1.8 |
| blankmap | RustFig | move_step | 0.1; 0.1; 0.1 |
| blankmap | RustFig | move_step_max | 0.1; 0.1; 0.1 |
| blankmap | RustFig | move_place | 5.4; 6.6; 5.8 |
| blankmap | RustFig | save | 3.6; 3.5; 3.6 |
| blankmap | RustFig | undo | 5.3; 5.3; 5.9 |
| blankmap | RustFig | x_open | 197.6; 185.6; 197.9 |
| blankmap | RustFig | x_zoom_in | 7.4; 7.3; 3.9 |
| blankmap | RustFig | x_zoom_out | 4.3; 4.1; 4.0 |
| blankmap | RustFig | x_pan | 3.1; 3.1; 3.1 |
| blankmap | RustFig | x_move_step | 4.2; 3.3; 6.5 |
| blankmap | RustFig | x_move_place | 13.0; 11.2; 10.5 |
| blankmap | RustFig | x_save | -; -; - |
| blankmap | RustFig | x_undo | 7.5; 8.2; 13.6 |
| ne-coastline-10m | xfig | open | 1213.3; 1256.6; 1256.0 |
| ne-coastline-10m | xfig | open_read | 78.3; 74.3; 81.5 |
| ne-coastline-10m | xfig | open_draw | 3.1; 2.8; 3.0 |
| ne-coastline-10m | xfig | zoom_in | 12.0; 12.1; 13.0 |
| ne-coastline-10m | xfig | zoom_out | 13.1; 13.3; 11.3 |
| ne-coastline-10m | xfig | pan | 12.1; 11.2; 13.7 |
| ne-coastline-10m | xfig | move_step | 1.6; 1.6; 1.6 |
| ne-coastline-10m | xfig | move_step_max | 1.7; 1.8; 1.7 |
| ne-coastline-10m | xfig | move_place | 131.4; 130.5; 126.3 |
| ne-coastline-10m | xfig | save | 59.6; 61.5; 57.9 |
| ne-coastline-10m | xfig | undo | 39.5; 39.9; 35.0 |
| ne-coastline-10m | xfig | x_open | 1251.7; 1291.3; 1296.0 |
| ne-coastline-10m | xfig | x_zoom_in | 29.5; 37.9; 39.6 |
| ne-coastline-10m | xfig | x_zoom_out | 36.4; 36.3; 40.9 |
| ne-coastline-10m | xfig | x_pan | 11.1; 10.1; 11.8 |
| ne-coastline-10m | xfig | x_move_step | 9.9; 9.6; 10.2 |
| ne-coastline-10m | xfig | x_move_place | 132.2; 132.3; 127.8 |
| ne-coastline-10m | xfig | x_save | 61.9; 64.7; 61.2 |
| ne-coastline-10m | xfig | x_undo | 43.2; 41.3; 38.6 |
| ne-coastline-10m | RustFig | open | 260.2; 242.0; 233.5 |
| ne-coastline-10m | RustFig | open_read | -; -; - |
| ne-coastline-10m | RustFig | open_draw | -; -; - |
| ne-coastline-10m | RustFig | zoom_in | 4.7; 4.7; 4.9 |
| ne-coastline-10m | RustFig | zoom_out | 5.8; 5.5; 6.1 |
| ne-coastline-10m | RustFig | pan | 3.3; 3.3; 2.8 |
| ne-coastline-10m | RustFig | move_step | 0.1; 0.1; 0.1 |
| ne-coastline-10m | RustFig | move_step_max | 0.2; 0.2; 0.2 |
| ne-coastline-10m | RustFig | move_place | 14.3; 12.8; 14.4 |
| ne-coastline-10m | RustFig | save | 9.9; 9.9; 10.1 |
| ne-coastline-10m | RustFig | undo | 16.7; 14.9; 17.2 |
| ne-coastline-10m | RustFig | x_open | 260.2; 242.0; 233.5 |
| ne-coastline-10m | RustFig | x_zoom_in | 6.2; 6.1; 6.3 |
| ne-coastline-10m | RustFig | x_zoom_out | 7.5; 6.9; 7.5 |
| ne-coastline-10m | RustFig | x_pan | 7.1; 4.6; 7.0 |
| ne-coastline-10m | RustFig | x_move_step | 4.7; 4.8; 3.0 |
| ne-coastline-10m | RustFig | x_move_place | 19.5; 17.9; 19.6 |
| ne-coastline-10m | RustFig | x_save | -; -; - |
| ne-coastline-10m | RustFig | x_undo | 19.7; 18.0; 20.3 |
| ne-countries-10m | xfig | open | 1269.5; 1250.5; 1231.9 |
| ne-countries-10m | xfig | open_read | 88.9; 88.8; 90.0 |
| ne-countries-10m | xfig | open_draw | 5.6; 5.4; 5.2 |
| ne-countries-10m | xfig | zoom_in | 20.5; 18.3; 19.7 |
| ne-countries-10m | xfig | zoom_out | 18.8; 19.0; 19.7 |
| ne-countries-10m | xfig | pan | 21.1; 22.7; 21.2 |
| ne-countries-10m | xfig | move_step | 1.0; 1.0; 1.0 |
| ne-countries-10m | xfig | move_step_max | 1.1; 1.1; 1.1 |
| ne-countries-10m | xfig | move_place | 163.3; 158.6; 160.0 |
| ne-countries-10m | xfig | save | 66.4; 73.6; 79.4 |
| ne-countries-10m | xfig | undo | 44.5; 52.0; 44.4 |
| ne-countries-10m | xfig | x_open | 1308.2; 1287.2; 1268.0 |
| ne-countries-10m | xfig | x_zoom_in | 33.0; 32.9; 32.7 |
| ne-countries-10m | xfig | x_zoom_out | 27.9; 37.5; 29.5 |
| ne-countries-10m | xfig | x_pan | 21.4; 21.9; 22.5 |
| ne-countries-10m | xfig | x_move_step | 9.9; 10.0; 9.6 |
| ne-countries-10m | xfig | x_move_place | 163.9; 160.9; 162.6 |
| ne-countries-10m | xfig | x_save | 69.2; 74.8; 80.6 |
| ne-countries-10m | xfig | x_undo | 46.3; 55.7; 45.1 |
| ne-countries-10m | RustFig | open | 258.0; 264.0; 265.7 |
| ne-countries-10m | RustFig | open_read | -; -; - |
| ne-countries-10m | RustFig | open_draw | -; -; - |
| ne-countries-10m | RustFig | zoom_in | 7.6; 7.3; 6.9 |
| ne-countries-10m | RustFig | zoom_out | 8.4; 8.7; 9.2 |
| ne-countries-10m | RustFig | pan | 5.4; 4.5; 5.1 |
| ne-countries-10m | RustFig | move_step | 0.1; 0.1; 0.1 |
| ne-countries-10m | RustFig | move_step_max | 0.4; 0.2; 0.4 |
| ne-countries-10m | RustFig | move_place | 19.1; 20.6; 21.5 |
| ne-countries-10m | RustFig | save | 14.1; 14.0; 13.4 |
| ne-countries-10m | RustFig | undo | 22.8; 22.9; 23.8 |
| ne-countries-10m | RustFig | x_open | 258.0; 264.0; 265.7 |
| ne-countries-10m | RustFig | x_zoom_in | 9.1; 12.8; 8.5 |
| ne-countries-10m | RustFig | x_zoom_out | 10.0; 10.1; 10.8 |
| ne-countries-10m | RustFig | x_pan | 6.9; 6.6; 6.5 |
| ne-countries-10m | RustFig | x_move_step | 6.0; 3.1; 2.9 |
| ne-countries-10m | RustFig | x_move_place | 24.6; 26.2; 28.9 |
| ne-countries-10m | RustFig | x_save | -; -; - |
| ne-countries-10m | RustFig | x_undo | 30.2; 27.8; 27.1 |
| greece | xfig | open | 1249.2; 1247.0; 1241.0 |
| greece | xfig | open_read | 87.5; 84.6; 84.3 |
| greece | xfig | open_draw | 4.8; 4.8; 4.9 |
| greece | xfig | zoom_in | 14.4; 12.5; 14.2 |
| greece | xfig | zoom_out | 16.0; 14.9; 15.9 |
| greece | xfig | pan | 12.1; 11.2; 11.8 |
| greece | xfig | move_step | 0.5; 0.5; 0.5 |
| greece | xfig | move_step_max | 0.8; 0.7; 0.6 |
| greece | xfig | move_place | 103.7; 100.9; 100.8 |
| greece | xfig | save | 46.4; 45.2; 46.2 |
| greece | xfig | undo | 26.7; 30.1; 30.2 |
| greece | xfig | x_open | 1284.0; 1280.1; 1276.7 |
| greece | xfig | x_zoom_in | 28.0; 37.9; 29.5 |
| greece | xfig | x_zoom_out | 38.7; 29.6; 29.7 |
| greece | xfig | x_pan | 14.4; 9.6; 15.1 |
| greece | xfig | x_move_step | 9.7; 9.9; 10.0 |
| greece | xfig | x_move_place | 105.2; 101.5; 101.5 |
| greece | xfig | x_save | 48.2; 48.1; 49.6 |
| greece | xfig | x_undo | 35.1; 34.2; 35.0 |
| greece | RustFig | open | 302.2; 288.9; 303.9 |
| greece | RustFig | open_read | -; -; - |
| greece | RustFig | open_draw | -; -; - |
| greece | RustFig | zoom_in | 14.4; 14.5; 14.6 |
| greece | RustFig | zoom_out | 14.9; 15.3; 15.1 |
| greece | RustFig | pan | 12.8; 13.7; 13.6 |
| greece | RustFig | move_step | 0.1; 0.1; 0.1 |
| greece | RustFig | move_step_max | 0.6; 0.5; 0.5 |
| greece | RustFig | move_place | 20.5; 19.0; 18.6 |
| greece | RustFig | save | 7.4; 7.4; 7.2 |
| greece | RustFig | undo | 20.0; 18.5; 19.1 |
| greece | RustFig | x_open | 302.2; 288.9; 303.9 |
| greece | RustFig | x_zoom_in | 15.9; 16.1; 16.2 |
| greece | RustFig | x_zoom_out | 16.4; 16.8; 16.7 |
| greece | RustFig | x_pan | 14.2; 15.2; 15.2 |
| greece | RustFig | x_move_step | 2.8; 4.3; 3.0 |
| greece | RustFig | x_move_place | 25.5; 30.1; 28.9 |
| greece | RustFig | x_save | -; -; - |
| greece | RustFig | x_undo | 24.4; 21.5; 22.1 |
| clifford-attractor | xfig | open | 1314.3; 1318.8; 1303.4 |
| clifford-attractor | xfig | open_read | 129.1; 129.1; 129.6 |
| clifford-attractor | xfig | open_draw | 33.4; 34.3; 33.3 |
| clifford-attractor | xfig | zoom_in | 60.5; 64.8; 61.4 |
| clifford-attractor | xfig | zoom_out | 60.9; 67.6; 59.8 |
| clifford-attractor | xfig | pan | 41.3; 44.1; 41.5 |
| clifford-attractor | xfig | move_step | 0.5; 0.5; 0.5 |
| clifford-attractor | xfig | move_step_max | 0.6; 0.6; 0.6 |
| clifford-attractor | xfig | move_place | 98.5; 103.2; 99.1 |
| clifford-attractor | xfig | save | 134.0; 133.5; 134.6 |
| clifford-attractor | xfig | undo | 34.0; 33.3; 32.7 |
| clifford-attractor | xfig | x_open | 1353.9; 1351.9; 1340.3 |
| clifford-attractor | xfig | x_zoom_in | 63.5; 64.7; 64.1 |
| clifford-attractor | xfig | x_zoom_out | 61.6; 68.4; 59.6 |
| clifford-attractor | xfig | x_pan | 42.0; 44.4; 41.3 |
| clifford-attractor | xfig | x_move_step | 9.7; 10.0; 9.7 |
| clifford-attractor | xfig | x_move_place | 101.8; 106.7; 99.2 |
| clifford-attractor | xfig | x_save | 136.0; 136.1; 135.7 |
| clifford-attractor | xfig | x_undo | 35.3; 35.2; 36.1 |
| clifford-attractor | RustFig | open | 468.5; 476.2; 496.4 |
| clifford-attractor | RustFig | open_read | -; -; - |
| clifford-attractor | RustFig | open_draw | -; -; - |
| clifford-attractor | RustFig | zoom_in | 39.9; 37.8; 45.8 |
| clifford-attractor | RustFig | zoom_out | 41.1; 51.2; 51.2 |
| clifford-attractor | RustFig | pan | 43.3; 50.4; 44.4 |
| clifford-attractor | RustFig | move_step | 0.1; 0.1; 0.2 |
| clifford-attractor | RustFig | move_step_max | 0.3; 0.3; 0.3 |
| clifford-attractor | RustFig | move_place | 68.9; 71.9; 71.4 |
| clifford-attractor | RustFig | save | 58.4; 56.3; 57.2 |
| clifford-attractor | RustFig | undo | 85.6; 82.7; 87.4 |
| clifford-attractor | RustFig | x_open | 468.5; 476.2; 496.4 |
| clifford-attractor | RustFig | x_zoom_in | 47.9; 39.5; 52.4 |
| clifford-attractor | RustFig | x_zoom_out | 49.5; 51.7; 53.2 |
| clifford-attractor | RustFig | x_pan | 44.7; 53.5; 46.8 |
| clifford-attractor | RustFig | x_move_step | 5.2; 4.2; 5.6 |
| clifford-attractor | RustFig | x_move_place | 89.9; 93.1; 89.1 |
| clifford-attractor | RustFig | x_save | -; -; - |
| clifford-attractor | RustFig | x_undo | 95.1; 87.8; 89.0 |
| cube8 | xfig | open | 1330.5; 1337.4; 1318.3 |
| cube8 | xfig | open_read | 112.4; 122.1; 121.7 |
| cube8 | xfig | open_draw | 46.3; 45.4; 44.9 |
| cube8 | xfig | zoom_in | 67.7; 66.1; 66.6 |
| cube8 | xfig | zoom_out | 79.8; 79.7; 79.7 |
| cube8 | xfig | pan | 67.6; 70.3; 65.8 |
| cube8 | xfig | move_step | 0.5; 0.5; 0.5 |
| cube8 | xfig | move_step_max | 0.7; 0.6; 0.7 |
| cube8 | xfig | move_place | 139.7; 142.6; 141.3 |
| cube8 | xfig | save | 110.8; 111.8; 118.0 |
| cube8 | xfig | undo | 38.7; 38.2; 38.7 |
| cube8 | xfig | x_open | 1366.2; 1372.8; 1353.2 |
| cube8 | xfig | x_zoom_in | 68.6; 65.9; 66.4 |
| cube8 | xfig | x_zoom_out | 80.2; 79.2; 81.5 |
| cube8 | xfig | x_pan | 68.6; 71.6; 66.9 |
| cube8 | xfig | x_move_step | 9.9; 9.7; 9.7 |
| cube8 | xfig | x_move_place | 141.3; 146.1; 142.1 |
| cube8 | xfig | x_save | 112.1; 115.4; 121.6 |
| cube8 | xfig | x_undo | 41.7; 38.9; 39.1 |
| cube8 | RustFig | open | 1377.6; 1407.1; 1393.0 |
| cube8 | RustFig | open_read | -; -; - |
| cube8 | RustFig | open_draw | -; -; - |
| cube8 | RustFig | zoom_in | 229.2; 237.5; 218.4 |
| cube8 | RustFig | zoom_out | 257.4; 276.1; 260.8 |
| cube8 | RustFig | pan | 164.7; 181.6; 192.1 |
| cube8 | RustFig | move_step | 0.2; 0.2; 0.2 |
| cube8 | RustFig | move_step_max | 3.2; 3.1; 7.6 |
| cube8 | RustFig | move_place | 345.8; 341.1; 337.3 |
| cube8 | RustFig | save | 115.6; 115.1; 118.9 |
| cube8 | RustFig | undo | 426.6; 414.5; 416.8 |
| cube8 | RustFig | x_open | 1377.6; 1407.1; 1393.0 |
| cube8 | RustFig | x_zoom_in | 232.3; 235.9; 219.8 |
| cube8 | RustFig | x_zoom_out | 253.7; 243.1; 255.9 |
| cube8 | RustFig | x_pan | 162.8; 188.2; 188.7 |
| cube8 | RustFig | x_move_step | 5.3; 9.4; 8.8 |
| cube8 | RustFig | x_move_place | 361.1; 366.2; 357.3 |
| cube8 | RustFig | x_save | -; -; - |
| cube8 | RustFig | x_undo | 423.4; 417.1; 414.5 |

</details>

Move test: 30 steps over 240 x 120 pixels. Figures with more than 300 objects (snowy, penrose-tiling, apollonian-gasket, blankmap, ne-coastline-10m, ne-countries-10m, greece, clifford-attractor, cube8) were glued into one compound first (xfig: Glue, middle-button region, right button; RustFig: select all, group), smaller ones move a single object.

<!-- results:end -->

## Results with RustFig at Retina 2x (2256x1372 pixels, for information)

<!-- results-retina:start -->

### Setup of this run

- Machine: Apple M4, macOS 27.2
- xfig: `/Users/fabian/rustfig-speed/xfig/src/src/xfig` (3.2.9a), default options, scratch `HOME`, on `Xvfb :78` 1400x1000x24; window `1322x818` (canvas 1128x686 pixels)
- RustFig: `/Users/fabian/rustfig-speed/bin/rustfig` (rustfig 0.9.0-beta.7), release build, label: main 2112440 (after P8), Retina 2x; `-geometry 1398x803`
- Each value is the median of 3 run(s), a new process per run; timeout 120 s per measurement ("> N s": not finished in that time; the real time is longer)
- Generated 2026-11-03 00:33 by `cargo xtask speed-compare`

- RustFig canvas as reported by `ui.state` (points): 1128x686; density native (the display's) (xfig's canvas: 1128x686 pixels)
### Results

xfig / RustFig is the speed-up of RustFig (above 1 = RustFig is faster).

#### Open (xfig: main() to figure drawn; RustFig: process start to drawn)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.20 s | 204 ms | 5.9x |
| feynman (small) | 4 kB, 34 obj. | 1.18 s | 195 ms | 6.0x |
| fourier (small) | 16 kB, 41 obj. | 1.17 s | 207 ms | 5.7x |
| pines | 11 kB, 31 obj. | 1.16 s | 182 ms | 6.4x |
| snowy | 173 kB, 436 obj. | 1.16 s | 207 ms | 5.6x |
| wenig | 587 kB, 85 obj. | 1.17 s | 183 ms | 6.4x |
| gosper-curve | 182 kB, 142 obj. | 1.17 s | 187 ms | 6.2x |
| penrose-tiling | 560 kB, 7615 obj. | 1.18 s | 201 ms | 5.9x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.19 s | 198 ms | 6.0x |
| blankmap | 1.0 MB, 2232 obj. | 1.19 s | 192 ms | 6.2x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.23 s | 218 ms | 5.7x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.25 s | 218 ms | 5.7x |
| greece | 3.2 MB, 2013 obj. | 1.26 s | 281 ms | 4.5x |
| clifford-attractor | 13.2 MB, 250001 obj. | 1.32 s | 404 ms | 3.3x |
| cube8 | 14.6 MB, 190260 obj. | 1.33 s | 1.18 s | 1.1x |

#### Open, part 1: reading the file (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 42.8 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 39.8 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 39.3 ms | n/a | - |
| pines | 11 kB, 31 obj. | 2.6 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 9.8 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 20.3 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 12.4 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 21.6 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 12.3 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 29.8 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 70.6 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 87.9 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 88.2 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 128 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 122 ms | n/a | - |

#### Open, part 2: first full canvas redraw (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.8 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 1.3 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 1.2 ms | n/a | - |
| pines | 11 kB, 31 obj. | 6.5 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 4.9 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 8.6 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 5.2 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 10.1 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 7.5 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 4.3 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 2.8 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 5.3 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 4.9 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 33.4 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 45.1 ms | n/a | - |

#### Zoom in (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 27.6 ms | 7.5 ms | 3.7x |
| feynman (small) | 4 kB, 34 obj. | 31.6 ms | 5.8 ms | 5.4x |
| fourier (small) | 16 kB, 41 obj. | 28.0 ms | 6.2 ms | 4.6x |
| pines | 11 kB, 31 obj. | 9.3 ms | 6.0 ms | 1.5x |
| snowy | 173 kB, 436 obj. | 8.0 ms | 2.7 ms | 2.9x |
| wenig | 587 kB, 85 obj. | 22.5 ms | 6.4 ms | 3.5x |
| gosper-curve | 182 kB, 142 obj. | 8.4 ms | 10.0 ms | 0.8x |
| penrose-tiling | 560 kB, 7615 obj. | 21.8 ms | 12.0 ms | 1.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 11.0 ms | 5.5 ms | 2.0x |
| blankmap | 1.0 MB, 2232 obj. | 13.6 ms | 6.6 ms | 2.0x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 13.1 ms | 15.8 ms | 0.8x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 18.3 ms | 20.0 ms | 0.9x |
| greece | 3.2 MB, 2013 obj. | 14.0 ms | 29.0 ms | 0.5x |
| clifford-attractor | 13.2 MB, 250001 obj. | 62.0 ms | 89.1 ms | 0.7x |
| cube8 | 14.6 MB, 190260 obj. | 65.9 ms | 453 ms | 0.1x |

#### Zoom out (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 7.4 ms | 7.7 ms | 1.0x |
| feynman (small) | 4 kB, 34 obj. | 6.7 ms | 6.3 ms | 1.1x |
| fourier (small) | 16 kB, 41 obj. | 5.8 ms | 6.6 ms | 0.9x |
| pines | 11 kB, 31 obj. | 7.7 ms | 9.2 ms | 0.8x |
| snowy | 173 kB, 436 obj. | 9.3 ms | 2.8 ms | 3.3x |
| wenig | 587 kB, 85 obj. | 17.6 ms | 6.7 ms | 2.6x |
| gosper-curve | 182 kB, 142 obj. | 8.6 ms | 8.5 ms | 1.0x |
| penrose-tiling | 560 kB, 7615 obj. | 23.2 ms | 11.9 ms | 1.9x |
| apollonian-gasket | 330 kB, 4221 obj. | 12.1 ms | 4.9 ms | 2.5x |
| blankmap | 1.0 MB, 2232 obj. | 15.8 ms | 6.4 ms | 2.5x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 12.8 ms | 9.1 ms | 1.4x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 18.7 ms | 12.6 ms | 1.5x |
| greece | 3.2 MB, 2013 obj. | 16.3 ms | 27.9 ms | 0.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 64.8 ms | 52.3 ms | 1.2x |
| cube8 | 14.6 MB, 190260 obj. | 79.9 ms | 423 ms | 0.2x |

#### Pan half a screen

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 4.7 ms | 8.2 ms | 0.6x |
| feynman (small) | 4 kB, 34 obj. | 4.3 ms | 6.7 ms | 0.6x |
| fourier (small) | 16 kB, 41 obj. | 4.4 ms | 5.9 ms | 0.7x |
| pines | 11 kB, 31 obj. | 4.4 ms | 6.6 ms | 0.7x |
| snowy | 173 kB, 436 obj. | 6.8 ms | 3.6 ms | 1.9x |
| wenig | 587 kB, 85 obj. | 18.4 ms | 5.2 ms | 3.6x |
| gosper-curve | 182 kB, 142 obj. | 4.9 ms | 9.0 ms | 0.5x |
| penrose-tiling | 560 kB, 7615 obj. | 15.7 ms | 8.9 ms | 1.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 11.8 ms | 3.9 ms | 3.0x |
| blankmap | 1.0 MB, 2232 obj. | 17.5 ms | 3.8 ms | 4.7x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 12.4 ms | 7.1 ms | 1.7x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 20.4 ms | 8.5 ms | 2.4x |
| greece | 3.2 MB, 2013 obj. | 13.7 ms | 22.7 ms | 0.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 41.7 ms | 57.1 ms | 0.7x |
| cube8 | 14.6 MB, 190260 obj. | 70.2 ms | 332 ms | 0.2x |

#### Move: one step of the drag (median of 30)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 0.8 ms | 0.1 ms | 6.7x |
| feynman (small) | 4 kB, 34 obj. | 10.0 ms | 0.1 ms | 87x |
| fourier (small) | 16 kB, 41 obj. | 0.6 ms | 0.1 ms | 5.3x |
| pines | 11 kB, 31 obj. | 0.9 ms | 0.1 ms | 6.7x |
| snowy | 173 kB, 436 obj. | 0.5 ms | 0.1 ms | 3.7x |
| wenig | 587 kB, 85 obj. | 2.2 ms | 0.1 ms | 19x |
| gosper-curve | 182 kB, 142 obj. | 0.7 ms | 0.1 ms | 5.6x |
| penrose-tiling | 560 kB, 7615 obj. | 0.6 ms | 0.1 ms | 5.0x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.3 ms | 0.1 ms | 9.5x |
| blankmap | 1.0 MB, 2232 obj. | 1.3 ms | 0.1 ms | 11x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.6 ms | 0.1 ms | 15x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.0 ms | 0.1 ms | 10x |
| greece | 3.2 MB, 2013 obj. | 0.6 ms | 0.1 ms | 5.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 0.5 ms | 0.2 ms | 2.8x |
| cube8 | 14.6 MB, 190260 obj. | 0.5 ms | 0.2 ms | 2.3x |

#### Move: place (final redraw)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 4.7 ms | 13.1 ms | 0.4x |
| feynman (small) | 4 kB, 34 obj. | 58.8 ms | 10.5 ms | 5.6x |
| fourier (small) | 16 kB, 41 obj. | 4.3 ms | 7.2 ms | 0.6x |
| pines | 11 kB, 31 obj. | 8.3 ms | 5.8 ms | 1.4x |
| snowy | 173 kB, 436 obj. | 13.5 ms | 4.8 ms | 2.8x |
| wenig | 587 kB, 85 obj. | 38.9 ms | 8.5 ms | 4.6x |
| gosper-curve | 182 kB, 142 obj. | 19.7 ms | 8.8 ms | 2.2x |
| penrose-tiling | 560 kB, 7615 obj. | 24.0 ms | 10.1 ms | 2.4x |
| apollonian-gasket | 330 kB, 4221 obj. | 18.8 ms | 9.2 ms | 2.0x |
| blankmap | 1.0 MB, 2232 obj. | 50.1 ms | 8.0 ms | 6.3x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 130 ms | 17.8 ms | 7.3x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 163 ms | 23.4 ms | 7.0x |
| greece | 3.2 MB, 2013 obj. | 100 ms | 24.6 ms | 4.1x |
| clifford-attractor | 13.2 MB, 250001 obj. | 101 ms | 74.6 ms | 1.4x |
| cube8 | 14.6 MB, 190260 obj. | 141 ms | 425 ms | 0.3x |

#### Save

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 2.6 ms | 1.5 ms | 1.7x |
| feynman (small) | 4 kB, 34 obj. | 2.5 ms | 1.6 ms | 1.5x |
| fourier (small) | 16 kB, 41 obj. | 2.2 ms | 1.7 ms | 1.3x |
| pines | 11 kB, 31 obj. | 2.5 ms | 1.6 ms | 1.5x |
| snowy | 173 kB, 436 obj. | 8.8 ms | 97.6 ms | 0.1x |
| wenig | 587 kB, 85 obj. | 19.3 ms | 11.2 ms | 1.7x |
| gosper-curve | 182 kB, 142 obj. | 8.8 ms | 4.7 ms | 1.9x |
| penrose-tiling | 560 kB, 7615 obj. | 20.5 ms | 4.3 ms | 4.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 12.6 ms | 5.5 ms | 2.3x |
| blankmap | 1.0 MB, 2232 obj. | 29.8 ms | 5.5 ms | 5.5x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 61.0 ms | 15.7 ms | 3.9x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 69.7 ms | 16.1 ms | 4.3x |
| greece | 3.2 MB, 2013 obj. | 47.7 ms | 10.2 ms | 4.7x |
| clifford-attractor | 13.2 MB, 250001 obj. | 134 ms | 59.7 ms | 2.2x |
| cube8 | 14.6 MB, 190260 obj. | 117 ms | 120 ms | 1.0x |

#### Undo the move

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 2.7 ms | 10.9 ms | 0.2x |
| feynman (small) | 4 kB, 34 obj. | 5.4 ms | 8.7 ms | 0.6x |
| fourier (small) | 16 kB, 41 obj. | 2.8 ms | 6.1 ms | 0.5x |
| pines | 11 kB, 31 obj. | 7.8 ms | 6.2 ms | 1.3x |
| snowy | 173 kB, 436 obj. | 4.3 ms | 4.5 ms | 1.0x |
| wenig | 587 kB, 85 obj. | 37.8 ms | 7.6 ms | 4.9x |
| gosper-curve | 182 kB, 142 obj. | 5.7 ms | 8.0 ms | 0.7x |
| penrose-tiling | 560 kB, 7615 obj. | 6.8 ms | 11.7 ms | 0.6x |
| apollonian-gasket | 330 kB, 4221 obj. | 20.0 ms | 9.7 ms | 2.1x |
| blankmap | 1.0 MB, 2232 obj. | 48.7 ms | 7.9 ms | 6.1x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 37.3 ms | 17.7 ms | 2.1x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 43.0 ms | 25.9 ms | 1.7x |
| greece | 3.2 MB, 2013 obj. | 30.4 ms | 25.2 ms | 1.2x |
| clifford-attractor | 13.2 MB, 250001 obj. | 33.4 ms | 77.9 ms | 0.4x |
| cube8 | 14.6 MB, 190260 obj. | 38.4 ms | 579 ms | 0.1x |

#### Cross-check: the same actions timed from outside

xfig: its own hooks against watching the canvas pixels; RustFig: its frame times against the wall time until `ui.state` reports the canvas current (never below one frame, 16.7 ms).

| Figure | Task | xfig, own hooks | xfig, pixels | RustFig, frame times | RustFig, wall to idle |
|---|---|---|---|---|---|
| carnot (small) | open | 1.20 s | 1.24 s | 204 ms | 204 ms |
| carnot (small) | zoom_in | 27.6 ms | 30.1 ms | 7.5 ms | 11.5 ms |
| carnot (small) | zoom_out | 7.4 ms | 40.7 ms | 7.7 ms | 12.9 ms |
| carnot (small) | pan | 4.7 ms | 10.6 ms | 8.2 ms | 9.2 ms |
| carnot (small) | move_step | 0.8 ms | 10.0 ms | 0.1 ms | 5.2 ms |
| carnot (small) | move_place | 4.7 ms | 57.1 ms | 13.1 ms | 22.0 ms |
| carnot (small) | save | 2.6 ms | 38.9 ms | 1.5 ms | n/a |
| carnot (small) | undo | 2.7 ms | 39.6 ms | 10.9 ms | 15.9 ms |
| feynman (small) | open | 1.18 s | 1.22 s | 195 ms | 195 ms |
| feynman (small) | zoom_in | 31.6 ms | 34.9 ms | 5.8 ms | 8.8 ms |
| feynman (small) | zoom_out | 6.7 ms | 40.2 ms | 6.3 ms | 9.4 ms |
| feynman (small) | pan | 4.3 ms | 10.1 ms | 6.7 ms | 9.2 ms |
| feynman (small) | move_step | 10.0 ms | 10.0 ms | 0.1 ms | 8.3 ms |
| feynman (small) | move_place | 58.8 ms | 58.8 ms | 10.5 ms | 26.9 ms |
| feynman (small) | save | 2.5 ms | 39.2 ms | 1.6 ms | n/a |
| feynman (small) | undo | 5.4 ms | 39.4 ms | 8.7 ms | 15.0 ms |
| fourier (small) | open | 1.17 s | 1.21 s | 207 ms | 207 ms |
| fourier (small) | zoom_in | 28.0 ms | 31.3 ms | 6.2 ms | 10.1 ms |
| fourier (small) | zoom_out | 5.8 ms | 38.5 ms | 6.6 ms | 8.5 ms |
| fourier (small) | pan | 4.4 ms | 11.2 ms | 5.9 ms | 9.4 ms |
| fourier (small) | move_step | 0.6 ms | 10.0 ms | 0.1 ms | 7.0 ms |
| fourier (small) | move_place | 4.3 ms | 55.6 ms | 7.2 ms | 21.8 ms |
| fourier (small) | save | 2.2 ms | 39.0 ms | 1.7 ms | n/a |
| fourier (small) | undo | 2.8 ms | 37.6 ms | 6.1 ms | 12.0 ms |
| pines | open | 1.16 s | 1.19 s | 182 ms | 182 ms |
| pines | zoom_in | 9.3 ms | 34.8 ms | 6.0 ms | 9.9 ms |
| pines | zoom_out | 7.7 ms | 39.4 ms | 9.2 ms | 17.6 ms |
| pines | pan | 4.4 ms | 10.7 ms | 6.6 ms | 11.5 ms |
| pines | move_step | 0.9 ms | 10.0 ms | 0.1 ms | 10.0 ms |
| pines | move_place | 8.3 ms | 60.6 ms | 5.8 ms | 23.9 ms |
| pines | save | 2.5 ms | 38.4 ms | 1.6 ms | n/a |
| pines | undo | 7.8 ms | 39.4 ms | 6.2 ms | 11.9 ms |
| snowy | open | 1.16 s | 1.19 s | 207 ms | 207 ms |
| snowy | zoom_in | 8.0 ms | 40.4 ms | 2.7 ms | 6.6 ms |
| snowy | zoom_out | 9.3 ms | 36.0 ms | 2.8 ms | 8.9 ms |
| snowy | pan | 6.8 ms | 11.5 ms | 3.6 ms | 9.3 ms |
| snowy | move_step | 0.5 ms | 9.8 ms | 0.1 ms | 10.2 ms |
| snowy | move_place | 13.5 ms | 53.6 ms | 4.8 ms | 21.3 ms |
| snowy | save | 8.8 ms | 40.3 ms | 97.6 ms | n/a |
| snowy | undo | 4.3 ms | 38.1 ms | 4.5 ms | 12.1 ms |
| wenig | open | 1.17 s | 1.21 s | 183 ms | 183 ms |
| wenig | zoom_in | 22.5 ms | 33.6 ms | 6.4 ms | 10.4 ms |
| wenig | zoom_out | 17.6 ms | 36.4 ms | 6.7 ms | 9.8 ms |
| wenig | pan | 18.4 ms | 20.0 ms | 5.2 ms | 8.8 ms |
| wenig | move_step | 2.2 ms | 10.2 ms | 0.1 ms | 9.2 ms |
| wenig | move_place | 38.9 ms | 52.7 ms | 8.5 ms | 61.8 ms |
| wenig | save | 19.3 ms | 37.9 ms | 11.2 ms | n/a |
| wenig | undo | 37.8 ms | 39.5 ms | 7.6 ms | 29.3 ms |
| gosper-curve | open | 1.17 s | 1.21 s | 187 ms | 187 ms |
| gosper-curve | zoom_in | 8.4 ms | 37.4 ms | 10.0 ms | 15.4 ms |
| gosper-curve | zoom_out | 8.6 ms | 39.3 ms | 8.5 ms | 12.4 ms |
| gosper-curve | pan | 4.9 ms | 10.5 ms | 9.0 ms | 14.8 ms |
| gosper-curve | move_step | 0.7 ms | 10.2 ms | 0.1 ms | 8.6 ms |
| gosper-curve | move_place | 19.7 ms | 56.2 ms | 8.8 ms | 36.5 ms |
| gosper-curve | save | 8.8 ms | 40.6 ms | 4.7 ms | n/a |
| gosper-curve | undo | 5.7 ms | 37.8 ms | 8.0 ms | 25.4 ms |
| penrose-tiling | open | 1.18 s | 1.22 s | 201 ms | 201 ms |
| penrose-tiling | zoom_in | 21.8 ms | 28.6 ms | 12.0 ms | 15.7 ms |
| penrose-tiling | zoom_out | 23.2 ms | 31.0 ms | 11.9 ms | 16.3 ms |
| penrose-tiling | pan | 15.7 ms | 15.9 ms | 8.9 ms | 12.4 ms |
| penrose-tiling | move_step | 0.6 ms | 10.1 ms | 0.1 ms | 8.7 ms |
| penrose-tiling | move_place | 24.0 ms | 50.9 ms | 10.1 ms | 18.9 ms |
| penrose-tiling | save | 20.5 ms | 37.3 ms | 4.3 ms | n/a |
| penrose-tiling | undo | 6.8 ms | 41.3 ms | 11.7 ms | 16.9 ms |
| apollonian-gasket | open | 1.19 s | 1.23 s | 198 ms | 198 ms |
| apollonian-gasket | zoom_in | 11.0 ms | 34.3 ms | 5.5 ms | 11.9 ms |
| apollonian-gasket | zoom_out | 12.1 ms | 34.7 ms | 4.9 ms | 10.6 ms |
| apollonian-gasket | pan | 11.8 ms | 10.6 ms | 3.9 ms | 7.6 ms |
| apollonian-gasket | move_step | 1.3 ms | 10.4 ms | 0.1 ms | 8.9 ms |
| apollonian-gasket | move_place | 18.8 ms | 58.1 ms | 9.2 ms | 18.7 ms |
| apollonian-gasket | save | 12.6 ms | 32.5 ms | 5.5 ms | n/a |
| apollonian-gasket | undo | 20.0 ms | 33.1 ms | 9.7 ms | 14.7 ms |
| blankmap | open | 1.19 s | 1.23 s | 192 ms | 192 ms |
| blankmap | zoom_in | 13.6 ms | 38.5 ms | 6.6 ms | 10.6 ms |
| blankmap | zoom_out | 15.8 ms | 28.1 ms | 6.4 ms | 10.3 ms |
| blankmap | pan | 17.5 ms | 16.9 ms | 3.8 ms | 10.0 ms |
| blankmap | move_step | 1.3 ms | 10.0 ms | 0.1 ms | 9.9 ms |
| blankmap | move_place | 50.1 ms | 53.4 ms | 8.0 ms | 20.4 ms |
| blankmap | save | 29.8 ms | 37.0 ms | 5.5 ms | n/a |
| blankmap | undo | 48.7 ms | 51.0 ms | 7.9 ms | 14.5 ms |
| ne-coastline-10m | open | 1.23 s | 1.27 s | 218 ms | 218 ms |
| ne-coastline-10m | zoom_in | 13.1 ms | 37.7 ms | 15.8 ms | 21.4 ms |
| ne-coastline-10m | zoom_out | 12.8 ms | 34.4 ms | 9.1 ms | 13.5 ms |
| ne-coastline-10m | pan | 12.4 ms | 11.2 ms | 7.1 ms | 11.2 ms |
| ne-coastline-10m | move_step | 1.6 ms | 9.8 ms | 0.1 ms | 5.4 ms |
| ne-coastline-10m | move_place | 130 ms | 132 ms | 17.8 ms | 33.4 ms |
| ne-coastline-10m | save | 61.0 ms | 62.7 ms | 15.7 ms | n/a |
| ne-coastline-10m | undo | 37.3 ms | 40.3 ms | 17.7 ms | 25.1 ms |
| ne-countries-10m | open | 1.25 s | 1.28 s | 218 ms | 218 ms |
| ne-countries-10m | zoom_in | 18.3 ms | 31.7 ms | 20.0 ms | 23.9 ms |
| ne-countries-10m | zoom_out | 18.7 ms | 34.6 ms | 12.6 ms | 20.3 ms |
| ne-countries-10m | pan | 20.4 ms | 21.6 ms | 8.5 ms | 12.7 ms |
| ne-countries-10m | move_step | 1.0 ms | 10.1 ms | 0.1 ms | 5.0 ms |
| ne-countries-10m | move_place | 163 ms | 166 ms | 23.4 ms | 33.3 ms |
| ne-countries-10m | save | 69.7 ms | 72.2 ms | 16.1 ms | n/a |
| ne-countries-10m | undo | 43.0 ms | 46.4 ms | 25.9 ms | 39.1 ms |
| greece | open | 1.26 s | 1.30 s | 281 ms | 281 ms |
| greece | zoom_in | 14.0 ms | 37.0 ms | 29.0 ms | 33.2 ms |
| greece | zoom_out | 16.3 ms | 38.3 ms | 27.9 ms | 33.6 ms |
| greece | pan | 13.7 ms | 13.9 ms | 22.7 ms | 30.9 ms |
| greece | move_step | 0.6 ms | 10.1 ms | 0.1 ms | 6.7 ms |
| greece | move_place | 100 ms | 101 ms | 24.6 ms | 35.2 ms |
| greece | save | 47.7 ms | 49.6 ms | 10.2 ms | n/a |
| greece | undo | 30.4 ms | 33.2 ms | 25.2 ms | 32.8 ms |
| clifford-attractor | open | 1.32 s | 1.35 s | 404 ms | 404 ms |
| clifford-attractor | zoom_in | 62.0 ms | 62.6 ms | 89.1 ms | 94.1 ms |
| clifford-attractor | zoom_out | 64.8 ms | 64.4 ms | 52.3 ms | 57.3 ms |
| clifford-attractor | pan | 41.7 ms | 41.2 ms | 57.1 ms | 62.0 ms |
| clifford-attractor | move_step | 0.5 ms | 10.1 ms | 0.2 ms | 9.2 ms |
| clifford-attractor | move_place | 101 ms | 104 ms | 74.6 ms | 98.0 ms |
| clifford-attractor | save | 134 ms | 138 ms | 59.7 ms | n/a |
| clifford-attractor | undo | 33.4 ms | 35.0 ms | 77.9 ms | 88.2 ms |
| cube8 | open | 1.33 s | 1.36 s | 1.18 s | 1.18 s |
| cube8 | zoom_in | 65.9 ms | 65.8 ms | 453 ms | 435 ms |
| cube8 | zoom_out | 79.9 ms | 81.7 ms | 423 ms | 412 ms |
| cube8 | pan | 70.2 ms | 72.2 ms | 332 ms | 316 ms |
| cube8 | move_step | 0.5 ms | 9.9 ms | 0.2 ms | 11.9 ms |
| cube8 | move_place | 141 ms | 143 ms | 425 ms | 458 ms |
| cube8 | save | 117 ms | 120 ms | 120 ms | n/a |
| cube8 | undo | 38.4 ms | 39.6 ms | 579 ms | 568 ms |

<details><summary>Raw numbers (every run, milliseconds)</summary>

| Figure | App | Task | Runs |
|---|---|---|---|
| carnot (small) | xfig | open | 1219.6; 1203.9; 1173.4 |
| carnot (small) | xfig | open_read | 43.0; 42.8; 41.0 |
| carnot (small) | xfig | open_draw | 1.9; 1.2; 1.8 |
| carnot (small) | xfig | zoom_in | 28.3; 27.6; 27.2 |
| carnot (small) | xfig | zoom_out | 7.7; 7.0; 7.4 |
| carnot (small) | xfig | pan | 4.7; 4.7; 4.5 |
| carnot (small) | xfig | move_step | 0.8; 0.8; 0.8 |
| carnot (small) | xfig | move_step_max | 1.0; 1.0; 1.1 |
| carnot (small) | xfig | move_place | 4.9; 4.2; 4.7 |
| carnot (small) | xfig | save | 2.6; 2.6; 1.7 |
| carnot (small) | xfig | undo | 3.8; 2.2; 2.7 |
| carnot (small) | xfig | x_open | 1255.1; 1241.4; 1206.5 |
| carnot (small) | xfig | x_zoom_in | 35.2; 29.2; 30.1 |
| carnot (small) | xfig | x_zoom_out | 41.1; 40.7; 40.2 |
| carnot (small) | xfig | x_pan | 10.6; 10.7; 10.1 |
| carnot (small) | xfig | x_move_step | 10.4; 10.0; 9.9 |
| carnot (small) | xfig | x_move_place | 58.7; 51.7; 57.1 |
| carnot (small) | xfig | x_save | 40.9; 38.9; 36.7 |
| carnot (small) | xfig | x_undo | 39.8; 39.6; 29.5 |
| carnot (small) | RustFig | open | 229.7; 203.9; 189.4 |
| carnot (small) | RustFig | open_read | -; -; - |
| carnot (small) | RustFig | open_draw | -; -; - |
| carnot (small) | RustFig | zoom_in | 7.4; 7.5; 7.5 |
| carnot (small) | RustFig | zoom_out | 7.8; 7.5; 7.7 |
| carnot (small) | RustFig | pan | 7.3; 8.3; 8.2 |
| carnot (small) | RustFig | move_step | 0.1; 0.1; 0.1 |
| carnot (small) | RustFig | move_step_max | 0.3; 0.2; 0.3 |
| carnot (small) | RustFig | move_place | 13.1; 13.1; 13.9 |
| carnot (small) | RustFig | save | 1.5; 1.4; 1.6 |
| carnot (small) | RustFig | undo | 11.0; 10.7; 10.9 |
| carnot (small) | RustFig | x_open | 229.7; 203.9; 189.4 |
| carnot (small) | RustFig | x_zoom_in | 13.0; 11.5; 10.4 |
| carnot (small) | RustFig | x_zoom_out | 14.2; 10.3; 12.9 |
| carnot (small) | RustFig | x_pan | 12.9; 8.7; 9.2 |
| carnot (small) | RustFig | x_move_step | 4.8; 6.1; 5.2 |
| carnot (small) | RustFig | x_move_place | 22.0; 26.4; 20.8 |
| carnot (small) | RustFig | x_save | -; -; - |
| carnot (small) | RustFig | x_undo | 15.9; 18.2; 15.4 |
| feynman (small) | xfig | open | 1207.0; 1182.4; 1175.3 |
| feynman (small) | xfig | open_read | 38.5; 41.4; 39.8 |
| feynman (small) | xfig | open_draw | 1.5; 1.3; 0.9 |
| feynman (small) | xfig | zoom_in | 30.2; 31.6; 32.5 |
| feynman (small) | xfig | zoom_out | 6.9; 6.7; 4.6 |
| feynman (small) | xfig | pan | 4.3; 7.3; 3.8 |
| feynman (small) | xfig | move_step | 9.9; 10.2; 10.0 |
| feynman (small) | xfig | move_step_max | 11.4; 11.3; 11.1 |
| feynman (small) | xfig | move_place | 50.9; 58.8; 61.3 |
| feynman (small) | xfig | save | 1.9; 2.6; 2.5 |
| feynman (small) | xfig | undo | 5.6; 5.4; 4.4 |
| feynman (small) | xfig | x_open | 1245.9; 1219.3; 1208.8 |
| feynman (small) | xfig | x_zoom_in | 33.7; 34.9; 35.4 |
| feynman (small) | xfig | x_zoom_out | 40.2; 32.9; 41.6 |
| feynman (small) | xfig | x_pan | 11.2; 9.8; 10.1 |
| feynman (small) | xfig | x_move_step | 9.9; 10.2; 10.0 |
| feynman (small) | xfig | x_move_place | 50.9; 58.8; 61.3 |
| feynman (small) | xfig | x_save | 33.9; 39.2; 40.8 |
| feynman (small) | xfig | x_undo | 31.6; 40.4; 39.4 |
| feynman (small) | RustFig | open | 217.9; 187.5; 195.5 |
| feynman (small) | RustFig | open_read | -; -; - |
| feynman (small) | RustFig | open_draw | -; -; - |
| feynman (small) | RustFig | zoom_in | 5.8; 5.8; 5.9 |
| feynman (small) | RustFig | zoom_out | 6.8; 6.3; 6.3 |
| feynman (small) | RustFig | pan | 6.4; 6.7; 6.7 |
| feynman (small) | RustFig | move_step | 0.1; 0.1; 0.1 |
| feynman (small) | RustFig | move_step_max | 0.4; 0.5; 0.2 |
| feynman (small) | RustFig | move_place | 9.3; 10.6; 10.5 |
| feynman (small) | RustFig | save | 1.6; 3.3; 1.6 |
| feynman (small) | RustFig | undo | 8.7; 7.9; 9.2 |
| feynman (small) | RustFig | x_open | 217.9; 187.5; 195.5 |
| feynman (small) | RustFig | x_zoom_in | 11.1; 8.8; 8.8 |
| feynman (small) | RustFig | x_zoom_out | 9.8; 9.3; 9.4 |
| feynman (small) | RustFig | x_pan | 8.4; 9.2; 12.2 |
| feynman (small) | RustFig | x_move_step | 8.3; 9.0; 5.3 |
| feynman (small) | RustFig | x_move_place | 20.6; 34.6; 26.9 |
| feynman (small) | RustFig | x_save | -; -; - |
| feynman (small) | RustFig | x_undo | 15.0; 13.2; 17.0 |
| fourier (small) | xfig | open | 1203.6; 1172.9; 1149.8 |
| fourier (small) | xfig | open_read | 44.9; 39.3; 34.9 |
| fourier (small) | xfig | open_draw | 1.2; 1.4; 0.9 |
| fourier (small) | xfig | zoom_in | 27.2; 28.0; 28.7 |
| fourier (small) | xfig | zoom_out | 5.8; 6.2; 5.4 |
| fourier (small) | xfig | pan | 4.8; 4.4; 4.3 |
| fourier (small) | xfig | move_step | 0.6; 0.8; 0.3 |
| fourier (small) | xfig | move_step_max | 0.9; 1.0; 0.6 |
| fourier (small) | xfig | move_place | 4.4; 4.3; 2.6 |
| fourier (small) | xfig | save | 3.0; 2.2; 1.6 |
| fourier (small) | xfig | undo | 2.8; 2.9; 2.7 |
| fourier (small) | xfig | x_open | 1241.9; 1209.7; 1185.4 |
| fourier (small) | xfig | x_zoom_in | 29.9; 31.3; 33.9 |
| fourier (small) | xfig | x_zoom_out | 40.4; 32.2; 38.5 |
| fourier (small) | xfig | x_pan | 11.2; 11.1; 11.4 |
| fourier (small) | xfig | x_move_step | 10.0; 10.2; 6.4 |
| fourier (small) | xfig | x_move_place | 55.6; 59.9; 51.6 |
| fourier (small) | xfig | x_save | 40.5; 39.0; 36.2 |
| fourier (small) | xfig | x_undo | 37.6; 40.3; 33.8 |
| fourier (small) | RustFig | open | 242.2; 195.8; 207.0 |
| fourier (small) | RustFig | open_read | -; -; - |
| fourier (small) | RustFig | open_draw | -; -; - |
| fourier (small) | RustFig | zoom_in | 5.9; 6.2; 6.3 |
| fourier (small) | RustFig | zoom_out | 6.3; 6.6; 6.8 |
| fourier (small) | RustFig | pan | 6.1; 5.9; 5.6 |
| fourier (small) | RustFig | move_step | 0.1; 0.1; 0.1 |
| fourier (small) | RustFig | move_step_max | 0.5; 0.6; 8.6 |
| fourier (small) | RustFig | move_place | 8.1; 7.2; 6.6 |
| fourier (small) | RustFig | save | 1.7; 1.7; 1.7 |
| fourier (small) | RustFig | undo | 6.1; 6.4; 5.7 |
| fourier (small) | RustFig | x_open | 242.2; 195.8; 207.0 |
| fourier (small) | RustFig | x_zoom_in | 9.4; 12.2; 10.1 |
| fourier (small) | RustFig | x_zoom_out | 7.9; 8.5; 9.0 |
| fourier (small) | RustFig | x_pan | 13.6; 8.1; 9.4 |
| fourier (small) | RustFig | x_move_step | 8.3; 7.0; 5.1 |
| fourier (small) | RustFig | x_move_place | 24.8; 17.1; 21.8 |
| fourier (small) | RustFig | x_save | -; -; - |
| fourier (small) | RustFig | x_undo | 16.4; 12.0; 11.7 |
| pines | xfig | open | 1155.6; 1148.1; 1184.1 |
| pines | xfig | open_read | 1.8; 2.8; 2.6 |
| pines | xfig | open_draw | 3.2; 6.5; 8.1 |
| pines | xfig | zoom_in | 6.7; 9.5; 9.3 |
| pines | xfig | zoom_out | 7.3; 9.6; 7.7 |
| pines | xfig | pan | 4.5; 4.0; 4.4 |
| pines | xfig | move_step | 0.9; 1.0; 0.9 |
| pines | xfig | move_step_max | 1.1; 1.2; 1.1 |
| pines | xfig | move_place | 8.5; 8.3; 7.2 |
| pines | xfig | save | 2.5; 2.5; 2.6 |
| pines | xfig | undo | 7.5; 9.0; 7.8 |
| pines | xfig | x_open | 1189.3; 1184.5; 1221.0 |
| pines | xfig | x_zoom_in | 34.8; 41.4; 31.3 |
| pines | xfig | x_zoom_out | 39.4; 30.9; 41.1 |
| pines | xfig | x_pan | 11.0; 9.5; 10.7 |
| pines | xfig | x_move_step | 10.0; 10.0; 10.0 |
| pines | xfig | x_move_place | 61.3; 60.6; 59.4 |
| pines | xfig | x_save | 38.4; 29.7; 41.3 |
| pines | xfig | x_undo | 32.2; 40.3; 39.4 |
| pines | RustFig | open | 200.7; 181.3; 182.0 |
| pines | RustFig | open_read | -; -; - |
| pines | RustFig | open_draw | -; -; - |
| pines | RustFig | zoom_in | 6.6; 6.0; 5.1 |
| pines | RustFig | zoom_out | 9.3; 5.9; 9.2 |
| pines | RustFig | pan | 6.6; 8.0; 5.8 |
| pines | RustFig | move_step | 0.1; 0.2; 0.1 |
| pines | RustFig | move_step_max | 0.5; 0.5; 0.5 |
| pines | RustFig | move_place | 5.8; 9.1; 5.7 |
| pines | RustFig | save | 1.6; 1.6; 2.0 |
| pines | RustFig | undo | 6.2; 6.2; 7.4 |
| pines | RustFig | x_open | 200.7; 181.3; 182.0 |
| pines | RustFig | x_zoom_in | 12.7; 9.9; 8.2 |
| pines | RustFig | x_zoom_out | 18.4; 9.0; 17.6 |
| pines | RustFig | x_pan | 10.4; 15.4; 11.5 |
| pines | RustFig | x_move_step | 9.1; 10.0; 11.6 |
| pines | RustFig | x_move_place | 20.4; 27.5; 23.9 |
| pines | RustFig | x_save | -; -; - |
| pines | RustFig | x_undo | 11.6; 11.9; 14.0 |
| snowy | xfig | open | 1156.9; 1168.4; 1141.8 |
| snowy | xfig | open_read | 9.8; 9.8; 6.1 |
| snowy | xfig | open_draw | 4.7; 4.9; 5.7 |
| snowy | xfig | zoom_in | 8.2; 8.0; 7.9 |
| snowy | xfig | zoom_out | 9.5; 8.1; 9.3 |
| snowy | xfig | pan | 6.8; 7.2; 6.8 |
| snowy | xfig | move_step | 0.5; 0.5; 0.5 |
| snowy | xfig | move_step_max | 0.6; 0.6; 0.6 |
| snowy | xfig | move_place | 16.6; 9.4; 13.5 |
| snowy | xfig | save | 8.8; 7.8; 9.2 |
| snowy | xfig | undo | 4.3; 5.0; 3.9 |
| snowy | xfig | x_open | 1190.7; 1204.9; 1175.0 |
| snowy | xfig | x_zoom_in | 34.5; 40.4; 41.0 |
| snowy | xfig | x_zoom_out | 33.4; 36.0; 37.2 |
| snowy | xfig | x_pan | 12.0; 11.5; 10.8 |
| snowy | xfig | x_move_step | 10.0; 9.8; 9.5 |
| snowy | xfig | x_move_place | 53.6; 47.0; 54.8 |
| snowy | xfig | x_save | 40.3; 41.1; 38.8 |
| snowy | xfig | x_undo | 38.1; 40.2; 34.8 |
| snowy | RustFig | open | 207.0; 206.5; 189.9 |
| snowy | RustFig | open_read | -; -; - |
| snowy | RustFig | open_draw | -; -; - |
| snowy | RustFig | zoom_in | 3.0; 2.6; 2.7 |
| snowy | RustFig | zoom_out | 4.0; 2.7; 2.8 |
| snowy | RustFig | pan | 2.7; 4.1; 3.6 |
| snowy | RustFig | move_step | 0.1; 0.1; 0.1 |
| snowy | RustFig | move_step_max | 0.5; 0.6; 0.6 |
| snowy | RustFig | move_place | 7.0; 4.7; 4.8 |
| snowy | RustFig | save | 171.6; 97.6; 1.8 |
| snowy | RustFig | undo | 4.9; 4.5; 4.4 |
| snowy | RustFig | x_open | 207.0; 206.5; 189.9 |
| snowy | RustFig | x_zoom_in | 6.3; 10.2; 6.6 |
| snowy | RustFig | x_zoom_out | 8.9; 11.5; 6.4 |
| snowy | RustFig | x_pan | 6.4; 9.3; 13.8 |
| snowy | RustFig | x_move_step | 10.2; 8.6; 10.5 |
| snowy | RustFig | x_move_place | 21.3; 12.5; 24.9 |
| snowy | RustFig | x_save | -; -; - |
| snowy | RustFig | x_undo | 11.6; 14.1; 12.1 |
| wenig | xfig | open | 1174.9; 1183.3; 1163.4 |
| wenig | xfig | open_read | 20.1; 23.2; 20.3 |
| wenig | xfig | open_draw | 8.6; 8.4; 9.2 |
| wenig | xfig | zoom_in | 20.0; 22.7; 22.5 |
| wenig | xfig | zoom_out | 19.9; 17.6; 14.0 |
| wenig | xfig | pan | 19.1; 17.3; 18.4 |
| wenig | xfig | move_step | 2.2; 2.2; 2.2 |
| wenig | xfig | move_step_max | 2.5; 2.5; 2.4 |
| wenig | xfig | move_place | 38.9; 43.0; 38.5 |
| wenig | xfig | save | 19.3; 18.8; 22.0 |
| wenig | xfig | undo | 35.8; 38.6; 37.8 |
| wenig | xfig | x_open | 1211.2; 1217.8; 1198.3 |
| wenig | xfig | x_zoom_in | 37.5; 30.8; 33.6 |
| wenig | xfig | x_zoom_out | 35.9; 38.9; 36.4 |
| wenig | xfig | x_pan | 20.0; 18.9; 21.0 |
| wenig | xfig | x_move_step | 10.5; 10.2; 10.2 |
| wenig | xfig | x_move_place | 53.6; 52.7; 48.8 |
| wenig | xfig | x_save | 37.4; 40.6; 37.9 |
| wenig | xfig | x_undo | 38.5; 41.2; 39.5 |
| wenig | RustFig | open | 208.9; 181.6; 182.5 |
| wenig | RustFig | open_read | -; -; - |
| wenig | RustFig | open_draw | -; -; - |
| wenig | RustFig | zoom_in | 6.4; 6.8; 5.8 |
| wenig | RustFig | zoom_out | 6.7; 7.7; 5.9 |
| wenig | RustFig | pan | 5.2; 4.9; 5.2 |
| wenig | RustFig | move_step | 0.1; 0.1; 0.1 |
| wenig | RustFig | move_step_max | 0.5; 0.5; 0.5 |
| wenig | RustFig | move_place | 9.2; 7.9; 8.5 |
| wenig | RustFig | save | 15.8; 9.9; 11.2 |
| wenig | RustFig | undo | 7.8; 6.8; 7.6 |
| wenig | RustFig | x_open | 208.9; 181.6; 182.5 |
| wenig | RustFig | x_zoom_in | 9.8; 10.4; 11.7 |
| wenig | RustFig | x_zoom_out | 9.7; 12.2; 9.8 |
| wenig | RustFig | x_pan | 8.9; 8.8; 8.3 |
| wenig | RustFig | x_move_step | 9.3; 9.2; 9.2 |
| wenig | RustFig | x_move_place | 63.4; 61.8; 61.0 |
| wenig | RustFig | x_save | -; -; - |
| wenig | RustFig | x_undo | 29.3; 28.3; 37.9 |
| gosper-curve | xfig | open | 1179.5; 1165.0; 1168.4 |
| gosper-curve | xfig | open_read | 10.9; 12.4; 12.5 |
| gosper-curve | xfig | open_draw | 5.3; 5.2; 5.0 |
| gosper-curve | xfig | zoom_in | 7.0; 8.4; 9.4 |
| gosper-curve | xfig | zoom_out | 7.4; 8.6; 8.7 |
| gosper-curve | xfig | pan | 4.9; 3.8; 5.2 |
| gosper-curve | xfig | move_step | 0.7; 0.7; 0.7 |
| gosper-curve | xfig | move_step_max | 0.8; 0.8; 0.8 |
| gosper-curve | xfig | move_place | 20.2; 19.7; 16.8 |
| gosper-curve | xfig | save | 8.3; 8.8; 8.9 |
| gosper-curve | xfig | undo | 5.7; 5.4; 6.0 |
| gosper-curve | xfig | x_open | 1216.1; 1203.3; 1207.5 |
| gosper-curve | xfig | x_zoom_in | 37.4; 34.9; 40.2 |
| gosper-curve | xfig | x_zoom_out | 39.4; 34.8; 39.3 |
| gosper-curve | xfig | x_pan | 10.5; 9.3; 11.1 |
| gosper-curve | xfig | x_move_step | 10.0; 10.3; 10.2 |
| gosper-curve | xfig | x_move_place | 56.2; 58.7; 53.9 |
| gosper-curve | xfig | x_save | 40.4; 41.6; 40.6 |
| gosper-curve | xfig | x_undo | 41.7; 37.8; 37.0 |
| gosper-curve | RustFig | open | 204.4; 187.1; 186.4 |
| gosper-curve | RustFig | open_read | -; -; - |
| gosper-curve | RustFig | open_draw | -; -; - |
| gosper-curve | RustFig | zoom_in | 11.1; 9.4; 10.0 |
| gosper-curve | RustFig | zoom_out | 8.5; 8.0; 9.3 |
| gosper-curve | RustFig | pan | 7.7; 9.0; 10.8 |
| gosper-curve | RustFig | move_step | 0.1; 0.1; 0.1 |
| gosper-curve | RustFig | move_step_max | 0.5; 0.5; 0.6 |
| gosper-curve | RustFig | move_place | 7.9; 8.8; 8.9 |
| gosper-curve | RustFig | save | 4.7; 4.8; 4.6 |
| gosper-curve | RustFig | undo | 7.9; 8.0; 8.1 |
| gosper-curve | RustFig | x_open | 204.4; 187.1; 186.4 |
| gosper-curve | RustFig | x_zoom_in | 18.4; 15.4; 13.6 |
| gosper-curve | RustFig | x_zoom_out | 11.3; 12.4; 14.1 |
| gosper-curve | RustFig | x_pan | 11.4; 14.8; 15.1 |
| gosper-curve | RustFig | x_move_step | 10.3; 8.4; 8.6 |
| gosper-curve | RustFig | x_move_place | 36.5; 44.7; 36.4 |
| gosper-curve | RustFig | x_save | -; -; - |
| gosper-curve | RustFig | x_undo | 18.5; 25.7; 25.4 |
| penrose-tiling | xfig | open | 1182.6; 1175.4; 1187.4 |
| penrose-tiling | xfig | open_read | 21.6; 22.1; 21.0 |
| penrose-tiling | xfig | open_draw | 8.1; 10.1; 13.3 |
| penrose-tiling | xfig | zoom_in | 21.8; 21.8; 19.5 |
| penrose-tiling | xfig | zoom_out | 23.2; 25.1; 23.0 |
| penrose-tiling | xfig | pan | 16.4; 15.7; 15.6 |
| penrose-tiling | xfig | move_step | 0.5; 0.6; 0.6 |
| penrose-tiling | xfig | move_step_max | 0.7; 0.8; 0.7 |
| penrose-tiling | xfig | move_place | 24.6; 23.7; 24.0 |
| penrose-tiling | xfig | save | 20.5; 19.1; 22.5 |
| penrose-tiling | xfig | undo | 7.3; 6.8; 6.7 |
| penrose-tiling | xfig | x_open | 1217.7; 1214.4; 1219.5 |
| penrose-tiling | xfig | x_zoom_in | 28.6; 33.8; 28.2 |
| penrose-tiling | xfig | x_zoom_out | 31.0; 30.1; 36.7 |
| penrose-tiling | xfig | x_pan | 16.2; 15.8; 15.9 |
| penrose-tiling | xfig | x_move_step | 10.1; 10.2; 9.9 |
| penrose-tiling | xfig | x_move_place | 50.9; 58.7; 50.4 |
| penrose-tiling | xfig | x_save | 37.3; 36.7; 39.7 |
| penrose-tiling | xfig | x_undo | 41.3; 41.7; 34.9 |
| penrose-tiling | RustFig | open | 212.4; 191.7; 201.4 |
| penrose-tiling | RustFig | open_read | -; -; - |
| penrose-tiling | RustFig | open_draw | -; -; - |
| penrose-tiling | RustFig | zoom_in | 12.2; 12.0; 11.9 |
| penrose-tiling | RustFig | zoom_out | 12.6; 10.8; 11.9 |
| penrose-tiling | RustFig | pan | 9.1; 8.8; 8.9 |
| penrose-tiling | RustFig | move_step | 0.1; 0.1; 0.1 |
| penrose-tiling | RustFig | move_step_max | 0.3; 0.5; 0.3 |
| penrose-tiling | RustFig | move_place | 11.4; 9.7; 10.1 |
| penrose-tiling | RustFig | save | 8.3; 4.3; 3.4 |
| penrose-tiling | RustFig | undo | 11.7; 10.6; 11.8 |
| penrose-tiling | RustFig | x_open | 212.4; 191.7; 201.4 |
| penrose-tiling | RustFig | x_zoom_in | 15.8; 15.6; 15.7 |
| penrose-tiling | RustFig | x_zoom_out | 16.3; 15.1; 16.9 |
| penrose-tiling | RustFig | x_pan | 13.1; 11.7; 12.4 |
| penrose-tiling | RustFig | x_move_step | 8.7; 9.4; 8.2 |
| penrose-tiling | RustFig | x_move_place | 19.6; 18.3; 18.9 |
| penrose-tiling | RustFig | x_save | -; -; - |
| penrose-tiling | RustFig | x_undo | 16.9; 16.5; 22.8 |
| apollonian-gasket | xfig | open | 1191.6; 1201.3; 1175.2 |
| apollonian-gasket | xfig | open_read | 11.6; 13.1; 12.3 |
| apollonian-gasket | xfig | open_draw | 6.3; 8.0; 7.5 |
| apollonian-gasket | xfig | zoom_in | 11.0; 8.8; 11.6 |
| apollonian-gasket | xfig | zoom_out | 11.9; 12.2; 12.1 |
| apollonian-gasket | xfig | pan | 11.3; 11.8; 12.5 |
| apollonian-gasket | xfig | move_step | 1.2; 1.3; 1.3 |
| apollonian-gasket | xfig | move_step_max | 1.5; 1.7; 1.6 |
| apollonian-gasket | xfig | move_place | 20.5; 18.8; 18.5 |
| apollonian-gasket | xfig | save | 13.0; 11.5; 12.6 |
| apollonian-gasket | xfig | undo | 20.0; 22.2; 19.4 |
| apollonian-gasket | xfig | x_open | 1226.5; 1240.8; 1213.5 |
| apollonian-gasket | xfig | x_zoom_in | 39.3; 34.3; 32.2 |
| apollonian-gasket | xfig | x_zoom_out | 34.3; 34.7; 38.7 |
| apollonian-gasket | xfig | x_pan | 10.2; 10.6; 10.9 |
| apollonian-gasket | xfig | x_move_step | 10.4; 10.3; 10.4 |
| apollonian-gasket | xfig | x_move_place | 61.3; 56.4; 58.1 |
| apollonian-gasket | xfig | x_save | 34.2; 30.4; 32.5 |
| apollonian-gasket | xfig | x_undo | 33.1; 37.3; 32.4 |
| apollonian-gasket | RustFig | open | 209.3; 197.5; 191.7 |
| apollonian-gasket | RustFig | open_read | -; -; - |
| apollonian-gasket | RustFig | open_draw | -; -; - |
| apollonian-gasket | RustFig | zoom_in | 5.5; 5.2; 5.8 |
| apollonian-gasket | RustFig | zoom_out | 4.9; 4.9; 4.9 |
| apollonian-gasket | RustFig | pan | 3.8; 3.9; 8.4 |
| apollonian-gasket | RustFig | move_step | 0.1; 0.2; 0.1 |
| apollonian-gasket | RustFig | move_step_max | 0.6; 0.6; 0.6 |
| apollonian-gasket | RustFig | move_place | 8.7; 9.2; 14.0 |
| apollonian-gasket | RustFig | save | 3.8; 5.5; 5.6 |
| apollonian-gasket | RustFig | undo | 9.7; 9.2; 9.7 |
| apollonian-gasket | RustFig | x_open | 209.3; 197.5; 191.7 |
| apollonian-gasket | RustFig | x_zoom_in | 9.6; 11.9; 12.8 |
| apollonian-gasket | RustFig | x_zoom_out | 8.1; 10.6; 14.0 |
| apollonian-gasket | RustFig | x_pan | 7.6; 6.9; 16.9 |
| apollonian-gasket | RustFig | x_move_step | 8.2; 8.9; 9.1 |
| apollonian-gasket | RustFig | x_move_place | 18.2; 18.7; 29.7 |
| apollonian-gasket | RustFig | x_save | -; -; - |
| apollonian-gasket | RustFig | x_undo | 14.9; 14.1; 14.7 |
| blankmap | xfig | open | 1191.1; 1216.4; 1160.0 |
| blankmap | xfig | open_read | 33.7; 29.8; 29.5 |
| blankmap | xfig | open_draw | 3.9; 4.5; 4.3 |
| blankmap | xfig | zoom_in | 13.6; 13.7; 12.8 |
| blankmap | xfig | zoom_out | 15.2; 15.8; 15.9 |
| blankmap | xfig | pan | 17.5; 15.3; 17.7 |
| blankmap | xfig | move_step | 1.3; 1.4; 1.3 |
| blankmap | xfig | move_step_max | 1.7; 1.5; 1.7 |
| blankmap | xfig | move_place | 47.4; 51.8; 50.1 |
| blankmap | xfig | save | 29.2; 29.8; 31.0 |
| blankmap | xfig | undo | 47.5; 51.5; 48.7 |
| blankmap | xfig | x_open | 1230.3; 1251.4; 1193.1 |
| blankmap | xfig | x_zoom_in | 38.5; 33.5; 39.7 |
| blankmap | xfig | x_zoom_out | 27.5; 28.1; 34.7 |
| blankmap | xfig | x_pan | 16.1; 17.5; 16.9 |
| blankmap | xfig | x_move_step | 10.0; 10.0; 10.2 |
| blankmap | xfig | x_move_place | 52.8; 53.4; 54.7 |
| blankmap | xfig | x_save | 37.0; 34.2; 38.6 |
| blankmap | xfig | x_undo | 48.6; 52.2; 51.0 |
| blankmap | RustFig | open | 213.9; 186.8; 192.0 |
| blankmap | RustFig | open_read | -; -; - |
| blankmap | RustFig | open_draw | -; -; - |
| blankmap | RustFig | zoom_in | 6.6; 7.4; 6.6 |
| blankmap | RustFig | zoom_out | 6.4; 6.5; 5.3 |
| blankmap | RustFig | pan | 3.3; 3.8; 4.5 |
| blankmap | RustFig | move_step | 0.1; 0.1; 0.1 |
| blankmap | RustFig | move_step_max | 0.6; 0.5; 0.6 |
| blankmap | RustFig | move_place | 8.0; 8.7; 7.7 |
| blankmap | RustFig | save | 4.1; 8.7; 5.5 |
| blankmap | RustFig | undo | 7.9; 7.8; 9.5 |
| blankmap | RustFig | x_open | 213.9; 186.8; 192.0 |
| blankmap | RustFig | x_zoom_in | 10.9; 10.6; 10.3 |
| blankmap | RustFig | x_zoom_out | 10.3; 9.6; 11.4 |
| blankmap | RustFig | x_pan | 10.0; 8.6; 12.0 |
| blankmap | RustFig | x_move_step | 12.4; 9.9; 7.9 |
| blankmap | RustFig | x_move_place | 20.4; 19.8; 21.4 |
| blankmap | RustFig | x_save | -; -; - |
| blankmap | RustFig | x_undo | 13.3; 18.7; 14.5 |
| ne-coastline-10m | xfig | open | 1234.5; 1227.9; 1255.9 |
| ne-coastline-10m | xfig | open_read | 70.3; 70.6; 77.0 |
| ne-coastline-10m | xfig | open_draw | 2.8; 2.9; 2.8 |
| ne-coastline-10m | xfig | zoom_in | 11.1; 13.8; 13.1 |
| ne-coastline-10m | xfig | zoom_out | 12.8; 13.2; 10.3 |
| ne-coastline-10m | xfig | pan | 11.7; 12.4; 12.7 |
| ne-coastline-10m | xfig | move_step | 1.6; 1.6; 1.4 |
| ne-coastline-10m | xfig | move_step_max | 1.7; 1.7; 1.7 |
| ne-coastline-10m | xfig | move_place | 130.1; 129.6; 133.3 |
| ne-coastline-10m | xfig | save | 59.0; 61.0; 63.4 |
| ne-coastline-10m | xfig | undo | 37.6; 37.3; 35.8 |
| ne-coastline-10m | xfig | x_open | 1271.5; 1262.7; 1297.5 |
| ne-coastline-10m | xfig | x_zoom_in | 40.1; 37.7; 30.9 |
| ne-coastline-10m | xfig | x_zoom_out | 38.2; 34.0; 34.4 |
| ne-coastline-10m | xfig | x_pan | 10.7; 15.6; 11.2 |
| ne-coastline-10m | xfig | x_move_step | 10.4; 9.8; 9.5 |
| ne-coastline-10m | xfig | x_move_place | 130.5; 132.4; 136.0 |
| ne-coastline-10m | xfig | x_save | 60.8; 62.7; 64.5 |
| ne-coastline-10m | xfig | x_undo | 41.3; 40.3; 38.2 |
| ne-coastline-10m | RustFig | open | 220.8; 217.7; 198.9 |
| ne-coastline-10m | RustFig | open_read | -; -; - |
| ne-coastline-10m | RustFig | open_draw | -; -; - |
| ne-coastline-10m | RustFig | zoom_in | 16.9; 15.5; 15.8 |
| ne-coastline-10m | RustFig | zoom_out | 8.5; 9.1; 9.6 |
| ne-coastline-10m | RustFig | pan | 7.1; 4.8; 9.0 |
| ne-coastline-10m | RustFig | move_step | 0.1; 0.1; 0.1 |
| ne-coastline-10m | RustFig | move_step_max | 0.7; 0.2; 0.5 |
| ne-coastline-10m | RustFig | move_place | 16.8; 18.7; 17.8 |
| ne-coastline-10m | RustFig | save | 17.0; 15.7; 12.6 |
| ne-coastline-10m | RustFig | undo | 19.6; 17.7; 16.5 |
| ne-coastline-10m | RustFig | x_open | 220.8; 217.7; 198.9 |
| ne-coastline-10m | RustFig | x_zoom_in | 21.4; 19.6; 23.4 |
| ne-coastline-10m | RustFig | x_zoom_out | 12.0; 13.5; 22.8 |
| ne-coastline-10m | RustFig | x_pan | 11.2; 9.8; 17.7 |
| ne-coastline-10m | RustFig | x_move_step | 5.0; 5.4; 7.8 |
| ne-coastline-10m | RustFig | x_move_place | 33.6; 33.4; 27.6 |
| ne-coastline-10m | RustFig | x_save | -; -; - |
| ne-coastline-10m | RustFig | x_undo | 25.1; 26.7; 21.1 |
| ne-countries-10m | xfig | open | 1220.0; 1260.4; 1245.3 |
| ne-countries-10m | xfig | open_read | 89.4; 87.9; 85.9 |
| ne-countries-10m | xfig | open_draw | 5.3; 5.2; 5.4 |
| ne-countries-10m | xfig | zoom_in | 18.3; 19.6; 16.0 |
| ne-countries-10m | xfig | zoom_out | 18.5; 20.2; 18.7 |
| ne-countries-10m | xfig | pan | 21.3; 18.6; 20.4 |
| ne-countries-10m | xfig | move_step | 1.0; 1.0; 1.0 |
| ne-countries-10m | xfig | move_step_max | 1.1; 1.1; 1.1 |
| ne-countries-10m | xfig | move_place | 163.3; 163.3; 162.6 |
| ne-countries-10m | xfig | save | 66.9; 71.0; 69.7 |
| ne-countries-10m | xfig | undo | 44.4; 42.9; 43.0 |
| ne-countries-10m | xfig | x_open | 1260.7; 1298.0; 1278.2 |
| ne-countries-10m | xfig | x_zoom_in | 34.2; 31.7; 28.0 |
| ne-countries-10m | xfig | x_zoom_out | 34.6; 37.9; 32.7 |
| ne-countries-10m | xfig | x_pan | 21.6; 19.6; 23.1 |
| ne-countries-10m | xfig | x_move_step | 10.1; 10.3; 10.1 |
| ne-countries-10m | xfig | x_move_place | 166.3; 166.7; 164.8 |
| ne-countries-10m | xfig | x_save | 68.5; 72.2; 72.4 |
| ne-countries-10m | xfig | x_undo | 46.9; 46.4; 43.2 |
| ne-countries-10m | RustFig | open | 232.0; 214.1; 217.8 |
| ne-countries-10m | RustFig | open_read | -; -; - |
| ne-countries-10m | RustFig | open_draw | -; -; - |
| ne-countries-10m | RustFig | zoom_in | 20.0; 20.9; 19.8 |
| ne-countries-10m | RustFig | zoom_out | 12.2; 13.4; 12.6 |
| ne-countries-10m | RustFig | pan | 8.5; 8.5; 8.6 |
| ne-countries-10m | RustFig | move_step | 0.1; 0.1; 0.1 |
| ne-countries-10m | RustFig | move_step_max | 0.3; 0.3; 0.4 |
| ne-countries-10m | RustFig | move_place | 23.4; 23.3; 24.2 |
| ne-countries-10m | RustFig | save | 16.2; 15.5; 16.1 |
| ne-countries-10m | RustFig | undo | 25.5; 26.2; 25.9 |
| ne-countries-10m | RustFig | x_open | 232.0; 214.1; 217.8 |
| ne-countries-10m | RustFig | x_zoom_in | 23.2; 26.2; 23.9 |
| ne-countries-10m | RustFig | x_zoom_out | 23.0; 20.3; 16.6 |
| ne-countries-10m | RustFig | x_pan | 12.7; 12.7; 12.5 |
| ne-countries-10m | RustFig | x_move_step | 5.0; 6.0; 5.0 |
| ne-countries-10m | RustFig | x_move_place | 33.0; 33.3; 33.5 |
| ne-countries-10m | RustFig | x_save | -; -; - |
| ne-countries-10m | RustFig | x_undo | 39.1; 29.7; 39.2 |
| greece | xfig | open | 1271.0; 1232.6; 1261.2 |
| greece | xfig | open_read | 91.2; 88.2; 81.2 |
| greece | xfig | open_draw | 4.8; 4.9; 5.2 |
| greece | xfig | zoom_in | 14.0; 14.6; 12.1 |
| greece | xfig | zoom_out | 17.0; 16.3; 14.4 |
| greece | xfig | pan | 13.7; 12.1; 14.3 |
| greece | xfig | move_step | 0.6; 0.5; 0.6 |
| greece | xfig | move_step_max | 0.7; 0.7; 0.7 |
| greece | xfig | move_place | 100.1; 104.6; 96.0 |
| greece | xfig | save | 47.7; 43.9; 49.2 |
| greece | xfig | undo | 30.4; 28.8; 31.9 |
| greece | xfig | x_open | 1307.7; 1270.8; 1298.9 |
| greece | xfig | x_zoom_in | 28.5; 37.0; 37.6 |
| greece | xfig | x_zoom_out | 34.2; 39.3; 38.3 |
| greece | xfig | x_pan | 11.8; 13.9; 15.7 |
| greece | xfig | x_move_step | 10.1; 9.6; 10.2 |
| greece | xfig | x_move_place | 101.2; 106.0; 96.6 |
| greece | xfig | x_save | 49.6; 45.5; 53.1 |
| greece | xfig | x_undo | 33.2; 31.9; 34.9 |
| greece | RustFig | open | 282.3; 281.5; 279.4 |
| greece | RustFig | open_read | -; -; - |
| greece | RustFig | open_draw | -; -; - |
| greece | RustFig | zoom_in | 29.6; 29.0; 28.2 |
| greece | RustFig | zoom_out | 28.1; 27.4; 27.9 |
| greece | RustFig | pan | 22.7; 22.7; 23.6 |
| greece | RustFig | move_step | 0.1; 0.1; 0.1 |
| greece | RustFig | move_step_max | 1.0; 3.1; 1.0 |
| greece | RustFig | move_place | 26.1; 24.1; 24.6 |
| greece | RustFig | save | 10.2; 10.5; 9.4 |
| greece | RustFig | undo | 1.9; 26.8; 25.2 |
| greece | RustFig | x_open | 282.3; 281.5; 279.4 |
| greece | RustFig | x_zoom_in | 34.4; 33.2; 32.6 |
| greece | RustFig | x_zoom_out | 33.5; 35.0; 33.6 |
| greece | RustFig | x_pan | 30.9; 30.4; 31.8 |
| greece | RustFig | x_move_step | 7.6; 4.9; 6.7 |
| greece | RustFig | x_move_place | 35.2; 43.5; 34.0 |
| greece | RustFig | x_save | -; -; - |
| greece | RustFig | x_undo | 29.8; 37.2; 32.8 |
| clifford-attractor | xfig | open | 1313.8; 1332.1; 1318.4 |
| clifford-attractor | xfig | open_read | 128.4; 128.4; 130.2 |
| clifford-attractor | xfig | open_draw | 33.4; 33.9; 32.9 |
| clifford-attractor | xfig | zoom_in | 62.7; 60.4; 62.0 |
| clifford-attractor | xfig | zoom_out | 61.7; 66.5; 64.8 |
| clifford-attractor | xfig | pan | 41.0; 41.7; 41.9 |
| clifford-attractor | xfig | move_step | 0.5; 0.6; 0.5 |
| clifford-attractor | xfig | move_step_max | 0.6; 0.8; 0.6 |
| clifford-attractor | xfig | move_place | 101.2; 102.2; 100.2 |
| clifford-attractor | xfig | save | 146.4; 134.3; 130.7 |
| clifford-attractor | xfig | undo | 33.4; 34.1; 33.2 |
| clifford-attractor | xfig | x_open | 1351.3; 1367.3; 1354.4 |
| clifford-attractor | xfig | x_zoom_in | 64.0; 61.3; 62.6 |
| clifford-attractor | xfig | x_zoom_out | 64.1; 66.5; 64.4 |
| clifford-attractor | xfig | x_pan | 40.9; 41.2; 44.8 |
| clifford-attractor | xfig | x_move_step | 10.1; 10.4; 9.6 |
| clifford-attractor | xfig | x_move_place | 103.7; 105.0; 102.0 |
| clifford-attractor | xfig | x_save | 147.4; 138.0; 133.3 |
| clifford-attractor | xfig | x_undo | 35.0; 37.7; 34.1 |
| clifford-attractor | RustFig | open | 405.3; 404.0; 397.4 |
| clifford-attractor | RustFig | open_read | -; -; - |
| clifford-attractor | RustFig | open_draw | -; -; - |
| clifford-attractor | RustFig | zoom_in | 89.1; 95.9; 87.9 |
| clifford-attractor | RustFig | zoom_out | 56.7; 52.3; 50.7 |
| clifford-attractor | RustFig | pan | 57.1; 57.6; 55.7 |
| clifford-attractor | RustFig | move_step | 0.3; 0.2; 0.2 |
| clifford-attractor | RustFig | move_step_max | 0.4; 0.4; 0.5 |
| clifford-attractor | RustFig | move_place | 71.3; 74.7; 74.6 |
| clifford-attractor | RustFig | save | 59.7; 60.0; 58.0 |
| clifford-attractor | RustFig | undo | 77.5; 89.0; 77.9 |
| clifford-attractor | RustFig | x_open | 405.3; 404.0; 397.4 |
| clifford-attractor | RustFig | x_zoom_in | 94.1; 100.7; 92.8 |
| clifford-attractor | RustFig | x_zoom_out | 63.1; 57.3; 54.8 |
| clifford-attractor | RustFig | x_pan | 65.3; 62.0; 59.4 |
| clifford-attractor | RustFig | x_move_step | 7.5; 9.2; 9.9 |
| clifford-attractor | RustFig | x_move_place | 93.3; 98.0; 105.1 |
| clifford-attractor | RustFig | x_save | -; -; - |
| clifford-attractor | RustFig | x_undo | 88.2; 97.4; 82.1 |
| cube8 | xfig | open | 1326.2; 1328.3; 1329.0 |
| cube8 | xfig | open_read | 120.1; 121.6; 121.6 |
| cube8 | xfig | open_draw | 45.1; 45.4; 44.6 |
| cube8 | xfig | zoom_in | 71.0; 64.2; 65.9 |
| cube8 | xfig | zoom_out | 79.9; 81.1; 79.6 |
| cube8 | xfig | pan | 70.3; 70.2; 66.4 |
| cube8 | xfig | move_step | 0.5; 0.5; 0.5 |
| cube8 | xfig | move_step_max | 0.6; 0.6; 0.7 |
| cube8 | xfig | move_place | 142.6; 141.5; 140.5 |
| cube8 | xfig | save | 116.1; 117.0; 118.1 |
| cube8 | xfig | undo | 38.8; 38.4; 36.3 |
| cube8 | xfig | x_open | 1361.3; 1363.6; 1365.8 |
| cube8 | xfig | x_zoom_in | 71.9; 64.7; 65.8 |
| cube8 | xfig | x_zoom_out | 81.7; 84.1; 79.1 |
| cube8 | xfig | x_pan | 72.7; 72.2; 68.7 |
| cube8 | xfig | x_move_step | 9.9; 9.8; 10.0 |
| cube8 | xfig | x_move_place | 145.8; 141.5; 143.0 |
| cube8 | xfig | x_save | 119.5; 119.9; 121.3 |
| cube8 | xfig | x_undo | 42.0; 39.6; 37.2 |
| cube8 | RustFig | open | 1139.5; 1176.1; 1197.3 |
| cube8 | RustFig | open_read | -; -; - |
| cube8 | RustFig | open_draw | -; -; - |
| cube8 | RustFig | zoom_in | 453.5; 509.0; 433.3 |
| cube8 | RustFig | zoom_out | 433.4; 415.4; 422.5 |
| cube8 | RustFig | pan | 341.0; 320.3; 332.2 |
| cube8 | RustFig | move_step | 0.3; 0.2; 0.2 |
| cube8 | RustFig | move_step_max | 2.6; 0.6; 1.7 |
| cube8 | RustFig | move_place | 410.7; 433.0; 425.5 |
| cube8 | RustFig | save | 118.5; 120.0; 121.2 |
| cube8 | RustFig | undo | 578.6; 641.4; 411.9 |
| cube8 | RustFig | x_open | 1139.5; 1176.1; 1197.3 |
| cube8 | RustFig | x_zoom_in | 434.8; 483.4; 426.9 |
| cube8 | RustFig | x_zoom_out | 413.2; 400.3; 412.1 |
| cube8 | RustFig | x_pan | 331.4; 315.9; 313.9 |
| cube8 | RustFig | x_move_step | 12.9; 10.9; 11.9 |
| cube8 | RustFig | x_move_place | 434.8; 470.2; 457.6 |
| cube8 | RustFig | x_save | -; -; - |
| cube8 | RustFig | x_undo | 567.8; 620.7; 415.3 |

</details>

Move test: 30 steps over 240 x 120 pixels. Figures with more than 300 objects (snowy, penrose-tiling, apollonian-gasket, blankmap, ne-coastline-10m, ne-countries-10m, greece, clifford-attractor, cube8) were glued into one compound first (xfig: Glue, middle-button region, right button; RustFig: select all, group), smaller ones move a single object.

<!-- results-retina:end -->
