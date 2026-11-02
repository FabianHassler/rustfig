# Speed comparison with xfig

**Status:** 2026-11-02. Measured with `cargo xtask speed-compare` (see below); the tables inside the result blocks are generated, everything else is written by hand. Revision history: 2026-11-02 first version (RustFig = local main e2c3c50, after the speed work, measured on a quiet Mac mini M4 at equal resolution).

RustFig against the real xfig 3.2.9a on the same figures: open, zoom, pan, move, save, undo. This page is the evidence for a speed claim on the website; it says where xfig is faster as well.

## Setup

**What is compared**

- **xfig 3.2.9a**, built from the unmodified sources in `old/xfig-3.2.9a` with the same configure flags, dependencies (Xaw3d, Xft, libpng, jpeg-turbo, libtiff, libxpm, ghostscript, Homebrew's app-defaults directory with `Fig.international: False`) and `CFLAGS=-O2` as Homebrew's formula (`brew cat xfig`), plus timing hooks. `xtask/xfig-timing/build.sh` copies the sources, applies `xtask/xfig-timing/xfig-timing.patch` (the hooks only; `old/` is never touched) and builds. With `XFIG_TIMING=1` the hooks print `XT <name> <start ms> <duration ms>` on stderr (CLOCK_MONOTONIC); without the variable they do nothing. Default options, scratch `HOME`, on its own `Xvfb :78` (1400x1000x24, software X server, started with `-fbdir`), window `1322x818`, canvas 1128x686 pixels. The build needs `-std=gnu17` with a current compiler (xfig's C is not C23-clean); otherwise the flags are Homebrew's.
- **RustFig**: release build, `--background`, scratch settings and recovery folders, driven through its control port. **Main table (same resolution)**: `RUSTFIG_PIXELS_PER_POINT=1` (a testing option added with this page, read at start-up: one device pixel per point on any display, and `-geometry` is then the window size in device pixels) and `-geometry 1398x803`, which gives a canvas of 1128x686 device pixels, the same as xfig's (checked in `ui.state` and printed in each result block). **Second table (Retina 2x, for information)**: without the option, the display's own density (this display: 2x), `-geometry 1398x803` points, so the canvas is 1128x686 points = 2256x1372 pixels, four times the pixels of xfig's. The build under test is given to `cargo xtask speed-compare --rustfig BIN`.
- Machine: **Mac mini (M4, 10 cores: 4 performance + 6 efficiency, 32 GB), macOS 27.2**, 4K display (2x scaling), a user logged in; load average 1.3-4.4 during the runs (some other activity). Earlier runs on a busy MacBook (M5, load 4-25, RustFig built before the Gosper fix) are not reported: they were too noisy. xfig and RustFig ran one after the other, never in parallel. The same figures, plain `.fig` copies (xfig reads `.fig.gz` too, but both get the same file). A new process for every run; each value is the **median of 3 runs**; a measurement that does not finish in 120 s is reported as "> 120 s".

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

Same resolution, Mac mini M4 (the main table below). Time of RustFig's own work against xfig's own hooks; above 1x = RustFig faster.

- **Open** (xfig `main()` to drawn; RustFig process start to drawn): RustFig is **4-6.5x faster on 13 of 15 figures** (178-285 ms against xfig's 1.1-1.2 s), 2.6x on clifford-attractor (250,000 objects: 483 ms against 1.26 s), but **slower on cube8** (1.77 s against 1.30 s; 190,000 polylines). xfig spends about 1.1 s in start-up before it reads the file; the reading (3-115 ms) and the first redraw (1-125 ms) are small.
- **Zoom and pan** (one step, half a screen): a draw. Both finish in 2-25 ms on figures up to a few thousand objects; RustFig is ahead by 1.3-7x on the small and mid figures for zoom in (carnot 4.7x, feynman 7x, fourier 6.7x), behind by about 2x on the polyline-heavy maps (ne-coastline, greece, apollonian: 0.5-0.6x) and on every pan of the small figures (xfig 4 ms against 10-15 ms). On clifford-attractor both are within 10 % (zoom 60 against 54 ms); **cube8 is the exception: xfig 3-4x faster** (zoom 65 against 230 ms).
- **Move, the drag**: both draw a step in about 1 ms on any figure (xfig an XOR outline, a compound's bounding box; RustFig the real object over a cached layer): **equal**.
- **Move, placing** (the final redraw): RustFig faster on the figures where xfig redraws slowly: ne-countries 164 against 37 ms (4.5x), ne-coastline 129 against 29 ms (4.5x), greece 105 against 34 ms (3.1x), blankmap 2.5x, wenig 2.6x, gosper-curve 2.2x; xfig faster on the smallest figures (carnot 5.5 against 18 ms) and on cube8 (141 against 517 ms).
- **Undo, Save**: Save is faster in xfig on small files (2 against 10 ms) and in RustFig on large ones (ne-coastline 35 against 66 ms, 1.9x); Undo mixed (RustFig 6.8x on wenig, 2.7x on blankmap, xfig 4x on small figures).
- **Not measurable for RustFig**: in the glue-then-move task on clifford-attractor (250,001 objects) `edit.select_all` did not return within the control channel's 60 s (a separate probe: about 65 s), so move step, place, save and undo are `n/a` there; open, zoom and pan were measured. The same happened in some cube8 runs, which are left out of that median. This is a RustFig finding (select all is far too slow beyond ~100,000 objects). xfig finished everything in under 1.3 s.
- **Retina 2x** (second table, for information): RustFig draws four times the pixels, yet the numbers are not slower, mostly the same or better (open 163-258 ms, cube8 zoom 618 ms against 230 ms in the main table: noise or caching differences matter more than the pixel count at this scale). Do not read a resolution effect into it without more runs.

## Results at the same resolution (RustFig 1128x686 device pixels, as xfig)

<!-- results:start -->

#### Setup of this run

- Machine: Apple M4, macOS 27.2
- xfig: `/Users/fabian/rustfig-speed/xfig-timing/src/src/xfig` (3.2.9a), default options, scratch `HOME`, on `Xvfb :78` 1400x1000x24; window `1322x818` (canvas 1128x686 pixels)
- RustFig: `target/release/rustfig` (rustfig 0.9.0-beta.7), release build, label: main e2c3c50 + RUSTFIG_PIXELS_PER_POINT (same resolution); `-geometry 1398x803`, `RUSTFIG_PIXELS_PER_POINT=1` (window size in device pixels)
- Each value is the median of 3 run(s), a new process per run; timeout 120 s per measurement ("> N s": not finished in that time; the real time is longer)
- Generated 2026-11-02 19:26 by `cargo xtask speed-compare`

- RustFig canvas as reported by `ui.state` (points): 1128x686; density 1 device pixel per point (xfig's canvas: 1128x686 pixels)
#### Results

xfig / RustFig is the speed-up of RustFig (above 1 = RustFig is faster).

##### Open (xfig: main() to figure drawn; RustFig: process start to drawn)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.15 s | 278 ms | 4.1x |
| feynman (small) | 4 kB, 34 obj. | 1.15 s | 238 ms | 4.9x |
| fourier (small) | 16 kB, 41 obj. | 1.16 s | 247 ms | 4.7x |
| pines | 11 kB, 31 obj. | 1.11 s | 235 ms | 4.7x |
| snowy | 173 kB, 436 obj. | 1.11 s | 213 ms | 5.2x |
| wenig | 587 kB, 85 obj. | 1.12 s | 201 ms | 5.6x |
| gosper-curve | 182 kB, 142 obj. | 1.12 s | 178 ms | 6.3x |
| penrose-tiling | 560 kB, 7615 obj. | 1.13 s | 187 ms | 6.0x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.12 s | 198 ms | 5.7x |
| blankmap | 1.0 MB, 2232 obj. | 1.19 s | 182 ms | 6.5x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.23 s | 235 ms | 5.2x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.20 s | 240 ms | 5.0x |
| greece | 3.2 MB, 2013 obj. | 1.19 s | 285 ms | 4.2x |
| clifford-attractor | 13.2 MB, 250001 obj. | 1.26 s | 483 ms | 2.6x |
| cube8 | 14.6 MB, 190260 obj. | 1.30 s | 1.77 s | 0.7x |

##### Open, part 1: reading the file (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 46.8 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 46.4 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 46.0 ms | n/a | - |
| pines | 11 kB, 31 obj. | 2.5 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 5.5 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 17.8 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 8.5 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 18.5 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 11.1 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 32.1 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 73.9 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 86.4 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 83.6 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 128 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 119 ms | n/a | - |

##### Open, part 2: first full canvas redraw (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 2.5 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 2.1 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 1.9 ms | n/a | - |
| pines | 11 kB, 31 obj. | 3.9 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 4.8 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 10.9 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 3.9 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 10.3 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 6.7 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 4.5 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 2.9 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 5.1 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 4.7 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 32.4 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 45.3 ms | n/a | - |

##### Zoom in (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 25.5 ms | 5.4 ms | 4.7x |
| feynman (small) | 4 kB, 34 obj. | 33.0 ms | 4.7 ms | 7.0x |
| fourier (small) | 16 kB, 41 obj. | 25.7 ms | 3.8 ms | 6.7x |
| pines | 11 kB, 31 obj. | 6.6 ms | 4.6 ms | 1.4x |
| snowy | 173 kB, 436 obj. | 6.9 ms | 2.7 ms | 2.6x |
| wenig | 587 kB, 85 obj. | 17.1 ms | 11.6 ms | 1.5x |
| gosper-curve | 182 kB, 142 obj. | 8.2 ms | 15.6 ms | 0.5x |
| penrose-tiling | 560 kB, 7615 obj. | 19.9 ms | 14.5 ms | 1.4x |
| apollonian-gasket | 330 kB, 4221 obj. | 9.2 ms | 15.2 ms | 0.6x |
| blankmap | 1.0 MB, 2232 obj. | 13.8 ms | 10.8 ms | 1.3x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 7.6 ms | 16.5 ms | 0.5x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 19.3 ms | 18.3 ms | 1.1x |
| greece | 3.2 MB, 2013 obj. | 13.8 ms | 24.2 ms | 0.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 60.4 ms | 53.9 ms | 1.1x |
| cube8 | 14.6 MB, 190260 obj. | 64.5 ms | 230 ms | 0.3x |

##### Zoom out (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 6.1 ms | 6.5 ms | 0.9x |
| feynman (small) | 4 kB, 34 obj. | 5.6 ms | 15.6 ms | 0.4x |
| fourier (small) | 16 kB, 41 obj. | 4.7 ms | 9.3 ms | 0.5x |
| pines | 11 kB, 31 obj. | 8.2 ms | 7.1 ms | 1.1x |
| snowy | 173 kB, 436 obj. | 5.7 ms | 3.9 ms | 1.5x |
| wenig | 587 kB, 85 obj. | 14.9 ms | 12.4 ms | 1.2x |
| gosper-curve | 182 kB, 142 obj. | 8.5 ms | 8.2 ms | 1.0x |
| penrose-tiling | 560 kB, 7615 obj. | 23.8 ms | 17.3 ms | 1.4x |
| apollonian-gasket | 330 kB, 4221 obj. | 11.7 ms | 17.0 ms | 0.7x |
| blankmap | 1.0 MB, 2232 obj. | 15.1 ms | 13.7 ms | 1.1x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 12.0 ms | 16.8 ms | 0.7x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 19.8 ms | 19.0 ms | 1.0x |
| greece | 3.2 MB, 2013 obj. | 15.2 ms | 1.7 ms | 8.8x |
| clifford-attractor | 13.2 MB, 250001 obj. | 62.2 ms | 56.7 ms | 1.1x |
| cube8 | 14.6 MB, 190260 obj. | 82.5 ms | 290 ms | 0.3x |

##### Pan half a screen

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 4.4 ms | 14.9 ms | 0.3x |
| feynman (small) | 4 kB, 34 obj. | 4.1 ms | 11.6 ms | 0.4x |
| fourier (small) | 16 kB, 41 obj. | 4.5 ms | 10.0 ms | 0.4x |
| pines | 11 kB, 31 obj. | 3.8 ms | 6.8 ms | 0.6x |
| snowy | 173 kB, 436 obj. | 4.4 ms | 2.6 ms | 1.7x |
| wenig | 587 kB, 85 obj. | 16.8 ms | 8.1 ms | 2.1x |
| gosper-curve | 182 kB, 142 obj. | 5.2 ms | 7.5 ms | 0.7x |
| penrose-tiling | 560 kB, 7615 obj. | 9.6 ms | 14.5 ms | 0.7x |
| apollonian-gasket | 330 kB, 4221 obj. | 10.7 ms | 13.2 ms | 0.8x |
| blankmap | 1.0 MB, 2232 obj. | 16.2 ms | 9.4 ms | 1.7x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 10.5 ms | 12.8 ms | 0.8x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 19.8 ms | 16.4 ms | 1.2x |
| greece | 3.2 MB, 2013 obj. | 11.8 ms | 24.4 ms | 0.5x |
| clifford-attractor | 13.2 MB, 250001 obj. | 44.4 ms | 58.1 ms | 0.8x |
| cube8 | 14.6 MB, 190260 obj. | 69.4 ms | 163 ms | 0.4x |

##### Move: one step of the drag (median of 30)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 0.8 ms | 0.8 ms | 1.0x |
| feynman (small) | 4 kB, 34 obj. | 9.1 ms | 0.8 ms | 12x |
| fourier (small) | 16 kB, 41 obj. | 0.8 ms | 0.9 ms | 0.9x |
| pines | 11 kB, 31 obj. | 1.0 ms | 0.8 ms | 1.2x |
| snowy | 173 kB, 436 obj. | 0.5 ms | 0.7 ms | 0.7x |
| wenig | 587 kB, 85 obj. | 1.8 ms | 0.7 ms | 2.4x |
| gosper-curve | 182 kB, 142 obj. | 0.8 ms | 1.0 ms | 0.8x |
| penrose-tiling | 560 kB, 7615 obj. | 0.6 ms | 1.4 ms | 0.4x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.4 ms | 1.2 ms | 1.2x |
| blankmap | 1.0 MB, 2232 obj. | 1.4 ms | 1.0 ms | 1.4x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.7 ms | 1.0 ms | 1.6x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.1 ms | 0.9 ms | 1.2x |
| greece | 3.2 MB, 2013 obj. | 0.6 ms | 0.9 ms | 0.7x |
| clifford-attractor | 13.2 MB, 250001 obj. | 0.6 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 0.6 ms | 1.3 ms | 0.4x |

##### Move: place (final redraw)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 5.5 ms | 18.0 ms | 0.3x |
| feynman (small) | 4 kB, 34 obj. | 53.6 ms | 13.0 ms | 4.1x |
| fourier (small) | 16 kB, 41 obj. | 3.5 ms | 10.4 ms | 0.3x |
| pines | 11 kB, 31 obj. | 7.3 ms | 6.3 ms | 1.2x |
| snowy | 173 kB, 436 obj. | 14.7 ms | 6.2 ms | 2.4x |
| wenig | 587 kB, 85 obj. | 37.1 ms | 14.5 ms | 2.6x |
| gosper-curve | 182 kB, 142 obj. | 19.5 ms | 8.9 ms | 2.2x |
| penrose-tiling | 560 kB, 7615 obj. | 23.9 ms | 18.9 ms | 1.3x |
| apollonian-gasket | 330 kB, 4221 obj. | 18.1 ms | 30.0 ms | 0.6x |
| blankmap | 1.0 MB, 2232 obj. | 48.9 ms | 19.2 ms | 2.5x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 129 ms | 28.9 ms | 4.5x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 164 ms | 36.6 ms | 4.5x |
| greece | 3.2 MB, 2013 obj. | 105 ms | 34.3 ms | 3.1x |
| clifford-attractor | 13.2 MB, 250001 obj. | 105 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 141 ms | 517 ms | 0.3x |

##### Save

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.6 ms | 10.8 ms | 0.1x |
| feynman (small) | 4 kB, 34 obj. | 2.4 ms | 9.7 ms | 0.2x |
| fourier (small) | 16 kB, 41 obj. | 1.8 ms | 10.7 ms | 0.2x |
| pines | 11 kB, 31 obj. | 1.8 ms | 11.3 ms | 0.2x |
| snowy | 173 kB, 436 obj. | 5.5 ms | 13.2 ms | 0.4x |
| wenig | 587 kB, 85 obj. | 15.0 ms | 10.2 ms | 1.5x |
| gosper-curve | 182 kB, 142 obj. | 9.0 ms | 6.4 ms | 1.4x |
| penrose-tiling | 560 kB, 7615 obj. | 21.3 ms | 19.1 ms | 1.1x |
| apollonian-gasket | 330 kB, 4221 obj. | 12.5 ms | 21.2 ms | 0.6x |
| blankmap | 1.0 MB, 2232 obj. | 28.9 ms | 21.7 ms | 1.3x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 66.0 ms | 35.1 ms | 1.9x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 73.3 ms | 39.9 ms | 1.8x |
| greece | 3.2 MB, 2013 obj. | 51.0 ms | 32.5 ms | 1.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 136 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 115 ms | 136 ms | 0.8x |

##### Undo the move

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 3.5 ms | 21.1 ms | 0.2x |
| feynman (small) | 4 kB, 34 obj. | 4.5 ms | 14.3 ms | 0.3x |
| fourier (small) | 16 kB, 41 obj. | 2.5 ms | 10.9 ms | 0.2x |
| pines | 11 kB, 31 obj. | 5.5 ms | 8.8 ms | 0.6x |
| snowy | 173 kB, 436 obj. | 3.1 ms | 9.1 ms | 0.3x |
| wenig | 587 kB, 85 obj. | 37.0 ms | 5.5 ms | 6.8x |
| gosper-curve | 182 kB, 142 obj. | 5.4 ms | 2.6 ms | 2.1x |
| penrose-tiling | 560 kB, 7615 obj. | 6.9 ms | 18.2 ms | 0.4x |
| apollonian-gasket | 330 kB, 4221 obj. | 20.3 ms | 30.0 ms | 0.7x |
| blankmap | 1.0 MB, 2232 obj. | 53.5 ms | 19.7 ms | 2.7x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 36.5 ms | 34.8 ms | 1.1x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 47.1 ms | 40.7 ms | 1.2x |
| greece | 3.2 MB, 2013 obj. | 28.8 ms | 35.0 ms | 0.8x |
| clifford-attractor | 13.2 MB, 250001 obj. | 36.5 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 38.4 ms | 629 ms | 0.1x |

##### Cross-check: the same actions timed from outside

xfig: its own hooks against watching the canvas pixels; RustFig: its frame times against the wall time until `ui.state` reports the canvas current (never below one frame, 16.7 ms).

| Figure | Task | xfig, own hooks | xfig, pixels | RustFig, frame times | RustFig, wall to idle |
|---|---|---|---|---|---|
| carnot (small) | open | 1.15 s | 1.19 s | 278 ms | 278 ms |
| carnot (small) | zoom_in | 25.5 ms | 32.1 ms | 5.4 ms | 16.7 ms |
| carnot (small) | zoom_out | 6.1 ms | 30.1 ms | 6.5 ms | 16.7 ms |
| carnot (small) | pan | 4.4 ms | 9.9 ms | 14.9 ms | 25.1 ms |
| carnot (small) | move_step | 0.8 ms | 9.0 ms | 0.8 ms | 50.4 ms |
| carnot (small) | move_place | 5.5 ms | 52.9 ms | 18.0 ms | 83.7 ms |
| carnot (small) | save | 1.6 ms | 32.7 ms | 10.8 ms | n/a |
| carnot (small) | undo | 3.5 ms | 31.2 ms | 21.1 ms | 38.2 ms |
| feynman (small) | open | 1.15 s | 1.19 s | 238 ms | 238 ms |
| feynman (small) | zoom_in | 33.0 ms | 32.4 ms | 4.7 ms | 16.7 ms |
| feynman (small) | zoom_out | 5.6 ms | 32.1 ms | 15.6 ms | 24.6 ms |
| feynman (small) | pan | 4.1 ms | 11.2 ms | 11.6 ms | 23.2 ms |
| feynman (small) | move_step | 9.1 ms | 9.1 ms | 0.8 ms | 50.0 ms |
| feynman (small) | move_place | 53.6 ms | 53.6 ms | 13.0 ms | 83.5 ms |
| feynman (small) | save | 2.4 ms | 34.1 ms | 9.7 ms | n/a |
| feynman (small) | undo | 4.5 ms | 35.4 ms | 14.3 ms | 38.3 ms |
| fourier (small) | open | 1.16 s | 1.20 s | 247 ms | 247 ms |
| fourier (small) | zoom_in | 25.7 ms | 32.7 ms | 3.8 ms | 16.7 ms |
| fourier (small) | zoom_out | 4.7 ms | 34.4 ms | 9.3 ms | 23.8 ms |
| fourier (small) | pan | 4.5 ms | 10.4 ms | 10.0 ms | 23.9 ms |
| fourier (small) | move_step | 0.8 ms | 8.9 ms | 0.9 ms | 50.1 ms |
| fourier (small) | move_place | 3.5 ms | 53.2 ms | 10.4 ms | 82.5 ms |
| fourier (small) | save | 1.8 ms | 30.3 ms | 10.7 ms | n/a |
| fourier (small) | undo | 2.5 ms | 33.2 ms | 10.9 ms | 27.6 ms |
| pines | open | 1.11 s | 1.15 s | 235 ms | 235 ms |
| pines | zoom_in | 6.6 ms | 30.3 ms | 4.6 ms | 20.5 ms |
| pines | zoom_out | 8.2 ms | 35.5 ms | 7.1 ms | 23.0 ms |
| pines | pan | 3.8 ms | 10.4 ms | 6.8 ms | 22.3 ms |
| pines | move_step | 1.0 ms | 8.9 ms | 0.8 ms | 50.1 ms |
| pines | move_place | 7.3 ms | 53.0 ms | 6.3 ms | 74.0 ms |
| pines | save | 1.8 ms | 34.3 ms | 11.3 ms | n/a |
| pines | undo | 5.5 ms | 34.5 ms | 8.8 ms | 27.0 ms |
| snowy | open | 1.11 s | 1.14 s | 213 ms | 213 ms |
| snowy | zoom_in | 6.9 ms | 30.5 ms | 2.7 ms | 16.9 ms |
| snowy | zoom_out | 5.7 ms | 29.9 ms | 3.9 ms | 16.7 ms |
| snowy | pan | 4.4 ms | 8.9 ms | 2.6 ms | 16.6 ms |
| snowy | move_step | 0.5 ms | 8.6 ms | 0.7 ms | 50.2 ms |
| snowy | move_place | 14.7 ms | 52.1 ms | 6.2 ms | 67.1 ms |
| snowy | save | 5.5 ms | 33.1 ms | 13.2 ms | n/a |
| snowy | undo | 3.1 ms | 33.8 ms | 9.1 ms | 29.3 ms |
| wenig | open | 1.12 s | 1.16 s | 201 ms | 201 ms |
| wenig | zoom_in | 17.1 ms | 31.9 ms | 11.6 ms | 23.4 ms |
| wenig | zoom_out | 14.9 ms | 31.6 ms | 12.4 ms | 22.5 ms |
| wenig | pan | 16.8 ms | 14.7 ms | 8.1 ms | 23.4 ms |
| wenig | move_step | 1.8 ms | 8.7 ms | 0.7 ms | 50.0 ms |
| wenig | move_place | 37.1 ms | 49.9 ms | 14.5 ms | 108 ms |
| wenig | save | 15.0 ms | 33.3 ms | 10.2 ms | n/a |
| wenig | undo | 37.0 ms | 37.1 ms | 5.5 ms | 27.0 ms |
| gosper-curve | open | 1.12 s | 1.16 s | 178 ms | 178 ms |
| gosper-curve | zoom_in | 8.2 ms | 35.4 ms | 15.6 ms | 31.7 ms |
| gosper-curve | zoom_out | 8.5 ms | 30.6 ms | 8.2 ms | 23.5 ms |
| gosper-curve | pan | 5.2 ms | 11.2 ms | 7.5 ms | 22.9 ms |
| gosper-curve | move_step | 0.8 ms | 9.8 ms | 1.0 ms | 50.2 ms |
| gosper-curve | move_place | 19.5 ms | 53.5 ms | 8.9 ms | 87.6 ms |
| gosper-curve | save | 9.0 ms | 35.0 ms | 6.4 ms | n/a |
| gosper-curve | undo | 5.4 ms | 34.3 ms | 2.6 ms | 18.6 ms |
| penrose-tiling | open | 1.13 s | 1.17 s | 187 ms | 187 ms |
| penrose-tiling | zoom_in | 19.9 ms | 31.4 ms | 14.5 ms | 27.3 ms |
| penrose-tiling | zoom_out | 23.8 ms | 28.8 ms | 17.3 ms | 26.7 ms |
| penrose-tiling | pan | 9.6 ms | 8.6 ms | 14.5 ms | 31.0 ms |
| penrose-tiling | move_step | 0.6 ms | 9.8 ms | 1.4 ms | 50.3 ms |
| penrose-tiling | move_place | 23.9 ms | 52.2 ms | 18.9 ms | 76.9 ms |
| penrose-tiling | save | 21.3 ms | 33.4 ms | 19.1 ms | n/a |
| penrose-tiling | undo | 6.9 ms | 32.7 ms | 18.2 ms | 33.3 ms |
| apollonian-gasket | open | 1.12 s | 1.15 s | 198 ms | 198 ms |
| apollonian-gasket | zoom_in | 9.2 ms | 34.2 ms | 15.2 ms | 27.1 ms |
| apollonian-gasket | zoom_out | 11.7 ms | 30.3 ms | 17.0 ms | 26.7 ms |
| apollonian-gasket | pan | 10.7 ms | 10.9 ms | 13.2 ms | 24.6 ms |
| apollonian-gasket | move_step | 1.4 ms | 10.0 ms | 1.2 ms | 50.5 ms |
| apollonian-gasket | move_place | 18.1 ms | 60.9 ms | 30.0 ms | 94.1 ms |
| apollonian-gasket | save | 12.5 ms | 40.7 ms | 21.2 ms | n/a |
| apollonian-gasket | undo | 20.3 ms | 36.6 ms | 30.0 ms | 40.3 ms |
| blankmap | open | 1.19 s | 1.23 s | 182 ms | 182 ms |
| blankmap | zoom_in | 13.8 ms | 38.0 ms | 10.8 ms | 24.5 ms |
| blankmap | zoom_out | 15.1 ms | 38.3 ms | 13.7 ms | 23.5 ms |
| blankmap | pan | 16.2 ms | 16.5 ms | 9.4 ms | 24.0 ms |
| blankmap | move_step | 1.4 ms | 10.3 ms | 1.0 ms | 50.3 ms |
| blankmap | move_place | 48.9 ms | 54.9 ms | 19.2 ms | 82.9 ms |
| blankmap | save | 28.9 ms | 35.4 ms | 21.7 ms | n/a |
| blankmap | undo | 53.5 ms | 55.3 ms | 19.7 ms | 31.3 ms |
| ne-coastline-10m | open | 1.23 s | 1.27 s | 235 ms | 235 ms |
| ne-coastline-10m | zoom_in | 7.6 ms | 38.7 ms | 16.5 ms | 31.1 ms |
| ne-coastline-10m | zoom_out | 12.0 ms | 35.9 ms | 16.8 ms | 29.4 ms |
| ne-coastline-10m | pan | 10.5 ms | 10.8 ms | 12.8 ms | 23.2 ms |
| ne-coastline-10m | move_step | 1.7 ms | 10.5 ms | 1.0 ms | 50.3 ms |
| ne-coastline-10m | move_place | 129 ms | 130 ms | 28.9 ms | 91.8 ms |
| ne-coastline-10m | save | 66.0 ms | 68.2 ms | 35.1 ms | n/a |
| ne-coastline-10m | undo | 36.5 ms | 39.7 ms | 34.8 ms | 53.1 ms |
| ne-countries-10m | open | 1.20 s | 1.23 s | 240 ms | 240 ms |
| ne-countries-10m | zoom_in | 19.3 ms | 30.1 ms | 18.3 ms | 23.9 ms |
| ne-countries-10m | zoom_out | 19.8 ms | 29.6 ms | 19.0 ms | 28.1 ms |
| ne-countries-10m | pan | 19.8 ms | 19.6 ms | 16.4 ms | 28.2 ms |
| ne-countries-10m | move_step | 1.1 ms | 9.9 ms | 0.9 ms | 50.5 ms |
| ne-countries-10m | move_place | 164 ms | 164 ms | 36.6 ms | 98.9 ms |
| ne-countries-10m | save | 73.3 ms | 75.8 ms | 39.9 ms | n/a |
| ne-countries-10m | undo | 47.1 ms | 49.1 ms | 40.7 ms | 53.3 ms |
| greece | open | 1.19 s | 1.23 s | 285 ms | 285 ms |
| greece | zoom_in | 13.8 ms | 33.2 ms | 24.2 ms | 37.2 ms |
| greece | zoom_out | 15.2 ms | 31.5 ms | 1.7 ms | 49.5 ms |
| greece | pan | 11.8 ms | 10.8 ms | 24.4 ms | 46.7 ms |
| greece | move_step | 0.6 ms | 9.9 ms | 0.9 ms | 50.5 ms |
| greece | move_place | 105 ms | 106 ms | 34.3 ms | 99.5 ms |
| greece | save | 51.0 ms | 54.1 ms | 32.5 ms | n/a |
| greece | undo | 28.8 ms | 30.0 ms | 35.0 ms | 56.5 ms |
| clifford-attractor | open | 1.26 s | 1.30 s | 483 ms | 483 ms |
| clifford-attractor | zoom_in | 60.4 ms | 62.8 ms | 53.9 ms | 54.2 ms |
| clifford-attractor | zoom_out | 62.2 ms | 64.0 ms | 56.7 ms | 54.2 ms |
| clifford-attractor | pan | 44.4 ms | 46.1 ms | 58.1 ms | 54.1 ms |
| clifford-attractor | move_step | 0.6 ms | 9.9 ms | n/a | n/a |
| clifford-attractor | move_place | 105 ms | 107 ms | n/a | n/a |
| clifford-attractor | save | 136 ms | 138 ms | n/a | n/a |
| clifford-attractor | undo | 36.5 ms | 39.2 ms | n/a | n/a |
| cube8 | open | 1.30 s | 1.34 s | 1.77 s | 1.77 s |
| cube8 | zoom_in | 64.5 ms | 65.4 ms | 230 ms | 232 ms |
| cube8 | zoom_out | 82.5 ms | 82.7 ms | 290 ms | 282 ms |
| cube8 | pan | 69.4 ms | 71.5 ms | 163 ms | 166 ms |
| cube8 | move_step | 0.6 ms | 10.2 ms | 1.3 ms | 50.6 ms |
| cube8 | move_place | 141 ms | 143 ms | 517 ms | 592 ms |
| cube8 | save | 115 ms | 117 ms | 136 ms | n/a |
| cube8 | undo | 38.4 ms | 41.7 ms | 629 ms | 594 ms |

<details><summary>Raw numbers (every run, milliseconds)</summary>

| Figure | App | Task | Runs |
|---|---|---|---|
| carnot (small) | xfig | open | 1166.9; 1152.8; 1143.2 |
| carnot (small) | xfig | open_read | 49.6; 43.2; 46.8 |
| carnot (small) | xfig | open_draw | 2.1; 2.6; 2.5 |
| carnot (small) | xfig | zoom_in | 25.5; 25.5; 25.9 |
| carnot (small) | xfig | zoom_out | 5.1; 6.5; 6.1 |
| carnot (small) | xfig | pan | 3.9; 4.4; 5.4 |
| carnot (small) | xfig | move_step | 0.7; 0.8; 0.9 |
| carnot (small) | xfig | move_step_max | 1.2; 1.3; 1.3 |
| carnot (small) | xfig | move_place | 5.5; 3.9; 5.6 |
| carnot (small) | xfig | save | 1.6; 1.5; 2.9 |
| carnot (small) | xfig | undo | 3.5; 2.5; 4.3 |
| carnot (small) | xfig | x_open | 1208.8; 1188.2; 1174.4 |
| carnot (small) | xfig | x_zoom_in | 32.1; 32.3; 30.1 |
| carnot (small) | xfig | x_zoom_out | 34.2; 30.1; 30.1 |
| carnot (small) | xfig | x_pan | 9.7; 9.9; 10.5 |
| carnot (small) | xfig | x_move_step | 8.4; 9.0; 9.0 |
| carnot (small) | xfig | x_move_place | 52.9; 53.3; 52.2 |
| carnot (small) | xfig | x_save | 32.7; 28.1; 35.5 |
| carnot (small) | xfig | x_undo | 31.2; 30.2; 34.0 |
| carnot (small) | RustFig | open | 284.3; 278.3; 274.0 |
| carnot (small) | RustFig | open_read | -; -; - |
| carnot (small) | RustFig | open_draw | -; -; - |
| carnot (small) | RustFig | zoom_in | 5.2; 5.5; 5.4 |
| carnot (small) | RustFig | zoom_out | 6.6; 6.5; 6.4 |
| carnot (small) | RustFig | pan | 13.1; 14.9; 15.8 |
| carnot (small) | RustFig | move_step | 0.8; 0.7; 0.9 |
| carnot (small) | RustFig | move_step_max | 1.3; 1.2; 1.5 |
| carnot (small) | RustFig | move_place | 18.3; 17.5; 18.0 |
| carnot (small) | RustFig | save | 9.3; 10.8; 11.9 |
| carnot (small) | RustFig | undo | 21.1; 21.0; 21.8 |
| carnot (small) | RustFig | x_open | 284.3; 278.3; 274.0 |
| carnot (small) | RustFig | x_zoom_in | 16.7; 16.7; 16.7 |
| carnot (small) | RustFig | x_zoom_out | 16.7; 16.7; 16.8 |
| carnot (small) | RustFig | x_pan | 22.2; 25.1; 25.5 |
| carnot (small) | RustFig | x_move_step | 50.4; 50.2; 50.4 |
| carnot (small) | RustFig | x_move_place | 93.4; 83.5; 83.7 |
| carnot (small) | RustFig | x_save | -; -; - |
| carnot (small) | RustFig | x_undo | 38.2; 39.1; 37.7 |
| feynman (small) | xfig | open | 1133.4; 1152.3; 1191.7 |
| feynman (small) | xfig | open_read | 35.8; 46.4; 51.0 |
| feynman (small) | xfig | open_draw | 1.5; 2.1; 2.1 |
| feynman (small) | xfig | zoom_in | 26.3; 33.0; 34.0 |
| feynman (small) | xfig | zoom_out | 5.8; 5.6; 4.7 |
| feynman (small) | xfig | pan | 3.3; 4.1; 4.6 |
| feynman (small) | xfig | move_step | 9.1; 8.8; 9.4 |
| feynman (small) | xfig | move_step_max | 11.1; 10.8; 10.6 |
| feynman (small) | xfig | move_place | 53.0; 53.6; 55.2 |
| feynman (small) | xfig | save | 2.5; 2.4; 0.9 |
| feynman (small) | xfig | undo | 4.6; 4.3; 4.5 |
| feynman (small) | xfig | x_open | 1165.9; 1190.7; 1228.4 |
| feynman (small) | xfig | x_zoom_in | 30.9; 32.4; 32.7 |
| feynman (small) | xfig | x_zoom_out | 35.4; 31.3; 32.1 |
| feynman (small) | xfig | x_pan | 9.4; 11.2; 11.4 |
| feynman (small) | xfig | x_move_step | 9.1; 8.8; 9.4 |
| feynman (small) | xfig | x_move_place | 53.0; 53.6; 55.2 |
| feynman (small) | xfig | x_save | 34.1; 34.6; 28.4 |
| feynman (small) | xfig | x_undo | 35.4; 35.5; 30.7 |
| feynman (small) | RustFig | open | 260.3; 229.8; 237.5 |
| feynman (small) | RustFig | open_read | -; -; - |
| feynman (small) | RustFig | open_draw | -; -; - |
| feynman (small) | RustFig | zoom_in | 4.9; 4.6; 4.7 |
| feynman (small) | RustFig | zoom_out | 15.7; 9.6; 15.6 |
| feynman (small) | RustFig | pan | 13.2; 11.6; 7.6 |
| feynman (small) | RustFig | move_step | 0.8; 0.8; 0.8 |
| feynman (small) | RustFig | move_step_max | 1.0; 1.1; 1.2 |
| feynman (small) | RustFig | move_place | 12.7; 13.0; 13.0 |
| feynman (small) | RustFig | save | 11.6; 8.7; 9.7 |
| feynman (small) | RustFig | undo | 14.3; 13.8; 18.8 |
| feynman (small) | RustFig | x_open | 260.3; 229.8; 237.5 |
| feynman (small) | RustFig | x_zoom_in | 16.7; 16.8; 16.7 |
| feynman (small) | RustFig | x_zoom_out | 24.6; 23.6; 37.8 |
| feynman (small) | RustFig | x_pan | 24.6; 23.2; 21.4 |
| feynman (small) | RustFig | x_move_step | 50.1; 50.0; 49.7 |
| feynman (small) | RustFig | x_move_place | 83.7; 82.7; 83.5 |
| feynman (small) | RustFig | x_save | -; -; - |
| feynman (small) | RustFig | x_undo | 30.1; 38.8; 38.3 |
| fourier (small) | xfig | open | 1165.9; 1156.6; 1146.2 |
| fourier (small) | xfig | open_read | 46.0; 43.3; 47.9 |
| fourier (small) | xfig | open_draw | 2.1; 1.4; 1.9 |
| fourier (small) | xfig | zoom_in | 25.7; 26.5; 25.4 |
| fourier (small) | xfig | zoom_out | 5.1; 4.7; 4.5 |
| fourier (small) | xfig | pan | 4.9; 3.4; 4.5 |
| fourier (small) | xfig | move_step | 0.8; 0.7; 0.9 |
| fourier (small) | xfig | move_step_max | 1.3; 1.3; 1.1 |
| fourier (small) | xfig | move_place | 3.8; 3.5; 3.0 |
| fourier (small) | xfig | save | 2.5; 1.8; 1.8 |
| fourier (small) | xfig | undo | 2.8; 2.5; 2.4 |
| fourier (small) | xfig | x_open | 1201.2; 1197.7; 1183.5 |
| fourier (small) | xfig | x_zoom_in | 32.7; 32.7; 32.0 |
| fourier (small) | xfig | x_zoom_out | 34.4; 35.7; 34.3 |
| fourier (small) | xfig | x_pan | 11.0; 10.0; 10.4 |
| fourier (small) | xfig | x_move_step | 9.0; 8.6; 8.9 |
| fourier (small) | xfig | x_move_place | 51.3; 53.2; 53.9 |
| fourier (small) | xfig | x_save | 29.0; 30.3; 34.9 |
| fourier (small) | xfig | x_undo | 33.1; 33.2; 34.7 |
| fourier (small) | RustFig | open | 269.2; 246.7; 235.1 |
| fourier (small) | RustFig | open_read | -; -; - |
| fourier (small) | RustFig | open_draw | -; -; - |
| fourier (small) | RustFig | zoom_in | 3.9; 3.7; 3.8 |
| fourier (small) | RustFig | zoom_out | 8.0; 9.4; 9.3 |
| fourier (small) | RustFig | pan | 10.4; 10.0; 9.7 |
| fourier (small) | RustFig | move_step | 0.9; 0.9; 0.6 |
| fourier (small) | RustFig | move_step_max | 1.0; 1.1; 1.0 |
| fourier (small) | RustFig | move_place | 12.1; 10.4; 9.6 |
| fourier (small) | RustFig | save | 11.5; 10.7; 8.9 |
| fourier (small) | RustFig | undo | 8.3; 10.9; 12.6 |
| fourier (small) | RustFig | x_open | 269.2; 246.7; 235.1 |
| fourier (small) | RustFig | x_zoom_in | 16.7; 16.8; 16.7 |
| fourier (small) | RustFig | x_zoom_out | 22.8; 25.8; 23.8 |
| fourier (small) | RustFig | x_pan | 24.2; 23.9; 22.0 |
| fourier (small) | RustFig | x_move_step | 50.0; 50.1; 50.1 |
| fourier (small) | RustFig | x_move_place | 83.2; 77.6; 82.5 |
| fourier (small) | RustFig | x_save | -; -; - |
| fourier (small) | RustFig | x_undo | 21.9; 27.6; 38.8 |
| pines | xfig | open | 1113.7; 1103.6; 1112.2 |
| pines | xfig | open_read | 2.5; 2.8; 1.2 |
| pines | xfig | open_draw | 3.9; 6.1; 3.6 |
| pines | xfig | zoom_in | 7.8; 5.6; 6.6 |
| pines | xfig | zoom_out | 6.6; 8.9; 8.2 |
| pines | xfig | pan | 4.1; 3.8; 3.7 |
| pines | xfig | move_step | 0.9; 1.0; 1.1 |
| pines | xfig | move_step_max | 1.4; 1.4; 1.5 |
| pines | xfig | move_place | 7.3; 6.6; 8.3 |
| pines | xfig | save | 2.2; 1.8; 1.7 |
| pines | xfig | undo | 5.5; 5.3; 6.0 |
| pines | xfig | x_open | 1154.4; 1146.0; 1148.9 |
| pines | xfig | x_zoom_in | 35.2; 30.1; 30.3 |
| pines | xfig | x_zoom_out | 35.7; 35.5; 34.5 |
| pines | xfig | x_pan | 9.9; 10.4; 10.5 |
| pines | xfig | x_move_step | 8.9; 8.8; 9.3 |
| pines | xfig | x_move_place | 52.6; 53.0; 53.0 |
| pines | xfig | x_save | 34.3; 32.1; 35.1 |
| pines | xfig | x_undo | 34.7; 34.5; 29.4 |
| pines | RustFig | open | 239.1; 194.7; 235.5 |
| pines | RustFig | open_read | -; -; - |
| pines | RustFig | open_draw | -; -; - |
| pines | RustFig | zoom_in | 4.5; 4.6; 7.3 |
| pines | RustFig | zoom_out | 8.4; 7.1; 6.9 |
| pines | RustFig | pan | 6.1; 7.5; 6.8 |
| pines | RustFig | move_step | 0.9; 0.8; 0.8 |
| pines | RustFig | move_step_max | 1.1; 1.0; 1.1 |
| pines | RustFig | move_place | 5.7; 6.9; 6.3 |
| pines | RustFig | save | 11.5; 10.7; 11.3 |
| pines | RustFig | undo | 9.1; 5.6; 8.8 |
| pines | RustFig | x_open | 239.1; 194.7; 235.5 |
| pines | RustFig | x_zoom_in | 20.5; 18.1; 22.9 |
| pines | RustFig | x_zoom_out | 23.0; 23.5; 22.5 |
| pines | RustFig | x_pan | 22.2; 22.3; 23.9 |
| pines | RustFig | x_move_step | 50.2; 50.1; 50.0 |
| pines | RustFig | x_move_place | 74.0; 74.0; 72.4 |
| pines | RustFig | x_save | -; -; - |
| pines | RustFig | x_undo | 27.9; 25.4; 27.0 |
| snowy | xfig | open | 1106.6; 1120.0; 1112.3 |
| snowy | xfig | open_read | 8.2; 5.5; 4.7 |
| snowy | xfig | open_draw | 4.4; 5.5; 4.8 |
| snowy | xfig | zoom_in | 6.3; 8.0; 6.9 |
| snowy | xfig | zoom_out | 8.3; 5.7; 4.1 |
| snowy | xfig | pan | 4.4; 4.4; 4.6 |
| snowy | xfig | move_step | 0.5; 0.4; 0.6 |
| snowy | xfig | move_step_max | 0.8; 0.9; 0.8 |
| snowy | xfig | move_place | 16.9; 11.8; 14.7 |
| snowy | xfig | save | 6.6; 3.8; 5.5 |
| snowy | xfig | undo | 2.2; 3.1; 3.3 |
| snowy | xfig | x_open | 1139.3; 1152.9; 1143.2 |
| snowy | xfig | x_zoom_in | 28.3; 30.5; 30.5 |
| snowy | xfig | x_zoom_out | 29.9; 29.4; 32.5 |
| snowy | xfig | x_pan | 9.2; 8.9; 8.0 |
| snowy | xfig | x_move_step | 8.6; 7.8; 9.2 |
| snowy | xfig | x_move_place | 52.1; 49.3; 52.2 |
| snowy | xfig | x_save | 31.1; 33.1; 34.7 |
| snowy | xfig | x_undo | 33.8; 33.2; 35.2 |
| snowy | RustFig | open | 218.9; 207.1; 213.0 |
| snowy | RustFig | open_read | -; -; - |
| snowy | RustFig | open_draw | -; -; - |
| snowy | RustFig | zoom_in | 1.9; 2.7; 4.1 |
| snowy | RustFig | zoom_out | 2.2; 3.9; 4.5 |
| snowy | RustFig | pan | 3.7; 2.6; 2.2 |
| snowy | RustFig | move_step | 0.8; 0.7; 0.7 |
| snowy | RustFig | move_step_max | 1.0; 0.9; 1.0 |
| snowy | RustFig | move_place | 5.3; 6.2; 6.6 |
| snowy | RustFig | save | 11.5; 13.6; 13.2 |
| snowy | RustFig | undo | 6.1; 9.1; 10.2 |
| snowy | RustFig | x_open | 218.9; 207.1; 213.0 |
| snowy | RustFig | x_zoom_in | 15.3; 16.9; 34.0 |
| snowy | RustFig | x_zoom_out | 18.3; 16.6; 16.7 |
| snowy | RustFig | x_pan | 17.0; 16.6; 16.1 |
| snowy | RustFig | x_move_step | 50.2; 50.1; 50.4 |
| snowy | RustFig | x_move_place | 67.6; 67.1; 67.0 |
| snowy | RustFig | x_save | -; -; - |
| snowy | RustFig | x_undo | 26.9; 29.3; 30.5 |
| wenig | xfig | open | 1115.5; 1130.1; 1123.2 |
| wenig | xfig | open_read | 17.8; 17.8; 18.2 |
| wenig | xfig | open_draw | 12.0; 7.6; 10.9 |
| wenig | xfig | zoom_in | 19.1; 17.0; 17.1 |
| wenig | xfig | zoom_out | 14.9; 14.3; 15.3 |
| wenig | xfig | pan | 18.7; 16.8; 14.8 |
| wenig | xfig | move_step | 1.8; 1.9; 1.5 |
| wenig | xfig | move_step_max | 2.7; 2.4; 2.4 |
| wenig | xfig | move_place | 37.9; 35.7; 37.1 |
| wenig | xfig | save | 12.8; 15.0; 15.6 |
| wenig | xfig | undo | 37.0; 37.2; 35.4 |
| wenig | xfig | x_open | 1153.0; 1167.5; 1157.0 |
| wenig | xfig | x_zoom_in | 28.3; 32.6; 31.9 |
| wenig | xfig | x_zoom_out | 31.6; 28.5; 33.3 |
| wenig | xfig | x_pan | 20.9; 14.7; 13.2 |
| wenig | xfig | x_move_step | 8.7; 9.1; 8.4 |
| wenig | xfig | x_move_place | 49.9; 49.6; 50.1 |
| wenig | xfig | x_save | 32.0; 34.6; 33.3 |
| wenig | xfig | x_undo | 37.1; 39.3; 37.1 |
| wenig | RustFig | open | 208.4; 201.1; 180.8 |
| wenig | RustFig | open_read | -; -; - |
| wenig | RustFig | open_draw | -; -; - |
| wenig | RustFig | zoom_in | 11.6; 9.7; 11.8 |
| wenig | RustFig | zoom_out | 12.0; 12.4; 12.6 |
| wenig | RustFig | pan | 8.1; 7.4; 12.0 |
| wenig | RustFig | move_step | 0.7; 0.7; 0.9 |
| wenig | RustFig | move_step_max | 1.2; 1.1; 1.1 |
| wenig | RustFig | move_place | 14.5; 14.2; 16.5 |
| wenig | RustFig | save | 10.2; 10.2; 9.8 |
| wenig | RustFig | undo | 5.3; 5.5; 5.5 |
| wenig | RustFig | x_open | 208.4; 201.1; 180.8 |
| wenig | RustFig | x_zoom_in | 23.4; 23.5; 21.8 |
| wenig | RustFig | x_zoom_out | 22.5; 22.3; 22.6 |
| wenig | RustFig | x_pan | 23.4; 20.6; 23.5 |
| wenig | RustFig | x_move_step | 50.0; 49.9; 50.1 |
| wenig | RustFig | x_move_place | 106.5; 124.7; 108.5 |
| wenig | RustFig | x_save | -; -; - |
| wenig | RustFig | x_undo | 27.3; 27.0; 25.6 |
| gosper-curve | xfig | open | 1117.7; 1121.2; 1126.4 |
| gosper-curve | xfig | open_read | 11.4; 8.2; 8.5 |
| gosper-curve | xfig | open_draw | 4.7; 3.9; 3.8 |
| gosper-curve | xfig | zoom_in | 8.2; 8.4; 8.2 |
| gosper-curve | xfig | zoom_out | 8.8; 8.0; 8.5 |
| gosper-curve | xfig | pan | 5.6; 5.2; 5.1 |
| gosper-curve | xfig | move_step | 0.8; 0.7; 0.8 |
| gosper-curve | xfig | move_step_max | 1.0; 0.9; 0.8 |
| gosper-curve | xfig | move_place | 20.4; 18.8; 19.5 |
| gosper-curve | xfig | save | 9.1; 9.0; 8.6 |
| gosper-curve | xfig | undo | 5.7; 5.4; 4.6 |
| gosper-curve | xfig | x_open | 1154.3; 1159.5; 1166.8 |
| gosper-curve | xfig | x_zoom_in | 32.9; 35.5; 35.4 |
| gosper-curve | xfig | x_zoom_out | 30.6; 29.4; 36.1 |
| gosper-curve | xfig | x_pan | 11.2; 11.2; 11.2 |
| gosper-curve | xfig | x_move_step | 9.7; 10.0; 9.8 |
| gosper-curve | xfig | x_move_place | 53.5; 52.6; 54.4 |
| gosper-curve | xfig | x_save | 35.0; 36.2; 33.8 |
| gosper-curve | xfig | x_undo | 34.3; 36.4; 32.0 |
| gosper-curve | RustFig | open | 183.1; 178.3; 173.5 |
| gosper-curve | RustFig | open_read | -; -; - |
| gosper-curve | RustFig | open_draw | -; -; - |
| gosper-curve | RustFig | zoom_in | 15.6; 17.4; 14.5 |
| gosper-curve | RustFig | zoom_out | 10.0; 8.2; 7.5 |
| gosper-curve | RustFig | pan | 7.5; 8.3; 7.2 |
| gosper-curve | RustFig | move_step | 1.0; 1.0; 0.9 |
| gosper-curve | RustFig | move_step_max | 1.3; 1.2; 1.2 |
| gosper-curve | RustFig | move_place | 9.1; 7.7; 8.9 |
| gosper-curve | RustFig | save | 14.3; 5.7; 6.4 |
| gosper-curve | RustFig | undo | 2.5; 3.0; 2.6 |
| gosper-curve | RustFig | x_open | 183.1; 178.3; 173.5 |
| gosper-curve | RustFig | x_zoom_in | 32.9; 31.7; 22.1 |
| gosper-curve | RustFig | x_zoom_out | 24.8; 23.5; 23.0 |
| gosper-curve | RustFig | x_pan | 22.1; 22.9; 23.1 |
| gosper-curve | RustFig | x_move_step | 50.2; 50.3; 50.1 |
| gosper-curve | RustFig | x_move_place | 92.8; 87.6; 80.3 |
| gosper-curve | RustFig | x_save | -; -; - |
| gosper-curve | RustFig | x_undo | 18.3; 18.6; 21.4 |
| penrose-tiling | xfig | open | 1136.3; 1131.7; 1131.2 |
| penrose-tiling | xfig | open_read | 16.2; 18.6; 18.5 |
| penrose-tiling | xfig | open_draw | 9.8; 10.8; 10.3 |
| penrose-tiling | xfig | zoom_in | 19.9; 19.4; 22.3 |
| penrose-tiling | xfig | zoom_out | 23.8; 24.0; 22.5 |
| penrose-tiling | xfig | pan | 9.6; 15.4; 6.9 |
| penrose-tiling | xfig | move_step | 0.6; 0.6; 0.6 |
| penrose-tiling | xfig | move_step_max | 0.7; 0.7; 0.7 |
| penrose-tiling | xfig | move_place | 24.8; 23.9; 22.4 |
| penrose-tiling | xfig | save | 21.3; 19.9; 21.5 |
| penrose-tiling | xfig | undo | 6.9; 6.0; 6.9 |
| penrose-tiling | xfig | x_open | 1171.0; 1168.2; 1168.8 |
| penrose-tiling | xfig | x_zoom_in | 31.4; 32.7; 29.5 |
| penrose-tiling | xfig | x_zoom_out | 28.4; 28.8; 31.9 |
| penrose-tiling | xfig | x_pan | 8.6; 14.9; 5.7 |
| penrose-tiling | xfig | x_move_step | 9.8; 9.7; 9.8 |
| penrose-tiling | xfig | x_move_place | 52.2; 53.4; 48.9 |
| penrose-tiling | xfig | x_save | 33.7; 31.2; 33.4 |
| penrose-tiling | xfig | x_undo | 36.0; 32.7; 31.8 |
| penrose-tiling | RustFig | open | 203.4; 187.4; 183.7 |
| penrose-tiling | RustFig | open_read | -; -; - |
| penrose-tiling | RustFig | open_draw | -; -; - |
| penrose-tiling | RustFig | zoom_in | 16.1; 13.5; 14.5 |
| penrose-tiling | RustFig | zoom_out | 17.9; 14.8; 17.3 |
| penrose-tiling | RustFig | pan | 15.5; 13.6; 14.5 |
| penrose-tiling | RustFig | move_step | 1.5; 1.4; 1.4 |
| penrose-tiling | RustFig | move_step_max | 2.1; 2.0; 2.0 |
| penrose-tiling | RustFig | move_place | 18.7; 18.9; 19.3 |
| penrose-tiling | RustFig | save | 18.7; 22.4; 19.1 |
| penrose-tiling | RustFig | undo | 16.3; 18.2; 18.6 |
| penrose-tiling | RustFig | x_open | 203.4; 187.4; 183.7 |
| penrose-tiling | RustFig | x_zoom_in | 27.3; 28.6; 24.7 |
| penrose-tiling | RustFig | x_zoom_out | 31.3; 26.7; 25.3 |
| penrose-tiling | RustFig | x_pan | 31.3; 21.6; 31.0 |
| penrose-tiling | RustFig | x_move_step | 50.3; 50.2; 50.3 |
| penrose-tiling | RustFig | x_move_place | 76.9; 83.5; 76.3 |
| penrose-tiling | RustFig | x_save | -; -; - |
| penrose-tiling | RustFig | x_undo | 42.7; 29.5; 33.3 |
| apollonian-gasket | xfig | open | 1120.9; 1104.1; 1166.3 |
| apollonian-gasket | xfig | open_read | 7.6; 12.6; 11.1 |
| apollonian-gasket | xfig | open_draw | 5.7; 6.7; 6.9 |
| apollonian-gasket | xfig | zoom_in | 9.2; 5.0; 10.4 |
| apollonian-gasket | xfig | zoom_out | 11.7; 11.1; 11.9 |
| apollonian-gasket | xfig | pan | 10.9; 10.7; 9.1 |
| apollonian-gasket | xfig | move_step | 1.4; 1.3; 1.4 |
| apollonian-gasket | xfig | move_step_max | 1.6; 1.5; 1.5 |
| apollonian-gasket | xfig | move_place | 18.0; 19.2; 18.1 |
| apollonian-gasket | xfig | save | 12.5; 12.6; 12.3 |
| apollonian-gasket | xfig | undo | 20.2; 20.4; 20.3 |
| apollonian-gasket | xfig | x_open | 1154.5; 1149.2; 1203.0 |
| apollonian-gasket | xfig | x_zoom_in | 34.2; 30.9; 41.2 |
| apollonian-gasket | xfig | x_zoom_out | 30.3; 30.3; 31.4 |
| apollonian-gasket | xfig | x_pan | 11.1; 10.6; 10.9 |
| apollonian-gasket | xfig | x_move_step | 9.6; 10.1; 10.0 |
| apollonian-gasket | xfig | x_move_place | 54.2; 60.9; 61.4 |
| apollonian-gasket | xfig | x_save | 32.9; 40.7; 41.4 |
| apollonian-gasket | xfig | x_undo | 32.4; 36.6; 38.9 |
| apollonian-gasket | RustFig | open | 213.6; 179.5; 197.9 |
| apollonian-gasket | RustFig | open_read | -; -; - |
| apollonian-gasket | RustFig | open_draw | -; -; - |
| apollonian-gasket | RustFig | zoom_in | 15.2; 15.2; 17.4 |
| apollonian-gasket | RustFig | zoom_out | 17.0; 18.3; 16.4 |
| apollonian-gasket | RustFig | pan | 13.2; 13.8; 12.8 |
| apollonian-gasket | RustFig | move_step | 1.3; 1.2; 1.1 |
| apollonian-gasket | RustFig | move_step_max | 1.8; 1.7; 1.6 |
| apollonian-gasket | RustFig | move_place | 33.1; 30.0; 30.0 |
| apollonian-gasket | RustFig | save | 21.2; 24.1; 19.3 |
| apollonian-gasket | RustFig | undo | 30.0; 34.7; 28.8 |
| apollonian-gasket | RustFig | x_open | 213.6; 179.5; 197.9 |
| apollonian-gasket | RustFig | x_zoom_in | 22.4; 27.1; 32.4 |
| apollonian-gasket | RustFig | x_zoom_out | 28.9; 26.7; 23.4 |
| apollonian-gasket | RustFig | x_pan | 33.3; 24.6; 23.9 |
| apollonian-gasket | RustFig | x_move_step | 50.5; 50.7; 50.4 |
| apollonian-gasket | RustFig | x_move_place | 91.5; 103.5; 94.1 |
| apollonian-gasket | RustFig | x_save | -; -; - |
| apollonian-gasket | RustFig | x_undo | 39.7; 49.9; 40.3 |
| blankmap | xfig | open | 1194.0; 1205.1; 1160.5 |
| blankmap | xfig | open_read | 29.6; 32.2; 32.1 |
| blankmap | xfig | open_draw | 5.0; 3.9; 4.5 |
| blankmap | xfig | zoom_in | 9.8; 13.8; 15.9 |
| blankmap | xfig | zoom_out | 15.4; 14.1; 15.1 |
| blankmap | xfig | pan | 16.2; 17.1; 10.7 |
| blankmap | xfig | move_step | 1.4; 1.4; 1.4 |
| blankmap | xfig | move_step_max | 1.5; 1.5; 1.5 |
| blankmap | xfig | move_place | 52.7; 47.8; 48.9 |
| blankmap | xfig | save | 28.7; 29.2; 28.9 |
| blankmap | xfig | undo | 55.5; 53.5; 52.6 |
| blankmap | xfig | x_open | 1231.8; 1243.2; 1199.5 |
| blankmap | xfig | x_zoom_in | 40.7; 38.0; 35.7 |
| blankmap | xfig | x_zoom_out | 29.5; 38.3; 39.2 |
| blankmap | xfig | x_pan | 16.5; 19.9; 9.6 |
| blankmap | xfig | x_move_step | 10.3; 10.4; 10.3 |
| blankmap | xfig | x_move_place | 55.7; 50.0; 54.9 |
| blankmap | xfig | x_save | 30.4; 37.1; 35.4 |
| blankmap | xfig | x_undo | 55.7; 54.1; 55.3 |
| blankmap | RustFig | open | 194.0; 182.4; 175.3 |
| blankmap | RustFig | open_read | -; -; - |
| blankmap | RustFig | open_draw | -; -; - |
| blankmap | RustFig | zoom_in | 10.8; 9.8; 12.2 |
| blankmap | RustFig | zoom_out | 13.7; 12.2; 14.7 |
| blankmap | RustFig | pan | 8.8; 9.4; 9.4 |
| blankmap | RustFig | move_step | 1.0; 1.0; 1.0 |
| blankmap | RustFig | move_step_max | 1.5; 1.4; 1.4 |
| blankmap | RustFig | move_place | 21.3; 17.1; 19.2 |
| blankmap | RustFig | save | 21.9; 21.0; 21.7 |
| blankmap | RustFig | undo | 18.4; 19.7; 20.7 |
| blankmap | RustFig | x_open | 194.0; 182.4; 175.3 |
| blankmap | RustFig | x_zoom_in | 24.5; 23.7; 26.8 |
| blankmap | RustFig | x_zoom_out | 22.1; 23.5; 23.8 |
| blankmap | RustFig | x_pan | 23.8; 24.0; 24.2 |
| blankmap | RustFig | x_move_step | 50.2; 51.1; 50.3 |
| blankmap | RustFig | x_move_place | 82.9; 75.7; 83.1 |
| blankmap | RustFig | x_save | -; -; - |
| blankmap | RustFig | x_undo | 31.8; 31.3; 30.6 |
| ne-coastline-10m | xfig | open | 1240.3; 1215.2; 1230.2 |
| ne-coastline-10m | xfig | open_read | 73.9; 61.7; 74.2 |
| ne-coastline-10m | xfig | open_draw | 3.0; 2.7; 2.9 |
| ne-coastline-10m | xfig | zoom_in | 8.4; 7.2; 7.6 |
| ne-coastline-10m | xfig | zoom_out | 12.0; 7.4; 12.4 |
| ne-coastline-10m | xfig | pan | 10.5; 6.9; 12.7 |
| ne-coastline-10m | xfig | move_step | 1.7; 1.6; 1.7 |
| ne-coastline-10m | xfig | move_step_max | 1.7; 1.9; 1.7 |
| ne-coastline-10m | xfig | move_place | 128.9; 132.8; 129.0 |
| ne-coastline-10m | xfig | save | 66.3; 66.0; 60.1 |
| ne-coastline-10m | xfig | undo | 36.5; 38.9; 32.7 |
| ne-coastline-10m | xfig | x_open | 1276.1; 1246.2; 1266.4 |
| ne-coastline-10m | xfig | x_zoom_in | 38.7; 28.9; 41.0 |
| ne-coastline-10m | xfig | x_zoom_out | 33.1; 38.6; 35.9 |
| ne-coastline-10m | xfig | x_pan | 10.8; 8.0; 10.9 |
| ne-coastline-10m | xfig | x_move_step | 10.6; 10.3; 10.5 |
| ne-coastline-10m | xfig | x_move_place | 129.6; 133.3; 129.2 |
| ne-coastline-10m | xfig | x_save | 68.6; 68.2; 63.2 |
| ne-coastline-10m | xfig | x_undo | 39.8; 39.7; 34.9 |
| ne-coastline-10m | RustFig | open | 252.6; 234.8; 231.4 |
| ne-coastline-10m | RustFig | open_read | -; -; - |
| ne-coastline-10m | RustFig | open_draw | -; -; - |
| ne-coastline-10m | RustFig | zoom_in | 15.7; 16.5; 16.8 |
| ne-coastline-10m | RustFig | zoom_out | 17.8; 14.2; 16.8 |
| ne-coastline-10m | RustFig | pan | 12.1; 15.5; 12.8 |
| ne-coastline-10m | RustFig | move_step | 0.8; 1.0; 1.1 |
| ne-coastline-10m | RustFig | move_step_max | 1.6; 1.6; 1.6 |
| ne-coastline-10m | RustFig | move_place | 29.7; 28.4; 28.9 |
| ne-coastline-10m | RustFig | save | 34.7; 35.1; 37.8 |
| ne-coastline-10m | RustFig | undo | 31.6; 34.8; 35.0 |
| ne-coastline-10m | RustFig | x_open | 252.6; 234.8; 231.4 |
| ne-coastline-10m | RustFig | x_zoom_in | 31.1; 25.0; 31.2 |
| ne-coastline-10m | RustFig | x_zoom_out | 29.4; 27.7; 29.6 |
| ne-coastline-10m | RustFig | x_pan | 23.2; 32.4; 22.8 |
| ne-coastline-10m | RustFig | x_move_step | 50.2; 50.5; 50.3 |
| ne-coastline-10m | RustFig | x_move_place | 94.3; 91.8; 82.2 |
| ne-coastline-10m | RustFig | x_save | -; -; - |
| ne-coastline-10m | RustFig | x_undo | 50.0; 53.1; 54.3 |
| ne-countries-10m | xfig | open | 1198.3; 1197.7; 1193.8 |
| ne-countries-10m | xfig | open_read | 86.4; 91.5; 86.1 |
| ne-countries-10m | xfig | open_draw | 5.1; 5.2; 5.1 |
| ne-countries-10m | xfig | zoom_in | 19.3; 18.6; 19.3 |
| ne-countries-10m | xfig | zoom_out | 19.8; 19.2; 19.9 |
| ne-countries-10m | xfig | pan | 19.8; 20.4; 19.1 |
| ne-countries-10m | xfig | move_step | 1.1; 1.1; 0.9 |
| ne-countries-10m | xfig | move_step_max | 1.2; 1.2; 1.6 |
| ne-countries-10m | xfig | move_place | 167.0; 163.6; 161.7 |
| ne-countries-10m | xfig | save | 74.2; 72.8; 73.3 |
| ne-countries-10m | xfig | undo | 43.7; 48.5; 47.1 |
| ne-countries-10m | xfig | x_open | 1232.4; 1233.4; 1233.7 |
| ne-countries-10m | xfig | x_zoom_in | 33.2; 28.5; 30.1 |
| ne-countries-10m | xfig | x_zoom_out | 31.5; 28.0; 29.6 |
| ne-countries-10m | xfig | x_pan | 19.1; 19.6; 19.7 |
| ne-countries-10m | xfig | x_move_step | 9.9; 9.9; 9.3 |
| ne-countries-10m | xfig | x_move_place | 168.1; 164.1; 162.4 |
| ne-countries-10m | xfig | x_save | 76.0; 75.6; 75.8 |
| ne-countries-10m | xfig | x_undo | 44.8; 49.6; 49.1 |
| ne-countries-10m | RustFig | open | 285.1; 240.5; 240.1 |
| ne-countries-10m | RustFig | open_read | -; -; - |
| ne-countries-10m | RustFig | open_draw | -; -; - |
| ne-countries-10m | RustFig | zoom_in | 16.2; 18.3; 18.9 |
| ne-countries-10m | RustFig | zoom_out | 18.4; 19.0; 20.6 |
| ne-countries-10m | RustFig | pan | 16.4; 16.5; 13.8 |
| ne-countries-10m | RustFig | move_step | 0.9; 0.9; 0.9 |
| ne-countries-10m | RustFig | move_step_max | 1.9; 1.6; 1.6 |
| ne-countries-10m | RustFig | move_place | 40.3; 35.5; 36.6 |
| ne-countries-10m | RustFig | save | 39.4; 39.9; 42.8 |
| ne-countries-10m | RustFig | undo | 40.7; 33.1; 43.9 |
| ne-countries-10m | RustFig | x_open | 285.1; 240.5; 240.1 |
| ne-countries-10m | RustFig | x_zoom_in | 21.9; 31.5; 23.9 |
| ne-countries-10m | RustFig | x_zoom_out | 22.6; 31.4; 28.1 |
| ne-countries-10m | RustFig | x_pan | 28.6; 28.2; 25.5 |
| ne-countries-10m | RustFig | x_move_step | 50.5; 50.2; 50.6 |
| ne-countries-10m | RustFig | x_move_place | 109.6; 98.9; 98.7 |
| ne-countries-10m | RustFig | x_save | -; -; - |
| ne-countries-10m | RustFig | x_undo | 55.0; 47.5; 53.3 |
| greece | xfig | open | 1193.1; 1199.3; 1193.2 |
| greece | xfig | open_read | 83.6; 89.8; 83.2 |
| greece | xfig | open_draw | 4.6; 4.7; 4.7 |
| greece | xfig | zoom_in | 13.2; 13.8; 14.6 |
| greece | xfig | zoom_out | 16.4; 15.2; 14.4 |
| greece | xfig | pan | 11.5; 12.0; 11.8 |
| greece | xfig | move_step | 0.6; 0.6; 0.6 |
| greece | xfig | move_step_max | 0.7; 0.7; 1.1 |
| greece | xfig | move_place | 104.7; 105.2; 100.5 |
| greece | xfig | save | 48.4; 51.0; 53.9 |
| greece | xfig | undo | 26.8; 29.7; 28.8 |
| greece | xfig | x_open | 1233.3; 1235.8; 1230.9 |
| greece | xfig | x_zoom_in | 33.8; 29.6; 33.2 |
| greece | xfig | x_zoom_out | 31.5; 32.1; 29.9 |
| greece | xfig | x_pan | 10.6; 11.2; 10.8 |
| greece | xfig | x_move_step | 10.0; 9.9; 9.7 |
| greece | xfig | x_move_place | 105.9; 106.2; 102.2 |
| greece | xfig | x_save | 50.6; 54.1; 55.6 |
| greece | xfig | x_undo | 29.2; 30.0; 30.2 |
| greece | RustFig | open | 284.0; 288.6; 284.5 |
| greece | RustFig | open_read | -; -; - |
| greece | RustFig | open_draw | -; -; - |
| greece | RustFig | zoom_in | 24.0; 26.7; 24.2 |
| greece | RustFig | zoom_out | 1.5; 27.7; 1.7 |
| greece | RustFig | pan | 24.4; 26.7; 1.5 |
| greece | RustFig | move_step | 0.7; 0.9; 1.0 |
| greece | RustFig | move_step_max | 1.4; 1.4; 1.4 |
| greece | RustFig | move_place | 33.4; 34.3; 37.7 |
| greece | RustFig | save | 32.5; 29.7; 32.5 |
| greece | RustFig | undo | 36.9; 35.0; 34.6 |
| greece | RustFig | x_open | 284.0; 288.6; 284.5 |
| greece | RustFig | x_zoom_in | 35.2; 37.3; 37.2 |
| greece | RustFig | x_zoom_out | 48.8; 53.1; 49.5 |
| greece | RustFig | x_pan | 46.7; 46.8; 37.4 |
| greece | RustFig | x_move_step | 50.4; 50.5; 50.5 |
| greece | RustFig | x_move_place | 99.7; 99.5; 98.8 |
| greece | RustFig | x_save | -; -; - |
| greece | RustFig | x_undo | 57.2; 56.5; 54.5 |
| clifford-attractor | xfig | open | 1258.3; 1271.9; 1256.6 |
| clifford-attractor | xfig | open_read | 132.8; 128.5; 121.5 |
| clifford-attractor | xfig | open_draw | 32.9; 32.0; 32.4 |
| clifford-attractor | xfig | zoom_in | 63.6; 59.5; 60.4 |
| clifford-attractor | xfig | zoom_out | 62.2; 60.2; 64.2 |
| clifford-attractor | xfig | pan | 44.8; 43.3; 44.4 |
| clifford-attractor | xfig | move_step | 0.6; 0.6; 0.6 |
| clifford-attractor | xfig | move_step_max | 0.6; 0.7; 0.7 |
| clifford-attractor | xfig | move_place | 106.3; 105.3; 102.8 |
| clifford-attractor | xfig | save | 131.5; 135.7; 138.2 |
| clifford-attractor | xfig | undo | 36.5; 37.2; 34.6 |
| clifford-attractor | xfig | x_open | 1293.1; 1308.7; 1296.9 |
| clifford-attractor | xfig | x_zoom_in | 65.8; 59.7; 62.8 |
| clifford-attractor | xfig | x_zoom_out | 64.0; 61.1; 64.6 |
| clifford-attractor | xfig | x_pan | 46.1; 42.9; 46.4 |
| clifford-attractor | xfig | x_move_step | 9.9; 9.9; 10.1 |
| clifford-attractor | xfig | x_move_place | 109.0; 106.5; 103.1 |
| clifford-attractor | xfig | x_save | 132.6; 138.0; 141.0 |
| clifford-attractor | xfig | x_undo | 39.4; 39.2; 36.7 |
| clifford-attractor | RustFig | open | 483.4; 471.4; 488.2 |
| clifford-attractor | RustFig | open_read | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":502}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":506}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":508}) |
| clifford-attractor | RustFig | open_draw | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":502}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":506}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":508}) |
| clifford-attractor | RustFig | zoom_in | 53.9; 51.6; 57.3 |
| clifford-attractor | RustFig | zoom_out | 55.8; 56.7; 57.5 |
| clifford-attractor | RustFig | pan | 54.4; 60.2; 58.1 |
| clifford-attractor | RustFig | move_step | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":502}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":506}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":508}) |
| clifford-attractor | RustFig | move_step_max | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":502}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":506}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":508}) |
| clifford-attractor | RustFig | move_place | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":502}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":506}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":508}) |
| clifford-attractor | RustFig | save | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":502}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":506}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":508}) |
| clifford-attractor | RustFig | undo | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":502}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":506}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":508}) |
| clifford-attractor | RustFig | x_open | 483.4; 471.4; 488.2 |
| clifford-attractor | RustFig | x_zoom_in | 54.0; 54.3; 54.2 |
| clifford-attractor | RustFig | x_zoom_out | 66.8; 54.2; 54.1 |
| clifford-attractor | RustFig | x_pan | 54.1; 54.2; 54.1 |
| clifford-attractor | RustFig | x_move_step | -; -; - |
| clifford-attractor | RustFig | x_move_place | -; -; - |
| clifford-attractor | RustFig | x_save | -; -; - |
| clifford-attractor | RustFig | x_undo | -; -; - |
| cube8 | xfig | open | 1324.1; 1302.6; 1279.8 |
| cube8 | xfig | open_read | 121.4; 112.6; 118.7 |
| cube8 | xfig | open_draw | 45.3; 46.4; 44.4 |
| cube8 | xfig | zoom_in | 64.5; 62.8; 65.4 |
| cube8 | xfig | zoom_out | 83.8; 75.5; 82.5 |
| cube8 | xfig | pan | 68.6; 69.4; 70.3 |
| cube8 | xfig | move_step | 0.6; 0.6; 0.6 |
| cube8 | xfig | move_step_max | 0.6; 0.7; 0.7 |
| cube8 | xfig | move_place | 140.6; 147.5; 134.8 |
| cube8 | xfig | save | 115.5; 124.2; 113.9 |
| cube8 | xfig | undo | 38.4; 44.0; 38.4 |
| cube8 | xfig | x_open | 1362.6; 1341.9; 1318.1 |
| cube8 | xfig | x_zoom_in | 65.0; 65.4; 67.0 |
| cube8 | xfig | x_zoom_out | 84.6; 77.2; 82.7 |
| cube8 | xfig | x_pan | 70.8; 71.5; 71.7 |
| cube8 | xfig | x_move_step | 10.3; 10.2; 10.0 |
| cube8 | xfig | x_move_place | 143.4; 147.5; 137.0 |
| cube8 | xfig | x_save | 116.7; 127.0; 116.8 |
| cube8 | xfig | x_undo | 41.7; 44.0; 39.9 |
| cube8 | RustFig | open | 1772.1; 1787.0; 1767.6 |
| cube8 | RustFig | open_read | -; -; - |
| cube8 | RustFig | open_draw | -; -; - |
| cube8 | RustFig | zoom_in | 230.4; 222.8; 232.7 |
| cube8 | RustFig | zoom_out | 293.1; 279.1; 289.8 |
| cube8 | RustFig | pan | 164.6; 162.8; 162.7 |
| cube8 | RustFig | move_step | 1.3; 3.9; 1.1 |
| cube8 | RustFig | move_step_max | 8.7; 17.8; 12.5 |
| cube8 | RustFig | move_place | 513.3; 523.0; 516.9 |
| cube8 | RustFig | save | 132.4; 137.5; 135.9 |
| cube8 | RustFig | undo | 699.2; 618.7; 628.8 |
| cube8 | RustFig | x_open | 1772.1; 1787.0; 1767.6 |
| cube8 | RustFig | x_zoom_in | 232.5; 215.8; 232.7 |
| cube8 | RustFig | x_zoom_out | 270.3; 283.3; 282.1 |
| cube8 | RustFig | x_pan | 166.6; 166.3; 166.1 |
| cube8 | RustFig | x_move_step | 50.1; 50.6; 50.7 |
| cube8 | RustFig | x_move_place | 582.4; 591.9; 592.5 |
| cube8 | RustFig | x_save | -; -; - |
| cube8 | RustFig | x_undo | 599.1; 593.8; 569.8 |

