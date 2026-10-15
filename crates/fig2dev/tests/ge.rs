#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `ge` driver against output of the real fig2dev 3.2.9a.

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
    "#  HREF=\"http://a\" ALT=\"A\" \n",
    "p s30 y02 c05 (127,127) (381,127) w48 (381,254);\n",
    "E s15 y01 c01 C02 (191,445) (318,572)\n",
    "\"a_b\" c01 f00 z12 y1 (508,508) l0 ;\n",
);

#[test]
fn as_fig2dev() {
    let (out, msgs) = mp_common::run("ge", BODY, &[]);
    assert_eq!(out, EXPECTED);
    assert!(msgs.is_empty(), "{msgs:?}");
}

#[test]
fn unknown_option() {
    let mut d = rustfig_fig2dev::drivers::driver("ge").unwrap();
    let mut ctx = rustfig_fig2dev::Ctx::default();
    assert_eq!(
        d.option(&mut ctx, 'a', None).unwrap_err(),
        "Argument -a unknown to ge driver."
    );
    assert!(d.option(&mut ctx, 'G', Some("1")).is_ok());
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test ge -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    if let Some((same, total)) = mp_common::compare("ge", &mp_common::HAND, &[&[], &[('m', Some("2"))]]) {
        // fig2dev reads user colours past the end of its colour table and
        // crashes on interpolated splines: those figures differ.
        assert!(same * 10 >= total * 8, "{same} of {total} identical");
    }
}
