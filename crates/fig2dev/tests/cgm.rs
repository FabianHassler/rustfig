#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `cgm` driver against output of the real fig2dev 3.2.9a
//! (`fig2dev -L cgm < fig`).

mod tex_common;
use tex_common::{compare_with_fig2dev, run_with, HAND};

const FIG: &str = concat!(
    "#FIG 3.2\n",
    "Landscape\n",
    "Center\n",
    "Inches\n",
    "Letter\n",
    "100.00\n",
    "Single\n",
    "-2\n",
    "1200 2\n",
    "2 1 1 2 4 7 50 -1 -1 4.000 0 0 -1 1 0 3\n",
    "\t1 1 1.00 60.00 120.00\n",
    "\t 600 600 1800 600 1800 1200\n",
    "2 2 0 1 0 1 40 -1 30 0.000 0 0 -1 0 0 5\n",
    "\t 2400 600 3600 600 3600 1200 2400 1200 2400 600\n",
    "1 3 0 1 0 7 30 -1 -1 0.000 1 0.0000 1200 2400 300 300 1200 2400 1500 2400\n",
    "4 1 0 20 -1 16 12 0.0000 4 135 450 2400 2400 It's\\001\n",
);
const CGM: &str = concat!(
    "BEGMF '(stdin)';\n",
    "mfversion 1;\n",
    "mfdesc 'Converted from (stdin) using fig2dev -Lcgm';\n",
    "mfelemlist 'DRAWINGPLUS';\n",
    "vdctype integer;\n",
    "fontlist 'Hardware',\n",
    "  'Times New Roman', 'Times New Roman Bold', 'Times New Roman Italic',\n",
    "  'Helvetica', 'Courier',\n",
    "  'Times-Roman', 'Times-Italic',\n",
    "  'Times-Bold', 'Times-BoldItalic',\n",
    "  'AvantGarde-Book', 'AvantGarde-BookOblique',\n",
    "  'AvantGarde-Demi', 'AvantGarde-DemiOblique',\n",
    "  'Bookman-light', 'Bookman-lightItalic',\n",
    "  'Bookman-Demi', 'Bookman-DemiItalic',\n",
    "  'Courier', 'Courier-Oblique',\n",
    "  'Courier-Bold', 'Courier-BoldOblique',\n",
    "  'Helvetica', 'Helvetica-Oblique',\n",
    "  'Helvetica-Bold', 'Helvetica-BoldOblique',\n",
    "  'Helvetica-Narrow', 'Helvetica-Narrow-Oblique',\n",
    "  'Helvetica-Narrow-Bold', 'Helvetica-Narrow-BoldOblique',\n",
    "  'NewCenturySchlbk-Roman', 'NewCenturySchlbk-Italic',\n",
    "  'NewCenturySchlbk-Bold', 'NewCenturySchlbk-BoldItalic',\n",
    "  'Palatino-Roman', 'Palatino-Italic',\n",
    "  'Palatino-Bold', 'Palatino-BoldItalic',\n",
    "  'Symbol', 'ZapfChancery-MediumItalic', 'ZapfDingbats';\n",
    "BEGMFDEFAULTS;\n",
    "  vdcext (0,0) (3034,2129);\n",
    "  clip off;\n",
    "  colrmode indexed;\n",
    "  colrtable 1\n",
    "\t0 0 0\n",
    "\t0 0 255\n",
    "\t0 255 0\n",
    "\t0 255 255\n",
    "\t255 0 0\n",
    "\t255 0 255\n",
    "\t255 255 0\n",
    "\t255 255 255\n",
    "\t0 0 142\n",
    "\t0 0 175\n",
    "\t0 0 209\n",
    "\t135 206 255\n",
    "\t0 142 0\n",
    "\t0 175 0\n",
    "\t0 209 0\n",
    "\t0 142 142\n",
    "\t0 175 175\n",
    "\t0 209 209\n",
    "\t142 0 0\n",
    "\t175 0 0\n",
    "\t209 0 0\n",
    "\t142 0 142\n",
    "\t175 0 175\n",
    "\t209 0 209\n",
    "\t127 48 0\n",
    "\t160 63 0\n",
    "\t191 96 0\n",
    "\t255 127 127\n",
    "\t255 160 160\n",
    "\t255 191 191\n",
    "\t255 224 224\n",
    "\t255 214 0;\n",
    "  linewidthmode abs;\n",
    "  edgewidthmode abs;\n",
    "  backcolr 255 255 255;\n",
    "  textprec stroke;\n",
    "  transparency ON;\n",
    "ENDMFDEFAULTS;\n",
    "BEGPIC '(stdin)';\n",
    "BEGPICBODY;\n",
    "% Polyline %\n",
    "linetype 2;\n",
    "linewidth 30;\n",
    "linecolr 5;\n",
    "line (22,2107) (1222,2107) (1222,1627);\n",
    "intstyle SOLID;\n",
    "fillcolr 5;\n",
    "edgevis ON;\n",
    "edgetype 1;\n",
    "edgewidth 15;\n",
    "edgecolr 5;\n",
    "polygon (1252,1657)(1222,1537)(1192,1657);\n",
    "% Box %\n",
    "colrtable 33 128 128 255;\n",
    "fillcolr 33;\n",
    "edgecolr 1;\n",
    "rect (1822,2107) (3022,1507);\n",
    "% Circle %\n",
    "intstyle EMPTY;\n",
    "circle (622,307) 300;\n",
    "% Text %\n",
    "textfontindex 23;\n",
    "textalign ctr base 0.0 0.0;\n",
    "textcolr 1;\n",
    "charheight 120;\n",
    "charori (0,1200) (1200,0);\n",
    "text (1822,307) final 'It''s';\n",
    "% End of Picture %\n",
    "ENDPIC;\n",
    "ENDMF;\n",
);

fn cgm(opts: &[(char, Option<&str>)]) -> Result<(String, Vec<String>, i32), String> {
    run_with("cgm", FIG.as_bytes(), None, opts, false)
}

#[test]
fn polyline_arrow_box_circle_text() {
    let (out, msgs, exit) = cgm(&[]).unwrap();
    assert_eq!(out, CGM);
    assert!(msgs.is_empty(), "{msgs:?}");
    assert_eq!(exit, 0);
}

#[test]
fn rounded_arrows_and_reset() {
    // -r: the arrow isn't moved back by the line width.
    let (out, _, _) = cgm(&[('r', None)]).unwrap();
    let expected = CGM.replace("(1222,1627);", "(1222,1597);").replace(
        "polygon (1252,1657)(1222,1537)(1192,1657);",
        "polygon (1252,1627)(1222,1507)(1192,1627);",
    );
    assert_eq!(out, expected);
    // fig2dev resets -r with every later option.
    let (out, _, _) = cgm(&[('r', None), ('G', Some("1"))]).unwrap();
    assert_eq!(out, CGM);
}

#[test]
fn options() {
    assert_eq!(cgm(&[('q', None)]).unwrap_err(), "Argument -q unknown to cgm driver.");
    // -a: no pipe to ralcgm; clear text, as when fig2dev can't open it.
    let (out, msgs, _) = cgm(&[('a', None)]).unwrap();
    assert_eq!(out, CGM);
    assert_eq!(
        msgs[0],
        "fig2dev: Can't open pipe to ralcgm, producing ASCII CGM instead."
    );
    assert_eq!(msgs[1], "Command was: ralcgm -b - -");
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test cgm -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let opts: &[&[&str]] = &[&[], &["-r"]];
    if let Some((same, total)) = compare_with_fig2dev("cgm", &HAND, opts) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