</details>

Move test: 30 steps over 240 x 120 pixels. Figures with more than 300 objects (snowy, penrose-tiling, apollonian-gasket, blankmap, ne-coastline-10m, ne-countries-10m, greece, clifford-attractor, cube8) were glued into one compound first (xfig: Glue, middle-button region, right button; RustFig: select all, group), smaller ones move a single object.

<!-- results:end -->

## Results with RustFig at Retina 2x (2256x1372 pixels, for information)

<!-- results-retina:start -->

#### Setup of this run

- Machine: Apple M4, macOS 27.2
- xfig: `/Users/fabian/rustfig-speed/xfig-timing/src/src/xfig` (3.2.9a), default options, scratch `HOME`, on `Xvfb :78` 1400x1000x24; window `1322x818` (canvas 1128x686 pixels)
- RustFig: `target/release/rustfig` (rustfig 0.9.0-beta.7), release build, label: main e2c3c50 (Retina 2x, for information); `-geometry 1398x803`
- Each value is the median of 3 run(s), a new process per run; timeout 120 s per measurement ("> N s": not finished in that time; the real time is longer)
- Generated 2026-11-02 19:48 by `cargo xtask speed-compare`

- RustFig canvas as reported by `ui.state` (points): 1128x686; density native (the display's) (xfig's canvas: 1128x686 pixels)
#### Results

