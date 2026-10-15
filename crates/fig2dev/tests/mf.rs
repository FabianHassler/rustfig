#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `mf` driver against output of the real fig2dev 3.2.9a.

mod mp_common;
mod tex_common;

const BODY: &str = concat!(
    "# HREF=\"http://a\" ALT=\"A\"\n",
    "2 1 1 2 4 7 50 -1 -1 4.000 0 0 -1 1 0 3\n",
    "\t1 1 1.00 60.00 120.00\n",
    "\t 600 600 1800 600 1800 1200\n",
    "1 3 0 1 0 7 40 -1 20 0.000 1 0.0000 1200 2400 300 300 1200 2400 1500 2400\n",
    "4 1 0 30 -1 0 12 0.0000 4 135 450 2400 2400 a_b\\001\n",
);

const EXPECTED: &str = concat!(
    "%\n",
    "% fig2dev -L mf (Version 3.2.9a)\n",
    "%\n",
    "%\n",
    "% Preamble\n",
    "%\n",
    "mag:=1; input grafbase.mf; code:=32;\n",
    "interim hdwdr:=1; interim hdten:=1;\n",
    "%\n",
    "% stdin (char 33)\n",
    "%\n",
    "xscale:=0.125; yscale:=0.125;\n",
    "bounds(0.000,8.000,0.000,8.000);\n",
    "unitlen:=1.0in#;\n",
    "beginmfpic(incr code);\n",
    "penwd := 0.01pt;\n",
    "drawpen := pencircle scaled penwd yscaled aspect_ratio;\n",
    "%  HREF=\"http://a\" ALT=\"A\" \n",
    "penwd := 0.25pt;\n",
    "drawpen := pencircle scaled penwd yscaled aspect_ratio;\n",
    "\tstore (curpath)\n",
    "  drawn polyline(false)\n",
    "\t   ((0.500000, 7.500000),\n",
    "\t   (1.500000, 7.500000),\n",
    "\t   (1.500000, 7.000000));\n",
    "penwd := 0.12pt;\n",
    "drawpen := pencircle scaled penwd yscaled aspect_ratio;\n",
    "\tstore (curpath)\n",
    "  filled circle((1.000000,6.000000),0.250000);\n",
    "endmfpic;\n",
    "end.\n",
);

#[test]
fn as_fig2dev() {
    let (out, msgs) = mp_common::run("mf", BODY, &[]);
    assert_eq!(out, EXPECTED);
    assert_eq!(msgs, ["warning: text ignored in MF output: `a_b'"]);
}

#[test]
fn unknown_option() {
    let mut d = rustfig_fig2dev::drivers::driver("mf").unwrap();
    let mut ctx = rustfig_fig2dev::Ctx::default();
    assert_eq!(
        d.option(&mut ctx, 'z', None).unwrap_err(),
        "Argument -z unknown to mf driver."
    );
    assert!(d.option(&mut ctx, 'G', Some("1")).is_ok());
}

const EXPECTED_OPTS: &str = concat!(
    "%\n",
    "% fig2dev -L mf (Version 3.2.9a)\n",
    "%\n",
    "%\n",
    "% Preamble\n",
    "%\n",
    "mag:=1; input grafbase.mf; code:=65;\n",
    "interim hdwdr:=1; interim hdten:=1;\n",
    "%\n",
    "% fig (char 66)\n",
    "%\n",
    "xscale:=0.125; yscale:=0.125;\n",
    "bounds(1.000,9.000,0.000,8.000);\n",
    "unitlen:=1.0in#;\n",
    "beginmfpic(incr code);\n",
    "penwd := 0.01pt;\n",
    "drawpen := pencircle scaled penwd yscaled aspect_ratio;\n",
    "%  HREF=\"http://a\" ALT=\"A\" \n",
    "penwd := 0.25pt;\n",
    "drawpen := pencircle scaled penwd yscaled aspect_ratio;\n",
    "\tstore (curpath)\n",
    "  drawn polyline(false)\n",
    "\t   ((0.500000, 9.500000),\n",
    "\t   (1.500000, 9.500000),\n",
    "\t   (1.500000, 9.000000));\n",
    "penwd := 0.12pt;\n",
    "drawpen := pencircle scaled penwd yscaled aspect_ratio;\n",
    "\tstore (curpath)\n",
    "  filled circle((1.000000,8.000000),0.250000);\n",
    "endmfpic;\n",
    "end.\n",
);

#[test]
fn options() {
    let opts = [
        ('C', Some("65")),
        ('n', Some("fig")),
        ('x', Some("1")),
        ('X', Some("9")),
        ('t', Some("10")),
    ];
    let (out, _) = mp_common::run("mf", BODY, &opts);
    assert_eq!(out, EXPECTED_OPTS);
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test mf -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    if let Some((same, total)) = mp_common::compare(
        "mf",
        &mp_common::HAND,
        &[
            &[],
            &[
                ('C', Some("65.5")),
                ('n', Some("name")),
                ('p', Some("2")),
                ('t', Some("10")),
                ('x', Some("1")),
                ('X', Some("9")),
                ('y', Some("0.5")),
                ('Y', Some("9")),
            ],
            &[('m', Some("2"))],
        ],
    ) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