xfig / RustFig is the speed-up of RustFig (above 1 = RustFig is faster).

##### Open (xfig: main() to figure drawn; RustFig: process start to drawn)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.15 s | 186 ms | 6.2x |
| feynman (small) | 4 kB, 34 obj. | 1.14 s | 185 ms | 6.1x |
| fourier (small) | 16 kB, 41 obj. | 1.19 s | 190 ms | 6.3x |
| pines | 11 kB, 31 obj. | 1.17 s | 181 ms | 6.5x |
| snowy | 173 kB, 436 obj. | 1.16 s | 182 ms | 6.4x |
| wenig | 587 kB, 85 obj. | 1.18 s | 163 ms | 7.3x |
| gosper-curve | 182 kB, 142 obj. | 1.18 s | 163 ms | 7.2x |
| penrose-tiling | 560 kB, 7615 obj. | 1.20 s | 171 ms | 7.0x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.18 s | 195 ms | 6.1x |
| blankmap | 1.0 MB, 2232 obj. | 1.20 s | 164 ms | 7.3x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.26 s | 189 ms | 6.7x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.27 s | 197 ms | 6.4x |
| greece | 3.2 MB, 2013 obj. | 1.26 s | 258 ms | 4.9x |
| clifford-attractor | 13.2 MB, 250001 obj. | 1.35 s | 394 ms | 3.4x |
| cube8 | 14.6 MB, 190260 obj. | 1.34 s | 1.36 s | 1.0x |

##### Open, part 1: reading the file (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 41.7 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 42.8 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 41.7 ms | n/a | - |
| pines | 11 kB, 31 obj. | 2.7 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 11.1 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 22.6 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 11.8 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 20.8 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 12.4 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 31.8 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 79.3 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 94.3 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 92.2 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 132 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 126 ms | n/a | - |

##### Open, part 2: first full canvas redraw (xfig only)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 1.1 ms | n/a | - |
| feynman (small) | 4 kB, 34 obj. | 1.0 ms | n/a | - |
| fourier (small) | 16 kB, 41 obj. | 0.9 ms | n/a | - |
| pines | 11 kB, 31 obj. | 6.9 ms | n/a | - |
| snowy | 173 kB, 436 obj. | 5.3 ms | n/a | - |
| wenig | 587 kB, 85 obj. | 10.3 ms | n/a | - |
| gosper-curve | 182 kB, 142 obj. | 4.4 ms | n/a | - |
| penrose-tiling | 560 kB, 7615 obj. | 13.6 ms | n/a | - |
| apollonian-gasket | 330 kB, 4221 obj. | 6.4 ms | n/a | - |
| blankmap | 1.0 MB, 2232 obj. | 3.6 ms | n/a | - |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 2.8 ms | n/a | - |
| ne-countries-10m | 6.0 MB, 4233 obj. | 5.8 ms | n/a | - |
| greece | 3.2 MB, 2013 obj. | 5.4 ms | n/a | - |
| clifford-attractor | 13.2 MB, 250001 obj. | 35.0 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 46.3 ms | n/a | - |

##### Zoom in (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 27.7 ms | 8.4 ms | 3.3x |
| feynman (small) | 4 kB, 34 obj. | 31.8 ms | 7.2 ms | 4.4x |
| fourier (small) | 16 kB, 41 obj. | 28.7 ms | 6.3 ms | 4.5x |
| pines | 11 kB, 31 obj. | 9.2 ms | 16.5 ms | 0.6x |
| snowy | 173 kB, 436 obj. | 7.8 ms | 8.9 ms | 0.9x |
| wenig | 587 kB, 85 obj. | 18.8 ms | 5.7 ms | 3.3x |
| gosper-curve | 182 kB, 142 obj. | 8.4 ms | 8.9 ms | 0.9x |
| penrose-tiling | 560 kB, 7615 obj. | 20.2 ms | 11.3 ms | 1.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 11.6 ms | 9.7 ms | 1.2x |
| blankmap | 1.0 MB, 2232 obj. | 12.2 ms | 6.6 ms | 1.8x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 11.2 ms | 14.4 ms | 0.8x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 15.8 ms | 18.1 ms | 0.9x |
| greece | 3.2 MB, 2013 obj. | 14.6 ms | 29.7 ms | 0.5x |
| clifford-attractor | 13.2 MB, 250001 obj. | 69.4 ms | 105 ms | 0.7x |
| cube8 | 14.6 MB, 190260 obj. | 70.2 ms | 618 ms | 0.1x |

##### Zoom out (one step)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 7.8 ms | 10.6 ms | 0.7x |
| feynman (small) | 4 kB, 34 obj. | 6.9 ms | 25.7 ms | 0.3x |
| fourier (small) | 16 kB, 41 obj. | 6.9 ms | 15.5 ms | 0.4x |
| pines | 11 kB, 31 obj. | 9.2 ms | 20.5 ms | 0.4x |
| snowy | 173 kB, 436 obj. | 8.3 ms | 9.6 ms | 0.9x |
| wenig | 587 kB, 85 obj. | 17.1 ms | 5.7 ms | 3.0x |
| gosper-curve | 182 kB, 142 obj. | 8.8 ms | 7.9 ms | 1.1x |
| penrose-tiling | 560 kB, 7615 obj. | 23.4 ms | 10.7 ms | 2.2x |
| apollonian-gasket | 330 kB, 4221 obj. | 11.7 ms | 7.0 ms | 1.7x |
| blankmap | 1.0 MB, 2232 obj. | 15.3 ms | 5.1 ms | 3.0x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 12.2 ms | 8.0 ms | 1.5x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 19.0 ms | 11.5 ms | 1.7x |
| greece | 3.2 MB, 2013 obj. | 16.2 ms | 25.8 ms | 0.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 71.8 ms | 60.0 ms | 1.2x |
| cube8 | 14.6 MB, 190260 obj. | 85.5 ms | 432 ms | 0.2x |

##### Pan half a screen

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 4.9 ms | 23.2 ms | 0.2x |
| feynman (small) | 4 kB, 34 obj. | 4.4 ms | 25.0 ms | 0.2x |
| fourier (small) | 16 kB, 41 obj. | 3.5 ms | 15.8 ms | 0.2x |
| pines | 11 kB, 31 obj. | 4.6 ms | 20.2 ms | 0.2x |
| snowy | 173 kB, 436 obj. | 6.8 ms | 8.3 ms | 0.8x |
| wenig | 587 kB, 85 obj. | 18.1 ms | 4.8 ms | 3.8x |
| gosper-curve | 182 kB, 142 obj. | 5.0 ms | 7.4 ms | 0.7x |
| penrose-tiling | 560 kB, 7615 obj. | 14.6 ms | 8.2 ms | 1.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 10.7 ms | 5.3 ms | 2.0x |
| blankmap | 1.0 MB, 2232 obj. | 16.6 ms | 3.6 ms | 4.7x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 13.4 ms | 4.8 ms | 2.8x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 21.6 ms | 7.3 ms | 3.0x |
| greece | 3.2 MB, 2013 obj. | 13.4 ms | 23.1 ms | 0.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 42.9 ms | 65.6 ms | 0.7x |
| cube8 | 14.6 MB, 190260 obj. | 75.4 ms | 223 ms | 0.3x |

##### Move: one step of the drag (median of 30)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 0.9 ms | 1.0 ms | 1.0x |
| feynman (small) | 4 kB, 34 obj. | 9.9 ms | 1.1 ms | 9.1x |
| fourier (small) | 16 kB, 41 obj. | 0.9 ms | 0.9 ms | 1.0x |
| pines | 11 kB, 31 obj. | 1.1 ms | 1.0 ms | 1.1x |
| snowy | 173 kB, 436 obj. | 0.6 ms | 1.1 ms | 0.6x |
| wenig | 587 kB, 85 obj. | 2.3 ms | 0.1 ms | 21x |
| gosper-curve | 182 kB, 142 obj. | 0.7 ms | 0.1 ms | 7.0x |
| penrose-tiling | 560 kB, 7615 obj. | 0.6 ms | 0.2 ms | 4.0x |
| apollonian-gasket | 330 kB, 4221 obj. | 1.3 ms | 0.2 ms | 7.0x |
| blankmap | 1.0 MB, 2232 obj. | 1.4 ms | 0.1 ms | 13x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 1.7 ms | 0.1 ms | 14x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 1.0 ms | 0.1 ms | 8.5x |
| greece | 3.2 MB, 2013 obj. | 0.6 ms | 0.1 ms | 5.3x |
| clifford-attractor | 13.2 MB, 250001 obj. | 0.5 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 0.5 ms | 2.0 ms | 0.3x |

##### Move: place (final redraw)

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 5.2 ms | 1.8 ms | 2.9x |
| feynman (small) | 4 kB, 34 obj. | 55.5 ms | 1.7 ms | 32x |
| fourier (small) | 16 kB, 41 obj. | 4.0 ms | 17.3 ms | 0.2x |
| pines | 11 kB, 31 obj. | 8.0 ms | 17.3 ms | 0.5x |
| snowy | 173 kB, 436 obj. | 17.9 ms | 17.3 ms | 1.0x |
| wenig | 587 kB, 85 obj. | 38.1 ms | 8.2 ms | 4.6x |
| gosper-curve | 182 kB, 142 obj. | 11.9 ms | 8.6 ms | 1.4x |
| penrose-tiling | 560 kB, 7615 obj. | 24.5 ms | 9.5 ms | 2.6x |
| apollonian-gasket | 330 kB, 4221 obj. | 20.7 ms | 12.6 ms | 1.6x |
| blankmap | 1.0 MB, 2232 obj. | 54.4 ms | 7.9 ms | 6.9x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 134 ms | 12.5 ms | 11x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 166 ms | 22.5 ms | 7.4x |
| greece | 3.2 MB, 2013 obj. | 103 ms | 22.5 ms | 4.6x |
| clifford-attractor | 13.2 MB, 250001 obj. | 105 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 144 ms | 559 ms | 0.3x |

##### Save

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 2.4 ms | 8.7 ms | 0.3x |
| feynman (small) | 4 kB, 34 obj. | 2.5 ms | 9.2 ms | 0.3x |
| fourier (small) | 16 kB, 41 obj. | 3.0 ms | 10.9 ms | 0.3x |
| pines | 11 kB, 31 obj. | 2.8 ms | 9.3 ms | 0.3x |
| snowy | 173 kB, 436 obj. | 8.2 ms | 15.8 ms | 0.5x |
| wenig | 587 kB, 85 obj. | 17.7 ms | 9.9 ms | 1.8x |
| gosper-curve | 182 kB, 142 obj. | 8.8 ms | 4.5 ms | 1.9x |
| penrose-tiling | 560 kB, 7615 obj. | 19.9 ms | 3.4 ms | 5.8x |
| apollonian-gasket | 330 kB, 4221 obj. | 11.8 ms | 3.6 ms | 3.2x |
| blankmap | 1.0 MB, 2232 obj. | 30.2 ms | 4.0 ms | 7.6x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 68.0 ms | 11.9 ms | 5.7x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 76.1 ms | 13.7 ms | 5.5x |
| greece | 3.2 MB, 2013 obj. | 47.6 ms | 8.9 ms | 5.3x |
| clifford-attractor | 13.2 MB, 250001 obj. | 139 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 120 ms | 119 ms | 1.0x |

##### Undo the move

| Figure | Size | xfig | RustFig | xfig / RustFig |
|---|---|---|---|---|
| carnot (small) | 12 kB, 64 obj. | 4.1 ms | 34.6 ms | 0.1x |
| feynman (small) | 4 kB, 34 obj. | 6.0 ms | 24.8 ms | 0.2x |
| fourier (small) | 16 kB, 41 obj. | 3.0 ms | 17.9 ms | 0.2x |
| pines | 11 kB, 31 obj. | 7.7 ms | 20.3 ms | 0.4x |
| snowy | 173 kB, 436 obj. | 4.9 ms | 16.0 ms | 0.3x |
| wenig | 587 kB, 85 obj. | 40.4 ms | 7.7 ms | 5.2x |
| gosper-curve | 182 kB, 142 obj. | 3.1 ms | 7.9 ms | 0.4x |
| penrose-tiling | 560 kB, 7615 obj. | 6.5 ms | 8.8 ms | 0.7x |
| apollonian-gasket | 330 kB, 4221 obj. | 24.7 ms | 13.0 ms | 1.9x |
| blankmap | 1.0 MB, 2232 obj. | 54.9 ms | 8.3 ms | 6.6x |
| ne-coastline-10m | 4.6 MB, 4069 obj. | 39.0 ms | 16.5 ms | 2.4x |
| ne-countries-10m | 6.0 MB, 4233 obj. | 47.7 ms | 26.3 ms | 1.8x |
| greece | 3.2 MB, 2013 obj. | 30.2 ms | 25.7 ms | 1.2x |
| clifford-attractor | 13.2 MB, 250001 obj. | 35.4 ms | n/a | - |
| cube8 | 14.6 MB, 190260 obj. | 40.1 ms | 766 ms | 0.1x |

##### Cross-check: the same actions timed from outside

xfig: its own hooks against watching the canvas pixels; RustFig: its frame times against the wall time until `ui.state` reports the canvas current (never below one frame, 16.7 ms).

| Figure | Task | xfig, own hooks | xfig, pixels | RustFig, frame times | RustFig, wall to idle |
|---|---|---|---|---|---|
| carnot (small) | open | 1.15 s | 1.18 s | 186 ms | 186 ms |
| carnot (small) | zoom_in | 27.7 ms | 30.0 ms | 8.4 ms | 22.8 ms |
| carnot (small) | zoom_out | 7.8 ms | 35.1 ms | 10.6 ms | 33.4 ms |
| carnot (small) | pan | 4.9 ms | 10.9 ms | 23.2 ms | 36.2 ms |
| carnot (small) | move_step | 0.9 ms | 9.8 ms | 1.0 ms | 50.1 ms |
| carnot (small) | move_place | 5.2 ms | 55.3 ms | 1.8 ms | 82.1 ms |
| carnot (small) | save | 2.4 ms | 32.2 ms | 8.7 ms | n/a |
| carnot (small) | undo | 4.1 ms | 35.3 ms | 34.6 ms | 48.0 ms |
| feynman (small) | open | 1.14 s | 1.18 s | 185 ms | 185 ms |
| feynman (small) | zoom_in | 31.8 ms | 32.7 ms | 7.2 ms | 21.6 ms |
| feynman (small) | zoom_out | 6.9 ms | 34.9 ms | 25.7 ms | 37.5 ms |
| feynman (small) | pan | 4.4 ms | 10.6 ms | 25.0 ms | 42.9 ms |
| feynman (small) | move_step | 9.9 ms | 9.9 ms | 1.1 ms | 50.3 ms |
| feynman (small) | move_place | 55.5 ms | 55.5 ms | 1.7 ms | 74.3 ms |
| feynman (small) | save | 2.5 ms | 34.0 ms | 9.2 ms | n/a |
| feynman (small) | undo | 6.0 ms | 34.3 ms | 24.8 ms | 42.2 ms |
| fourier (small) | open | 1.19 s | 1.23 s | 190 ms | 190 ms |
| fourier (small) | zoom_in | 28.7 ms | 35.3 ms | 6.3 ms | 16.7 ms |
| fourier (small) | zoom_out | 6.9 ms | 37.4 ms | 15.5 ms | 24.3 ms |
| fourier (small) | pan | 3.5 ms | 8.9 ms | 15.8 ms | 25.8 ms |
| fourier (small) | move_step | 0.9 ms | 10.4 ms | 0.9 ms | 50.3 ms |
| fourier (small) | move_place | 4.0 ms | 61.2 ms | 17.3 ms | 82.8 ms |
| fourier (small) | save | 3.0 ms | 37.0 ms | 10.9 ms | n/a |
| fourier (small) | undo | 3.0 ms | 39.4 ms | 17.9 ms | 33.0 ms |
| pines | open | 1.17 s | 1.21 s | 181 ms | 181 ms |
| pines | zoom_in | 9.2 ms | 41.1 ms | 16.5 ms | 35.3 ms |
| pines | zoom_out | 9.2 ms | 40.4 ms | 20.5 ms | 35.0 ms |
| pines | pan | 4.6 ms | 11.1 ms | 20.2 ms | 37.4 ms |
| pines | move_step | 1.1 ms | 10.1 ms | 1.0 ms | 50.2 ms |
| pines | move_place | 8.0 ms | 57.9 ms | 17.3 ms | 87.0 ms |
| pines | save | 2.8 ms | 32.2 ms | 9.3 ms | n/a |
| pines | undo | 7.7 ms | 36.2 ms | 20.3 ms | 40.4 ms |
| snowy | open | 1.16 s | 1.20 s | 182 ms | 182 ms |
| snowy | zoom_in | 7.8 ms | 37.1 ms | 8.9 ms | 27.7 ms |
| snowy | zoom_out | 8.3 ms | 33.0 ms | 9.6 ms | 26.8 ms |
| snowy | pan | 6.8 ms | 11.0 ms | 8.3 ms | 26.1 ms |
| snowy | move_step | 0.6 ms | 10.4 ms | 1.1 ms | 50.2 ms |
| snowy | move_place | 17.9 ms | 59.7 ms | 17.3 ms | 83.0 ms |
| snowy | save | 8.2 ms | 41.2 ms | 15.8 ms | n/a |
| snowy | undo | 4.9 ms | 37.8 ms | 16.0 ms | 36.6 ms |
| wenig | open | 1.18 s | 1.22 s | 163 ms | 163 ms |
| wenig | zoom_in | 18.8 ms | 29.3 ms | 5.7 ms | 9.1 ms |
| wenig | zoom_out | 17.1 ms | 38.0 ms | 5.7 ms | 9.2 ms |
| wenig | pan | 18.1 ms | 16.6 ms | 4.8 ms | 8.0 ms |
| wenig | move_step | 2.3 ms | 10.5 ms | 0.1 ms | 5.0 ms |
| wenig | move_place | 38.1 ms | 53.6 ms | 8.2 ms | 57.3 ms |
| wenig | save | 17.7 ms | 39.8 ms | 9.9 ms | n/a |
| wenig | undo | 40.4 ms | 42.3 ms | 7.7 ms | 29.2 ms |
| gosper-curve | open | 1.18 s | 1.22 s | 163 ms | 163 ms |
| gosper-curve | zoom_in | 8.4 ms | 39.6 ms | 8.9 ms | 12.4 ms |
| gosper-curve | zoom_out | 8.8 ms | 36.3 ms | 7.9 ms | 10.8 ms |
| gosper-curve | pan | 5.0 ms | 10.9 ms | 7.4 ms | 10.8 ms |
| gosper-curve | move_step | 0.7 ms | 10.0 ms | 0.1 ms | 5.0 ms |
| gosper-curve | move_place | 11.9 ms | 55.7 ms | 8.6 ms | 32.4 ms |
| gosper-curve | save | 8.8 ms | 34.0 ms | 4.5 ms | n/a |
| gosper-curve | undo | 3.1 ms | 37.8 ms | 7.9 ms | 18.4 ms |
| penrose-tiling | open | 1.20 s | 1.24 s | 171 ms | 171 ms |
| penrose-tiling | zoom_in | 20.2 ms | 36.5 ms | 11.3 ms | 14.1 ms |
| penrose-tiling | zoom_out | 23.4 ms | 33.7 ms | 10.7 ms | 13.7 ms |
| penrose-tiling | pan | 14.6 ms | 15.7 ms | 8.2 ms | 11.0 ms |
| penrose-tiling | move_step | 0.6 ms | 10.2 ms | 0.2 ms | 4.9 ms |
| penrose-tiling | move_place | 24.5 ms | 52.3 ms | 9.5 ms | 18.3 ms |
| penrose-tiling | save | 19.9 ms | 39.3 ms | 3.4 ms | n/a |
| penrose-tiling | undo | 6.5 ms | 37.0 ms | 8.8 ms | 13.3 ms |
| apollonian-gasket | open | 1.18 s | 1.22 s | 195 ms | 195 ms |
| apollonian-gasket | zoom_in | 11.6 ms | 38.3 ms | 9.7 ms | 12.9 ms |
| apollonian-gasket | zoom_out | 11.7 ms | 38.4 ms | 7.0 ms | 10.7 ms |
| apollonian-gasket | pan | 10.7 ms | 11.5 ms | 5.3 ms | 9.1 ms |
| apollonian-gasket | move_step | 1.3 ms | 11.9 ms | 0.2 ms | 5.6 ms |
| apollonian-gasket | move_place | 20.7 ms | 61.3 ms | 12.6 ms | 22.4 ms |
| apollonian-gasket | save | 11.8 ms | 40.1 ms | 3.6 ms | n/a |
| apollonian-gasket | undo | 24.7 ms | 36.4 ms | 13.0 ms | 17.7 ms |
| blankmap | open | 1.20 s | 1.24 s | 164 ms | 164 ms |
| blankmap | zoom_in | 12.2 ms | 39.5 ms | 6.6 ms | 9.6 ms |
| blankmap | zoom_out | 15.3 ms | 31.1 ms | 5.1 ms | 8.5 ms |
| blankmap | pan | 16.6 ms | 15.7 ms | 3.6 ms | 6.6 ms |
| blankmap | move_step | 1.4 ms | 10.8 ms | 0.1 ms | 5.0 ms |
| blankmap | move_place | 54.4 ms | 55.1 ms | 7.9 ms | 16.5 ms |
| blankmap | save | 30.2 ms | 36.6 ms | 4.0 ms | n/a |
| blankmap | undo | 54.9 ms | 55.5 ms | 8.3 ms | 12.9 ms |
| ne-coastline-10m | open | 1.26 s | 1.30 s | 189 ms | 189 ms |
| ne-coastline-10m | zoom_in | 11.2 ms | 37.6 ms | 14.4 ms | 17.5 ms |
| ne-coastline-10m | zoom_out | 12.2 ms | 33.2 ms | 8.0 ms | 11.0 ms |
| ne-coastline-10m | pan | 13.4 ms | 11.4 ms | 4.8 ms | 7.8 ms |
| ne-coastline-10m | move_step | 1.7 ms | 10.2 ms | 0.1 ms | 5.2 ms |
| ne-coastline-10m | move_place | 134 ms | 137 ms | 12.5 ms | 21.5 ms |
| ne-coastline-10m | save | 68.0 ms | 71.6 ms | 11.9 ms | n/a |
| ne-coastline-10m | undo | 39.0 ms | 42.1 ms | 16.5 ms | 21.6 ms |
| ne-countries-10m | open | 1.27 s | 1.31 s | 197 ms | 197 ms |
| ne-countries-10m | zoom_in | 15.8 ms | 38.1 ms | 18.1 ms | 21.0 ms |
| ne-countries-10m | zoom_out | 19.0 ms | 32.5 ms | 11.5 ms | 14.2 ms |
| ne-countries-10m | pan | 21.6 ms | 21.8 ms | 7.3 ms | 10.2 ms |
| ne-countries-10m | move_step | 1.0 ms | 10.2 ms | 0.1 ms | 4.9 ms |
| ne-countries-10m | move_place | 166 ms | 167 ms | 22.5 ms | 32.2 ms |
| ne-countries-10m | save | 76.1 ms | 79.1 ms | 13.7 ms | n/a |
| ne-countries-10m | undo | 47.7 ms | 49.7 ms | 26.3 ms | 30.1 ms |
| greece | open | 1.26 s | 1.30 s | 258 ms | 258 ms |
| greece | zoom_in | 14.6 ms | 32.5 ms | 29.7 ms | 33.2 ms |
| greece | zoom_out | 16.2 ms | 31.6 ms | 25.8 ms | 30.5 ms |
| greece | pan | 13.4 ms | 11.3 ms | 23.1 ms | 26.3 ms |
| greece | move_step | 0.6 ms | 10.3 ms | 0.1 ms | 5.0 ms |
| greece | move_place | 103 ms | 105 ms | 22.5 ms | 31.5 ms |
| greece | save | 47.6 ms | 49.0 ms | 8.9 ms | n/a |
| greece | undo | 30.2 ms | 34.0 ms | 25.7 ms | 31.2 ms |
| clifford-attractor | open | 1.35 s | 1.39 s | 394 ms | 394 ms |
| clifford-attractor | zoom_in | 69.4 ms | 70.9 ms | 105 ms | 90.4 ms |
| clifford-attractor | zoom_out | 71.8 ms | 72.4 ms | 60.0 ms | 57.0 ms |
| clifford-attractor | pan | 42.9 ms | 42.5 ms | 65.6 ms | 58.8 ms |
| clifford-attractor | move_step | 0.5 ms | 10.3 ms | n/a | n/a |
| clifford-attractor | move_place | 105 ms | 106 ms | n/a | n/a |
| clifford-attractor | save | 139 ms | 142 ms | n/a | n/a |
| clifford-attractor | undo | 35.4 ms | 37.1 ms | n/a | n/a |
| cube8 | open | 1.34 s | 1.38 s | 1.36 s | 1.36 s |
| cube8 | zoom_in | 70.2 ms | 71.1 ms | 618 ms | 477 ms |
| cube8 | zoom_out | 85.5 ms | 87.5 ms | 432 ms | 342 ms |
| cube8 | pan | 75.4 ms | 76.3 ms | 223 ms | 186 ms |
| cube8 | move_step | 0.5 ms | 10.2 ms | 2.0 ms | 21.6 ms |
| cube8 | move_place | 144 ms | 147 ms | 559 ms | 586 ms |
| cube8 | save | 120 ms | 122 ms | 119 ms | n/a |
| cube8 | undo | 40.1 ms | 41.8 ms | 766 ms | 595 ms |

<details><summary>Raw numbers (every run, milliseconds)</summary>

| Figure | App | Task | Runs |
|---|---|---|---|
| carnot (small) | xfig | open | 1135.2; 1155.0; 1148.6 |
| carnot (small) | xfig | open_read | 44.0; 41.7; 38.7 |
| carnot (small) | xfig | open_draw | 1.8; 1.0; 1.1 |
| carnot (small) | xfig | zoom_in | 28.2; 26.5; 27.7 |
| carnot (small) | xfig | zoom_out | 7.8; 7.6; 7.9 |
| carnot (small) | xfig | pan | 5.1; 4.6; 4.9 |
| carnot (small) | xfig | move_step | 0.9; 0.8; 0.9 |
| carnot (small) | xfig | move_step_max | 1.0; 1.0; 1.0 |
| carnot (small) | xfig | move_place | 4.5; 5.7; 5.2 |
| carnot (small) | xfig | save | 3.0; 1.5; 2.4 |
| carnot (small) | xfig | undo | 3.7; 4.3; 4.1 |
| carnot (small) | xfig | x_open | 1173.6; 1192.9; 1184.0 |
| carnot (small) | xfig | x_zoom_in | 30.4; 30.0; 27.7 |
| carnot (small) | xfig | x_zoom_out | 34.2; 36.0; 35.1 |
| carnot (small) | xfig | x_pan | 11.3; 8.9; 10.9 |
| carnot (small) | xfig | x_move_step | 9.6; 9.8; 10.0 |
| carnot (small) | xfig | x_move_place | 53.9; 56.3; 55.3 |
| carnot (small) | xfig | x_save | 36.1; 32.2; 29.6 |
| carnot (small) | xfig | x_undo | 30.9; 35.4; 35.3 |
| carnot (small) | RustFig | open | 196.0; 185.3; 185.9 |
| carnot (small) | RustFig | open_read | -; -; - |
| carnot (small) | RustFig | open_draw | -; -; - |
| carnot (small) | RustFig | zoom_in | 8.4; 8.8; 8.3 |
| carnot (small) | RustFig | zoom_out | 11.3; 10.6; 10.3 |
| carnot (small) | RustFig | pan | 23.6; 23.2; 21.9 |
| carnot (small) | RustFig | move_step | 1.0; 1.0; 0.9 |
| carnot (small) | RustFig | move_step_max | 1.4; 1.4; 1.3 |
| carnot (small) | RustFig | move_place | 1.8; 28.3; 1.8 |
| carnot (small) | RustFig | save | 8.7; 6.9; 13.0 |
| carnot (small) | RustFig | undo | 34.9; 30.0; 34.6 |
| carnot (small) | RustFig | x_open | 196.0; 185.3; 185.9 |
| carnot (small) | RustFig | x_zoom_in | 22.8; 23.4; 21.8 |
| carnot (small) | RustFig | x_zoom_out | 27.2; 33.4; 33.4 |
| carnot (small) | RustFig | x_pan | 35.7; 36.2; 39.9 |
| carnot (small) | RustFig | x_move_step | 50.1; 50.1; 50.2 |
| carnot (small) | RustFig | x_move_place | 75.5; 82.1; 83.4 |
| carnot (small) | RustFig | x_save | -; -; - |
| carnot (small) | RustFig | x_undo | 47.9; 52.1; 48.0 |
| feynman (small) | xfig | open | 1137.4; 1142.7; 1137.8 |
| feynman (small) | xfig | open_read | 35.9; 44.3; 42.8 |
| feynman (small) | xfig | open_draw | 1.0; 1.3; 0.9 |
| feynman (small) | xfig | zoom_in | 31.8; 31.7; 32.2 |
| feynman (small) | xfig | zoom_out | 6.9; 6.7; 7.2 |
| feynman (small) | xfig | pan | 4.4; 4.4; 4.4 |
| feynman (small) | xfig | move_step | 9.8; 9.9; 10.0 |
| feynman (small) | xfig | move_step_max | 11.0; 10.9; 11.1 |
| feynman (small) | xfig | move_place | 53.2; 56.0; 55.5 |
| feynman (small) | xfig | save | 2.5; 2.4; 2.6 |
| feynman (small) | xfig | undo | 6.0; 6.2; 6.0 |
| feynman (small) | xfig | x_open | 1175.3; 1182.2; 1173.5 |
| feynman (small) | xfig | x_zoom_in | 31.6; 32.7; 34.3 |
| feynman (small) | xfig | x_zoom_out | 36.5; 34.9; 33.2 |
| feynman (small) | xfig | x_pan | 10.6; 10.9; 10.5 |
| feynman (small) | xfig | x_move_step | 9.8; 9.9; 10.0 |
| feynman (small) | xfig | x_move_place | 53.2; 56.0; 55.5 |
| feynman (small) | xfig | x_save | 35.8; 34.0; 32.9 |
| feynman (small) | xfig | x_undo | 35.2; 34.3; 32.6 |
| feynman (small) | RustFig | open | 202.7; 181.5; 185.4 |
| feynman (small) | RustFig | open_read | -; -; - |
| feynman (small) | RustFig | open_draw | -; -; - |
| feynman (small) | RustFig | zoom_in | 7.4; 7.2; 7.1 |
| feynman (small) | RustFig | zoom_out | 26.0; 22.6; 25.7 |
| feynman (small) | RustFig | pan | 24.7; 25.0; 25.9 |
| feynman (small) | RustFig | move_step | 1.1; 1.1; 1.0 |
| feynman (small) | RustFig | move_step_max | 1.4; 1.4; 1.4 |
| feynman (small) | RustFig | move_place | 2.1; 1.7; 1.7 |
| feynman (small) | RustFig | save | 7.8; 9.2; 12.2 |
| feynman (small) | RustFig | undo | 24.8; 23.6; 26.1 |
| feynman (small) | RustFig | x_open | 202.7; 181.5; 185.4 |
| feynman (small) | RustFig | x_zoom_in | 21.6; 16.9; 23.5 |
| feynman (small) | RustFig | x_zoom_out | 40.7; 31.1; 37.5 |
| feynman (small) | RustFig | x_pan | 42.4; 42.9; 43.1 |
| feynman (small) | RustFig | x_move_step | 50.3; 50.4; 50.3 |
| feynman (small) | RustFig | x_move_place | 74.3; 76.6; 71.6 |
| feynman (small) | RustFig | x_save | -; -; - |
| feynman (small) | RustFig | x_undo | 42.2; 40.3; 45.0 |
| fourier (small) | xfig | open | 1185.0; 1194.1; 1209.1 |
| fourier (small) | xfig | open_read | 41.9; 41.1; 41.7 |
| fourier (small) | xfig | open_draw | 0.9; 1.1; 0.8 |
| fourier (small) | xfig | zoom_in | 27.6; 28.7; 29.8 |
| fourier (small) | xfig | zoom_out | 3.5; 6.9; 7.1 |
| fourier (small) | xfig | pan | 3.5; 4.1; 3.4 |
| fourier (small) | xfig | move_step | 0.9; 0.9; 0.7 |
| fourier (small) | xfig | move_step_max | 0.9; 1.0; 1.0 |
| fourier (small) | xfig | move_place | 4.6; 3.3; 4.0 |
| fourier (small) | xfig | save | 3.1; 3.0; 3.0 |
| fourier (small) | xfig | undo | 3.1; 3.0; 2.9 |
| fourier (small) | xfig | x_open | 1220.0; 1231.4; 1246.3 |
| fourier (small) | xfig | x_zoom_in | 28.9; 35.3; 35.7 |
| fourier (small) | xfig | x_zoom_out | 34.3; 41.7; 37.4 |
| fourier (small) | xfig | x_pan | 8.1; 10.1; 8.9 |
| fourier (small) | xfig | x_move_step | 10.4; 10.5; 8.5 |
| fourier (small) | xfig | x_move_place | 61.8; 53.7; 61.2 |
| fourier (small) | xfig | x_save | 37.0; 35.3; 40.4 |
| fourier (small) | xfig | x_undo | 40.1; 35.6; 39.4 |
| fourier (small) | RustFig | open | 201.4; 190.1; 184.9 |
| fourier (small) | RustFig | open_read | -; -; - |
| fourier (small) | RustFig | open_draw | -; -; - |
| fourier (small) | RustFig | zoom_in | 6.3; 6.8; 6.3 |
| fourier (small) | RustFig | zoom_out | 16.6; 15.1; 15.5 |
| fourier (small) | RustFig | pan | 15.8; 16.9; 13.8 |
| fourier (small) | RustFig | move_step | 0.7; 0.9; 0.9 |
| fourier (small) | RustFig | move_step_max | 1.4; 6.0; 1.3 |
| fourier (small) | RustFig | move_place | 17.8; 16.0; 17.3 |
| fourier (small) | RustFig | save | 7.5; 10.9; 11.9 |
| fourier (small) | RustFig | undo | 16.1; 17.9; 18.0 |
| fourier (small) | RustFig | x_open | 201.4; 190.1; 184.9 |
| fourier (small) | RustFig | x_zoom_in | 16.7; 22.7; 16.7 |
| fourier (small) | RustFig | x_zoom_out | 25.3; 22.5; 24.3 |
| fourier (small) | RustFig | x_pan | 25.8; 29.7; 21.3 |
| fourier (small) | RustFig | x_move_step | 50.2; 50.3; 50.4 |
| fourier (small) | RustFig | x_move_place | 82.9; 77.9; 82.8 |
| fourier (small) | RustFig | x_save | -; -; - |
| fourier (small) | RustFig | x_undo | 33.0; 33.9; 29.9 |
| pines | xfig | open | 1173.8; 1177.8; 1165.9 |
| pines | xfig | open_read | 2.7; 2.7; 2.8 |
| pines | xfig | open_draw | 6.9; 7.1; 6.3 |
| pines | xfig | zoom_in | 8.9; 9.4; 9.2 |
| pines | xfig | zoom_out | 9.5; 9.2; 9.0 |
| pines | xfig | pan | 4.6; 4.5; 4.6 |
| pines | xfig | move_step | 1.1; 1.1; 1.0 |
| pines | xfig | move_step_max | 1.1; 1.1; 1.1 |
| pines | xfig | move_place | 8.0; 8.6; 7.9 |
| pines | xfig | save | 2.8; 2.7; 2.8 |
| pines | xfig | undo | 7.7; 7.2; 7.8 |
| pines | xfig | x_open | 1213.9; 1213.6; 1202.4 |
| pines | xfig | x_zoom_in | 41.1; 41.1; 40.2 |
| pines | xfig | x_zoom_out | 36.1; 41.7; 40.4 |
| pines | xfig | x_pan | 10.9; 11.1; 11.7 |
| pines | xfig | x_move_step | 10.3; 10.1; 10.1 |
| pines | xfig | x_move_place | 57.9; 57.0; 61.8 |
| pines | xfig | x_save | 41.6; 32.2; 31.9 |
| pines | xfig | x_undo | 31.8; 36.2; 41.3 |
| pines | RustFig | open | 181.0; 177.3; 182.9 |
| pines | RustFig | open_read | -; -; - |
| pines | RustFig | open_draw | -; -; - |
| pines | RustFig | zoom_in | 16.5; 17.1; 11.9 |
| pines | RustFig | zoom_out | 20.6; 20.0; 20.5 |
| pines | RustFig | pan | 21.7; 20.2; 18.6 |
| pines | RustFig | move_step | 1.0; 0.9; 1.0 |
| pines | RustFig | move_step_max | 1.4; 1.2; 1.4 |
| pines | RustFig | move_place | 17.3; 1.7; 17.6 |
| pines | RustFig | save | 13.2; 9.3; 8.5 |
| pines | RustFig | undo | 18.7; 20.6; 20.3 |
| pines | RustFig | x_open | 181.0; 177.3; 182.9 |
| pines | RustFig | x_zoom_in | 35.3; 35.4; 26.3 |
| pines | RustFig | x_zoom_out | 35.0; 37.7; 34.8 |
| pines | RustFig | x_pan | 39.5; 37.4; 36.2 |
| pines | RustFig | x_move_step | 50.1; 50.3; 50.2 |
| pines | RustFig | x_move_place | 87.0; 75.0; 99.5 |
| pines | RustFig | x_save | -; -; - |
| pines | RustFig | x_undo | 37.8; 40.4; 41.6 |
| snowy | xfig | open | 1205.1; 1161.3; 1146.4 |
| snowy | xfig | open_read | 11.1; 11.1; 9.4 |
| snowy | xfig | open_draw | 5.6; 5.3; 4.5 |
| snowy | xfig | zoom_in | 7.4; 7.8; 8.4 |
| snowy | xfig | zoom_out | 8.3; 8.6; 4.2 |
| snowy | xfig | pan | 6.8; 6.1; 7.0 |
| snowy | xfig | move_step | 0.6; 0.6; 0.6 |
| snowy | xfig | move_step_max | 0.7; 0.6; 0.6 |
| snowy | xfig | move_place | 18.6; 17.4; 17.9 |
| snowy | xfig | save | 8.2; 8.0; 8.4 |
| snowy | xfig | undo | 4.9; 4.8; 4.9 |
| snowy | xfig | x_open | 1248.0; 1204.1; 1187.0 |
| snowy | xfig | x_zoom_in | 32.2; 37.1; 41.3 |
| snowy | xfig | x_zoom_out | 36.0; 33.0; 33.0 |
| snowy | xfig | x_pan | 11.3; 11.0; 10.1 |
| snowy | xfig | x_move_step | 10.9; 10.4; 10.4 |
| snowy | xfig | x_move_place | 50.4; 60.2; 59.7 |
| snowy | xfig | x_save | 41.2; 41.7; 41.0 |
| snowy | xfig | x_undo | 34.2; 39.3; 37.8 |
| snowy | RustFig | open | 182.6; 182.3; 167.7 |
| snowy | RustFig | open_read | -; -; - |
| snowy | RustFig | open_draw | -; -; - |
| snowy | RustFig | zoom_in | 8.9; 10.7; 8.9 |
| snowy | RustFig | zoom_out | 9.5; 9.6; 10.8 |
| snowy | RustFig | pan | 8.3; 9.3; 7.9 |
| snowy | RustFig | move_step | 1.1; 1.1; 1.1 |
| snowy | RustFig | move_step_max | 1.4; 1.3; 10.0 |
| snowy | RustFig | move_place | 17.3; 18.6; 16.7 |
| snowy | RustFig | save | 15.8; 12.5; 15.8 |
| snowy | RustFig | undo | 16.2; 16.0; 15.3 |
| snowy | RustFig | x_open | 182.6; 182.3; 167.7 |
| snowy | RustFig | x_zoom_in | 26.2; 27.7; 32.4 |
| snowy | RustFig | x_zoom_out | 26.8; 25.8; 31.5 |
| snowy | RustFig | x_pan | 24.8; 26.1; 31.7 |
| snowy | RustFig | x_move_step | 50.3; 50.2; 50.1 |
| snowy | RustFig | x_move_place | 83.2; 83.0; 79.8 |
| snowy | RustFig | x_save | -; -; - |
| snowy | RustFig | x_undo | 36.6; 32.2; 37.6 |
| wenig | xfig | open | 1189.4; 1183.4; 1173.6 |
| wenig | xfig | open_read | 22.6; 25.2; 21.5 |
| wenig | xfig | open_draw | 10.3; 12.0; 9.8 |
| wenig | xfig | zoom_in | 22.1; 16.3; 18.8 |
| wenig | xfig | zoom_out | 14.9; 17.8; 17.1 |
| wenig | xfig | pan | 18.1; 18.1; 18.1 |
| wenig | xfig | move_step | 2.3; 2.3; 2.3 |
| wenig | xfig | move_step_max | 2.5; 2.5; 2.4 |
| wenig | xfig | move_place | 38.1; 36.1; 41.0 |
| wenig | xfig | save | 17.7; 16.9; 21.7 |
| wenig | xfig | undo | 41.4; 39.6; 40.4 |
| wenig | xfig | x_open | 1228.0; 1217.0; 1212.2 |
| wenig | xfig | x_zoom_in | 34.5; 28.1; 29.3 |
| wenig | xfig | x_zoom_out | 40.0; 36.4; 38.0 |
| wenig | xfig | x_pan | 16.7; 16.6; 16.5 |
| wenig | xfig | x_move_step | 10.5; 10.5; 10.3 |
| wenig | xfig | x_move_place | 53.9; 48.6; 53.6 |
| wenig | xfig | x_save | 39.8; 40.9; 36.2 |
| wenig | xfig | x_undo | 42.9; 39.8; 42.3 |
| wenig | RustFig | open | 181.5; 163.0; 159.8 |
| wenig | RustFig | open_read | -; -; - |
| wenig | RustFig | open_draw | -; -; - |
| wenig | RustFig | zoom_in | 15.1; 5.7; 5.6 |
| wenig | RustFig | zoom_out | 6.8; 5.7; 5.7 |
| wenig | RustFig | pan | 4.8; 4.9; 4.6 |
| wenig | RustFig | move_step | 0.1; 0.1; 0.1 |
| wenig | RustFig | move_step_max | 0.2; 0.2; 0.5 |
| wenig | RustFig | move_place | 7.1; 8.2; 8.2 |
| wenig | RustFig | save | 10.4; 9.7; 9.9 |
| wenig | RustFig | undo | 7.3; 7.9; 7.7 |
| wenig | RustFig | x_open | 181.5; 163.0; 159.8 |
| wenig | RustFig | x_zoom_in | 23.5; 9.1; 8.5 |
| wenig | RustFig | x_zoom_out | 10.5; 8.7; 9.2 |
| wenig | RustFig | x_pan | 10.0; 8.0; 7.6 |
| wenig | RustFig | x_move_step | 5.0; 5.0; 5.2 |
| wenig | RustFig | x_move_place | 60.5; 55.4; 57.3 |
| wenig | RustFig | x_save | -; -; - |
| wenig | RustFig | x_undo | 28.4; 29.4; 29.2 |
| gosper-curve | xfig | open | 1187.8; 1151.2; 1178.7 |
| gosper-curve | xfig | open_read | 13.0; 5.3; 11.8 |
| gosper-curve | xfig | open_draw | 4.7; 3.1; 4.4 |
| gosper-curve | xfig | zoom_in | 8.4; 8.5; 8.1 |
| gosper-curve | xfig | zoom_out | 8.8; 11.1; 8.4 |
| gosper-curve | xfig | pan | 5.0; 5.0; 4.9 |
| gosper-curve | xfig | move_step | 0.7; 0.7; 0.7 |
| gosper-curve | xfig | move_step_max | 0.8; 0.8; 0.8 |
| gosper-curve | xfig | move_place | 11.7; 11.9; 22.2 |
| gosper-curve | xfig | save | 3.7; 8.8; 9.0 |
| gosper-curve | xfig | undo | 2.6; 3.1; 5.9 |
| gosper-curve | xfig | x_open | 1230.1; 1180.2; 1216.0 |
| gosper-curve | xfig | x_zoom_in | 39.6; 35.0; 40.9 |
| gosper-curve | xfig | x_zoom_out | 38.2; 33.8; 36.3 |
| gosper-curve | xfig | x_pan | 11.0; 10.9; 10.8 |
| gosper-curve | xfig | x_move_step | 10.0; 10.0; 10.5 |
| gosper-curve | xfig | x_move_place | 56.3; 55.2; 55.7 |
| gosper-curve | xfig | x_save | 37.1; 34.0; 32.3 |
| gosper-curve | xfig | x_undo | 33.3; 39.8; 37.8 |
| gosper-curve | RustFig | open | 181.3; 158.7; 163.2 |
| gosper-curve | RustFig | open_read | -; -; - |
| gosper-curve | RustFig | open_draw | -; -; - |
| gosper-curve | RustFig | zoom_in | 8.9; 8.9; 8.7 |
| gosper-curve | RustFig | zoom_out | 7.9; 7.9; 8.6 |
| gosper-curve | RustFig | pan | 7.4; 7.4; 7.4 |
| gosper-curve | RustFig | move_step | 0.1; 0.1; 0.1 |
| gosper-curve | RustFig | move_step_max | 0.2; 0.5; 0.3 |
| gosper-curve | RustFig | move_place | 8.6; 9.5; 8.4 |
| gosper-curve | RustFig | save | 4.5; 4.7; 4.5 |
| gosper-curve | RustFig | undo | 7.8; 7.9; 8.1 |
| gosper-curve | RustFig | x_open | 181.3; 158.7; 163.2 |
| gosper-curve | RustFig | x_zoom_in | 12.4; 12.4; 11.5 |
| gosper-curve | RustFig | x_zoom_out | 10.8; 10.7; 11.5 |
| gosper-curve | RustFig | x_pan | 10.9; 10.8; 10.3 |
| gosper-curve | RustFig | x_move_step | 5.0; 5.0; 5.3 |
| gosper-curve | RustFig | x_move_place | 32.4; 32.3; 34.2 |
| gosper-curve | RustFig | x_save | -; -; - |
| gosper-curve | RustFig | x_undo | 18.4; 18.4; 18.7 |
| penrose-tiling | xfig | open | 1199.7; 1198.9; 1200.2 |
| penrose-tiling | xfig | open_read | 21.6; 20.0; 20.8 |
| penrose-tiling | xfig | open_draw | 13.6; 14.3; 13.3 |
| penrose-tiling | xfig | zoom_in | 20.2; 20.7; 20.1 |
| penrose-tiling | xfig | zoom_out | 23.4; 22.9; 23.8 |
| penrose-tiling | xfig | pan | 14.2; 14.6; 14.7 |
| penrose-tiling | xfig | move_step | 0.6; 0.6; 0.6 |
| penrose-tiling | xfig | move_step_max | 0.6; 0.8; 0.7 |
| penrose-tiling | xfig | move_place | 24.5; 25.4; 23.5 |
| penrose-tiling | xfig | save | 18.9; 20.4; 19.9 |
| penrose-tiling | xfig | undo | 6.5; 6.6; 6.3 |
| penrose-tiling | xfig | x_open | 1234.8; 1238.1; 1239.7 |
| penrose-tiling | xfig | x_zoom_in | 37.9; 36.5; 31.0 |
| penrose-tiling | xfig | x_zoom_out | 35.7; 30.4; 33.7 |
| penrose-tiling | xfig | x_pan | 15.7; 15.2; 15.9 |
| penrose-tiling | xfig | x_move_step | 9.6; 10.2; 10.5 |
| penrose-tiling | xfig | x_move_place | 49.9; 52.3; 55.6 |
| penrose-tiling | xfig | x_save | 32.8; 39.8; 39.3 |
| penrose-tiling | xfig | x_undo | 37.0; 31.6; 40.7 |
| penrose-tiling | RustFig | open | 191.6; 171.1; 169.2 |
| penrose-tiling | RustFig | open_read | -; -; - |
| penrose-tiling | RustFig | open_draw | -; -; - |
| penrose-tiling | RustFig | zoom_in | 11.3; 11.2; 11.3 |
| penrose-tiling | RustFig | zoom_out | 10.9; 10.7; 10.3 |
| penrose-tiling | RustFig | pan | 8.2; 8.5; 8.0 |
| penrose-tiling | RustFig | move_step | 0.2; 0.2; 0.1 |
| penrose-tiling | RustFig | move_step_max | 0.6; 0.6; 0.5 |
| penrose-tiling | RustFig | move_place | 9.5; 9.6; 9.1 |
| penrose-tiling | RustFig | save | 3.4; 3.4; 3.5 |
| penrose-tiling | RustFig | undo | 8.6; 8.8; 9.1 |
| penrose-tiling | RustFig | x_open | 191.6; 171.1; 169.2 |
| penrose-tiling | RustFig | x_zoom_in | 14.1; 14.1; 14.1 |
| penrose-tiling | RustFig | x_zoom_out | 13.7; 13.7; 13.1 |
| penrose-tiling | RustFig | x_pan | 11.0; 11.2; 10.8 |
| penrose-tiling | RustFig | x_move_step | 5.0; 4.9; 4.9 |
| penrose-tiling | RustFig | x_move_place | 18.6; 18.3; 17.4 |
| penrose-tiling | RustFig | x_save | -; -; - |
| penrose-tiling | RustFig | x_undo | 13.2; 13.3; 14.3 |
| apollonian-gasket | xfig | open | 1177.6; 1183.5; 1184.4 |
| apollonian-gasket | xfig | open_read | 11.9; 13.0; 12.4 |
| apollonian-gasket | xfig | open_draw | 5.5; 6.4; 11.2 |
| apollonian-gasket | xfig | zoom_in | 10.2; 11.6; 15.5 |
| apollonian-gasket | xfig | zoom_out | 11.3; 11.7; 15.9 |
| apollonian-gasket | xfig | pan | 9.9; 10.7; 13.7 |
| apollonian-gasket | xfig | move_step | 1.4; 1.3; 1.3 |
| apollonian-gasket | xfig | move_step_max | 1.6; 1.4; 1.5 |
| apollonian-gasket | xfig | move_place | 20.7; 17.8; 22.3 |
| apollonian-gasket | xfig | save | 11.8; 12.7; 7.5 |
| apollonian-gasket | xfig | undo | 21.0; 26.8; 24.7 |
| apollonian-gasket | xfig | x_open | 1209.8; 1221.7; 1222.9 |
| apollonian-gasket | xfig | x_zoom_in | 33.3; 38.3; 41.9 |
| apollonian-gasket | xfig | x_zoom_out | 39.1; 38.4; 33.3 |
| apollonian-gasket | xfig | x_pan | 11.1; 11.5; 13.4 |
| apollonian-gasket | xfig | x_move_step | 10.0; 12.6; 11.9 |
| apollonian-gasket | xfig | x_move_place | 56.8; 61.3; 62.3 |
| apollonian-gasket | xfig | x_save | 35.0; 41.9; 40.1 |
| apollonian-gasket | xfig | x_undo | 36.4; 36.7; 31.7 |
| apollonian-gasket | RustFig | open | 226.7; 184.1; 195.4 |
| apollonian-gasket | RustFig | open_read | -; -; - |
| apollonian-gasket | RustFig | open_draw | -; -; - |
| apollonian-gasket | RustFig | zoom_in | 10.0; 9.1; 9.7 |
| apollonian-gasket | RustFig | zoom_out | 8.1; 7.0; 7.0 |
| apollonian-gasket | RustFig | pan | 5.3; 4.8; 5.3 |
| apollonian-gasket | RustFig | move_step | 0.2; 0.2; 0.2 |
| apollonian-gasket | RustFig | move_step_max | 0.4; 0.3; 0.4 |
| apollonian-gasket | RustFig | move_place | 12.6; 12.6; 12.9 |
| apollonian-gasket | RustFig | save | 3.7; 3.5; 3.6 |
| apollonian-gasket | RustFig | undo | 11.9; 13.1; 13.0 |
| apollonian-gasket | RustFig | x_open | 226.7; 184.1; 195.4 |
| apollonian-gasket | RustFig | x_zoom_in | 13.9; 12.7; 12.9 |
| apollonian-gasket | RustFig | x_zoom_out | 12.0; 10.0; 10.7 |
| apollonian-gasket | RustFig | x_pan | 9.3; 7.8; 9.1 |
| apollonian-gasket | RustFig | x_move_step | 5.6; 5.3; 5.7 |
| apollonian-gasket | RustFig | x_move_place | 22.9; 21.5; 22.4 |
| apollonian-gasket | RustFig | x_save | -; -; - |
| apollonian-gasket | RustFig | x_undo | 17.1; 18.3; 17.7 |
| blankmap | xfig | open | 1195.5; 1205.8; 1202.2 |
| blankmap | xfig | open_read | 31.8; 34.0; 30.1 |
| blankmap | xfig | open_draw | 3.0; 4.7; 3.6 |
| blankmap | xfig | zoom_in | 8.8; 12.2; 12.6 |
| blankmap | xfig | zoom_out | 14.2; 16.5; 15.3 |
| blankmap | xfig | pan | 16.2; 16.8; 16.6 |
| blankmap | xfig | move_step | 1.3; 1.4; 1.4 |
| blankmap | xfig | move_step_max | 2.7; 1.5; 1.5 |
| blankmap | xfig | move_place | 54.4; 53.5; 54.8 |
| blankmap | xfig | save | 30.2; 30.7; 26.1 |
| blankmap | xfig | undo | 56.1; 54.9; 52.7 |
| blankmap | xfig | x_open | 1228.2; 1246.4; 1239.4 |
| blankmap | xfig | x_zoom_in | 40.1; 39.5; 39.2 |
| blankmap | xfig | x_zoom_out | 33.9; 29.8; 31.1 |
| blankmap | xfig | x_pan | 15.0; 16.0; 15.7 |
| blankmap | xfig | x_move_step | 11.0; 10.8; 10.1 |
| blankmap | xfig | x_move_place | 57.0; 54.9; 55.1 |
| blankmap | xfig | x_save | 36.9; 32.8; 36.6 |
| blankmap | xfig | x_undo | 57.1; 54.8; 55.5 |
| blankmap | RustFig | open | 179.7; 163.8; 163.6 |
| blankmap | RustFig | open_read | -; -; - |
| blankmap | RustFig | open_draw | -; -; - |
| blankmap | RustFig | zoom_in | 6.7; 6.0; 6.6 |
| blankmap | RustFig | zoom_out | 5.2; 5.1; 5.1 |
| blankmap | RustFig | pan | 3.7; 3.5; 3.6 |
| blankmap | RustFig | move_step | 0.1; 0.1; 0.1 |
| blankmap | RustFig | move_step_max | 0.2; 0.2; 0.2 |
| blankmap | RustFig | move_place | 7.7; 7.9; 8.7 |
| blankmap | RustFig | save | 4.6; 4.0; 3.8 |
| blankmap | RustFig | undo | 8.2; 8.3; 8.3 |
| blankmap | RustFig | x_open | 179.7; 163.8; 163.6 |
| blankmap | RustFig | x_zoom_in | 9.6; 9.0; 10.0 |
| blankmap | RustFig | x_zoom_out | 8.5; 8.5; 8.6 |
| blankmap | RustFig | x_pan | 6.6; 6.4; 6.7 |
| blankmap | RustFig | x_move_step | 5.0; 5.4; 5.0 |
| blankmap | RustFig | x_move_place | 16.5; 16.3; 18.1 |
| blankmap | RustFig | x_save | -; -; - |
| blankmap | RustFig | x_undo | 12.8; 13.0; 12.9 |
| ne-coastline-10m | xfig | open | 1261.2; 1256.6; 1246.9 |
| ne-coastline-10m | xfig | open_read | 83.5; 79.3; 75.5 |
| ne-coastline-10m | xfig | open_draw | 2.7; 2.8; 3.0 |
| ne-coastline-10m | xfig | zoom_in | 11.4; 11.2; 11.1 |
| ne-coastline-10m | xfig | zoom_out | 12.3; 11.3; 12.2 |
| ne-coastline-10m | xfig | pan | 13.7; 10.4; 13.4 |
| ne-coastline-10m | xfig | move_step | 1.7; 1.7; 1.7 |
| ne-coastline-10m | xfig | move_step_max | 1.8; 1.7; 1.7 |
| ne-coastline-10m | xfig | move_place | 136.0; 133.8; 129.1 |
| ne-coastline-10m | xfig | save | 68.0; 67.5; 70.6 |
| ne-coastline-10m | xfig | undo | 39.6; 39.0; 36.7 |
| ne-coastline-10m | xfig | x_open | 1302.0; 1299.0; 1285.3 |
| ne-coastline-10m | xfig | x_zoom_in | 32.0; 37.6; 40.0 |
| ne-coastline-10m | xfig | x_zoom_out | 32.2; 41.6; 33.2 |
| ne-coastline-10m | xfig | x_pan | 11.8; 9.7; 11.4 |
| ne-coastline-10m | xfig | x_move_step | 10.2; 10.5; 10.0 |
| ne-coastline-10m | xfig | x_move_place | 137.2; 136.7; 130.9 |
| ne-coastline-10m | xfig | x_save | 71.6; 70.7; 73.7 |
| ne-coastline-10m | xfig | x_undo | 43.6; 42.1; 37.6 |
| ne-coastline-10m | RustFig | open | 202.0; 188.9; 185.8 |
| ne-coastline-10m | RustFig | open_read | -; -; - |
| ne-coastline-10m | RustFig | open_draw | -; -; - |
| ne-coastline-10m | RustFig | zoom_in | 15.4; 14.4; 14.1 |
| ne-coastline-10m | RustFig | zoom_out | 8.1; 8.0; 7.7 |
| ne-coastline-10m | RustFig | pan | 4.8; 5.0; 4.7 |
| ne-coastline-10m | RustFig | move_step | 0.1; 0.1; 0.1 |
| ne-coastline-10m | RustFig | move_step_max | 0.7; 0.9; 0.7 |
| ne-coastline-10m | RustFig | move_place | 12.5; 12.4; 15.3 |
| ne-coastline-10m | RustFig | save | 11.9; 10.6; 12.2 |
| ne-coastline-10m | RustFig | undo | 16.6; 15.9; 16.5 |
| ne-coastline-10m | RustFig | x_open | 202.0; 188.9; 185.8 |
| ne-coastline-10m | RustFig | x_zoom_in | 18.6; 17.2; 17.5 |
| ne-coastline-10m | RustFig | x_zoom_out | 11.0; 11.4; 10.4 |
| ne-coastline-10m | RustFig | x_pan | 8.3; 7.8; 7.5 |
| ne-coastline-10m | RustFig | x_move_step | 5.3; 5.0; 5.2 |
| ne-coastline-10m | RustFig | x_move_place | 21.5; 21.4; 23.9 |
| ne-coastline-10m | RustFig | x_save | -; -; - |
| ne-coastline-10m | RustFig | x_undo | 21.8; 20.5; 21.6 |
| ne-countries-10m | xfig | open | 1274.2; 1249.6; 1265.4 |
| ne-countries-10m | xfig | open_read | 88.0; 97.0; 94.3 |
| ne-countries-10m | xfig | open_draw | 5.2; 5.9; 5.8 |
| ne-countries-10m | xfig | zoom_in | 11.4; 20.1; 15.8 |
| ne-countries-10m | xfig | zoom_out | 19.0; 19.7; 19.0 |
| ne-countries-10m | xfig | pan | 19.9; 22.4; 21.6 |
| ne-countries-10m | xfig | move_step | 1.0; 1.0; 1.0 |
| ne-countries-10m | xfig | move_step_max | 1.1; 1.1; 1.1 |
| ne-countries-10m | xfig | move_place | 166.3; 169.5; 164.9 |
| ne-countries-10m | xfig | save | 71.6; 76.1; 76.4 |
| ne-countries-10m | xfig | undo | 44.8; 47.7; 48.4 |
| ne-countries-10m | xfig | x_open | 1311.8; 1291.3; 1306.8 |
| ne-countries-10m | xfig | x_zoom_in | 38.1; 38.5; 33.3 |
| ne-countries-10m | xfig | x_zoom_out | 34.2; 32.5; 28.4 |
| ne-countries-10m | xfig | x_pan | 20.0; 22.2; 21.8 |
| ne-countries-10m | xfig | x_move_step | 10.2; 10.2; 9.9 |
| ne-countries-10m | xfig | x_move_place | 167.0; 173.0; 167.3 |
| ne-countries-10m | xfig | x_save | 74.0; 79.7; 79.1 |
| ne-countries-10m | xfig | x_undo | 46.8; 50.0; 49.7 |
| ne-countries-10m | RustFig | open | 207.8; 192.3; 197.4 |
| ne-countries-10m | RustFig | open_read | -; -; - |
| ne-countries-10m | RustFig | open_draw | -; -; - |
| ne-countries-10m | RustFig | zoom_in | 18.1; 17.6; 18.4 |
| ne-countries-10m | RustFig | zoom_out | 11.2; 11.5; 11.8 |
| ne-countries-10m | RustFig | pan | 7.3; 7.3; 7.5 |
| ne-countries-10m | RustFig | move_step | 0.1; 0.1; 0.1 |
| ne-countries-10m | RustFig | move_step_max | 0.5; 0.8; 1.0 |
| ne-countries-10m | RustFig | move_place | 22.2; 23.8; 22.5 |
| ne-countries-10m | RustFig | save | 13.6; 13.7; 14.6 |
| ne-countries-10m | RustFig | undo | 26.1; 26.3; 26.9 |
| ne-countries-10m | RustFig | x_open | 207.8; 192.3; 197.4 |
| ne-countries-10m | RustFig | x_zoom_in | 21.0; 20.6; 21.4 |
| ne-countries-10m | RustFig | x_zoom_out | 14.1; 14.2; 14.6 |
| ne-countries-10m | RustFig | x_pan | 10.2; 10.1; 10.4 |
| ne-countries-10m | RustFig | x_move_step | 4.9; 5.0; 4.9 |
| ne-countries-10m | RustFig | x_move_place | 31.8; 33.8; 32.2 |
| ne-countries-10m | RustFig | x_save | -; -; - |
| ne-countries-10m | RustFig | x_undo | 29.8; 31.8; 30.1 |
| greece | xfig | open | 1296.2; 1259.1; 1262.0 |
| greece | xfig | open_read | 96.2; 92.2; 89.0 |
| greece | xfig | open_draw | 5.2; 5.4; 5.5 |
| greece | xfig | zoom_in | 13.8; 15.1; 14.6 |
| greece | xfig | zoom_out | 15.6; 16.2; 17.0 |
| greece | xfig | pan | 12.3; 13.4; 13.4 |
| greece | xfig | move_step | 0.6; 0.5; 0.6 |
| greece | xfig | move_step_max | 0.6; 0.7; 0.7 |
| greece | xfig | move_place | 102.6; 107.4; 96.2 |
| greece | xfig | save | 52.3; 45.1; 47.6 |
| greece | xfig | undo | 30.7; 30.2; 29.1 |
| greece | xfig | x_open | 1336.1; 1300.7; 1300.7 |
| greece | xfig | x_zoom_in | 39.5; 31.3; 32.5 |
| greece | xfig | x_zoom_out | 28.5; 31.6; 39.4 |
| greece | xfig | x_pan | 14.9; 11.3; 11.2 |
| greece | xfig | x_move_step | 10.4; 10.3; 10.1 |
| greece | xfig | x_move_place | 105.2; 109.7; 97.6 |
| greece | xfig | x_save | 54.2; 47.4; 49.0 |
| greece | xfig | x_undo | 34.0; 33.4; 34.4 |
| greece | RustFig | open | 274.3; 257.1; 258.1 |
| greece | RustFig | open_read | -; -; - |
| greece | RustFig | open_draw | -; -; - |
| greece | RustFig | zoom_in | 30.0; 29.7; 27.5 |
| greece | RustFig | zoom_out | 25.8; 26.2; 24.5 |
| greece | RustFig | pan | 23.4; 23.1; 22.5 |
| greece | RustFig | move_step | 0.1; 0.1; 0.1 |
| greece | RustFig | move_step_max | 1.2; 1.0; 1.2 |
| greece | RustFig | move_place | 22.1; 22.5; 23.3 |
| greece | RustFig | save | 8.0; 9.3; 8.9 |
| greece | RustFig | undo | 1.7; 25.8; 25.7 |
| greece | RustFig | x_open | 274.3; 257.1; 258.1 |
| greece | RustFig | x_zoom_in | 33.2; 33.2; 32.2 |
| greece | RustFig | x_zoom_out | 30.5; 31.0; 27.6 |
| greece | RustFig | x_pan | 28.2; 25.6; 26.3 |
| greece | RustFig | x_move_step | 5.0; 5.3; 5.0 |
| greece | RustFig | x_move_place | 30.7; 31.5; 33.3 |
| greece | RustFig | x_save | -; -; - |
| greece | RustFig | x_undo | 29.1; 31.2; 32.0 |
| clifford-attractor | xfig | open | 1354.2; 1336.5; 1369.1 |
| clifford-attractor | xfig | open_read | 130.8; 133.2; 132.5 |
| clifford-attractor | xfig | open_draw | 35.0; 34.9; 35.0 |
| clifford-attractor | xfig | zoom_in | 69.4; 63.2; 72.4 |
| clifford-attractor | xfig | zoom_out | 66.9; 71.8; 73.8 |
| clifford-attractor | xfig | pan | 42.9; 41.6; 49.5 |
| clifford-attractor | xfig | move_step | 0.5; 0.5; 0.6 |
| clifford-attractor | xfig | move_step_max | 0.6; 0.6; 0.6 |
| clifford-attractor | xfig | move_place | 105.1; 105.5; 104.0 |
| clifford-attractor | xfig | save | 137.2; 145.0; 139.2 |
| clifford-attractor | xfig | undo | 34.0; 35.4; 36.2 |
| clifford-attractor | xfig | x_open | 1389.6; 1378.6; 1411.3 |
| clifford-attractor | xfig | x_zoom_in | 70.9; 63.2; 74.9 |
| clifford-attractor | xfig | x_zoom_out | 69.5; 72.4; 75.0 |
| clifford-attractor | xfig | x_pan | 42.4; 42.5; 50.7 |
| clifford-attractor | xfig | x_move_step | 10.8; 10.3; 10.2 |
| clifford-attractor | xfig | x_move_place | 107.5; 106.3; 105.3 |
| clifford-attractor | xfig | x_save | 141.0; 147.2; 142.0 |
| clifford-attractor | xfig | x_undo | 35.9; 37.4; 37.1 |
| clifford-attractor | RustFig | open | 405.9; 392.1; 394.1 |
| clifford-attractor | RustFig | open_read | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":536}) |
| clifford-attractor | RustFig | open_draw | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":536}) |
| clifford-attractor | RustFig | zoom_in | 111.6; 98.6; 104.6 |
| clifford-attractor | RustFig | zoom_out | 56.6; 60.0; 67.0 |
| clifford-attractor | RustFig | pan | 66.2; 65.6; 60.8 |
| clifford-attractor | RustFig | move_step | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":536}) |
| clifford-attractor | RustFig | move_step_max | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":536}) |
| clifford-attractor | RustFig | move_place | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":536}) |
| clifford-attractor | RustFig | save | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":536}) |
| clifford-attractor | RustFig | undo | n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":539}); n/a (engine.execute {"command":"edit.select_all","params":{}}: {"error":{"code":"timeout","message":"the app did not answer"},"id":536}) |
| clifford-attractor | RustFig | x_open | 405.9; 392.1; 394.1 |
| clifford-attractor | RustFig | x_zoom_in | 92.5; 86.2; 90.4 |
| clifford-attractor | RustFig | x_zoom_out | 54.4; 57.0; 63.6 |
| clifford-attractor | RustFig | x_pan | 58.8; 59.4; 56.5 |
| clifford-attractor | RustFig | x_move_step | -; -; - |
| clifford-attractor | RustFig | x_move_place | -; -; - |
| clifford-attractor | RustFig | x_save | -; -; - |
| clifford-attractor | RustFig | x_undo | -; -; - |
| cube8 | xfig | open | 1336.0; 1337.5; 1340.4 |
| cube8 | xfig | open_read | 121.7; 129.2; 126.5 |
| cube8 | xfig | open_draw | 48.0; 46.3; 45.7 |
| cube8 | xfig | zoom_in | 70.6; 69.3; 70.2 |
| cube8 | xfig | zoom_out | 86.3; 82.0; 85.5 |
| cube8 | xfig | pan | 78.5; 75.4; 75.2 |
| cube8 | xfig | move_step | 0.5; 0.5; 0.5 |
| cube8 | xfig | move_step_max | 0.6; 0.7; 0.8 |
| cube8 | xfig | move_place | 144.3; 143.9; 147.4 |
| cube8 | xfig | save | 118.9; 119.5; 125.1 |
| cube8 | xfig | undo | 39.1; 40.2; 40.1 |
| cube8 | xfig | x_open | 1370.8; 1375.5; 1378.0 |
| cube8 | xfig | x_zoom_in | 71.5; 71.1; 70.1 |
| cube8 | xfig | x_zoom_out | 88.6; 84.1; 87.5 |
| cube8 | xfig | x_pan | 81.6; 76.3; 76.2 |
| cube8 | xfig | x_move_step | 10.4; 10.2; 10.1 |
| cube8 | xfig | x_move_place | 147.4; 146.5; 148.9 |
| cube8 | xfig | x_save | 121.2; 122.0; 127.9 |
| cube8 | xfig | x_undo | 39.2; 43.6; 41.8 |
| cube8 | RustFig | open | 1316.7; 1356.0; 1389.7 |
| cube8 | RustFig | open_read | -; -; - |
| cube8 | RustFig | open_draw | -; -; - |
| cube8 | RustFig | zoom_in | 618.5; 595.8; 622.2 |
| cube8 | RustFig | zoom_out | 437.7; 424.8; 431.5 |
| cube8 | RustFig | pan | 225.7; 222.7; 218.0 |
| cube8 | RustFig | move_step | 4.0; 2.0; 1.5 |
| cube8 | RustFig | move_step_max | 6.4; 9.9; 7.8 |
| cube8 | RustFig | move_place | 558.7; 597.9; 531.6 |
| cube8 | RustFig | save | 118.9; 120.5; 118.9 |
| cube8 | RustFig | undo | 766.3; 753.4; 797.9 |
| cube8 | RustFig | x_open | 1316.7; 1356.0; 1389.7 |
| cube8 | RustFig | x_zoom_in | 485.9; 468.8; 476.8 |
| cube8 | RustFig | x_zoom_out | 345.9; 336.2; 341.6 |
| cube8 | RustFig | x_pan | 186.4; 187.6; 182.5 |
| cube8 | RustFig | x_move_step | 25.6; 19.5; 21.6 |
| cube8 | RustFig | x_move_place | 586.5; 656.4; 567.3 |
| cube8 | RustFig | x_save | -; -; - |
| cube8 | RustFig | x_undo | 580.7; 595.5; 612.0 |

</details>

Move test: 30 steps over 240 x 120 pixels. Figures with more than 300 objects (snowy, penrose-tiling, apollonian-gasket, blankmap, ne-coastline-10m, ne-countries-10m, greece, clifford-attractor, cube8) were glued into one compound first (xfig: Glue, middle-button region, right button; RustFig: select all, group), smaller ones move a single object.

<!-- results-retina:end -->
