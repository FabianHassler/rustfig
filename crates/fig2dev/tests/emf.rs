#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `emf` driver (binary output): structure checks, hostile values,
//! and an opt-in comparison with the real fig2dev 3.2.9a (`cmp`).

use std::process::Command;

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

const HDR: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";

fn emf(body: &str, opts: &[(char, Option<&str>)]) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut ctx = Ctx {
        lang: "emf".into(),
        from: Some("t.fig".into()),
        ..Ctx::default()
    };
    let mut dev = drivers::driver("emf").unwrap();
    for &(c, a) in opts {
        if !ctx.general_option(c, a)? {
            dev.option(&mut ctx, c, a)?;
        }
    }
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, format!("{HDR}{body}").as_bytes()).map_err(|e| format!("{e:?}"))?;
    let status = convert(&mut ctx, dev.as_mut(), &objects)?;
    Ok((status.output, ctx.messages))
}

fn word(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

const BOX: &str = "2 2 0 2 4 7 50 -1 20 0.000 0 0 -1 0 0 5\n\t 1200 1200 3000 1200 3000 2400 1200 2400 1200 1200\n";

#[test]
fn a_box_makes_a_wellformed_metafile() {
    let (b, msgs) = emf(BOX, &[]).unwrap();
    // header: type 1, signature " EMF", sizes and counts match the file
    assert_eq!(word(&b, 0), 1);
    assert_eq!(word(&b, 40), 0x464D_4520);
    assert_eq!(word(&b, 48) as usize, b.len());
    // walk the records
    let (mut i, mut n) = (0, 0);
    let mut last = 0;
    while i < b.len() {
        last = word(&b, i);
        i += word(&b, i + 4) as usize;
        n += 1;
    }
    assert_eq!(i, b.len());
    assert_eq!(n, word(&b, 52));
    assert_eq!(last, 14, "ends with EMR_EOF");
    // the description: "Converted from t.fig using fig2dev 3.2.9a for winnt"
    assert_eq!(&b[88..92], b"C\0o\0");
    assert!(msgs.iter().all(|m| !m.contains("Unsupported")), "{msgs:?}");
}

#[test]
fn unknown_option_and_level() {
    let mut ctx = Ctx::default();
    let mut dev = drivers::driver("emf").unwrap();
    assert_eq!(
        dev.option(&mut ctx, 'q', None).unwrap_err(),
        "Argument -q unknown to emf driver."
    );
    dev.option(&mut ctx, 'l', Some("os2")).unwrap();
    assert_eq!(ctx.messages, ["warning: unknown level os2 ignored"]);
}

#[test]
fn hostile_values_dont_panic() {
    let bodies = [
        "2 1 1 1 0 7 50 -1 -1 0.001 0 0 -1 1 1 2
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 0 0 2147483647 -2147483648
2 2 3 1 0 7 50 -1 -1 0.010 0 0 -1 0 0 5
	 -2147483648 0 2147483647 0 2147483647 9 0 9 0 0
2 4 1 1 0 7 50 -1 -1 4.0 0 0 0 0 0 5
	 0 0 0 0 0 0 0 0 0 0
1 1 1 1 0 7 50 -1 -1 4.0 1 1e300 0 0 5 5 0 0 0 0
",
        "5 1 2 4 7 2 0 -1 0 0 0 0 1 1 79E6 12003420 79196029 3765 5984 3270 6254 51752685
	1 1 2 120 240
	1 1 2 120 240
5 1 0 1 0 7 50 -1 -1 0.0 0 0 1 0 0.0 0.0 600 0 0 600 -600 0
	1 1 1.0 60.0 0.0
5 1 0 1 0 7 50 -1 -1 0.0 0 0 1 1 0.0 0.0 0 0 0 0 0 0
	14 1 1.0 60.0 0.0
	14 1 1.0 60.0 0.0
",
        "2 1 0 1 0 7 50 -1 -1 0.0 0 0 -1 1 1 1
	14 1 1.0 -60.0 -10.0
	14 0 1.0 60.0 0.0
	 -2147483648 2147483647
2 3 0 1 99 99 50 -1 25 0.0 0 0 -1 0 0 4
	 0 0 100 0 100 100 0 0
2 1 0 1 0 7 50 -1 99 0.0 0 0 -1 0 0 2
	 0 0 100 100
",
        "4 7 0 50 -1 99 -1e9 1e30 4 135 -1e30 2147483647 -2147483648 a\\001
4 1 -5 50 -1 -3 12 -6.0 6 135 450 0 0 \\344\\001
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 2
	0 /dev/null
	 0 0 100 100
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5
	0 /etc/passwd
	 100 100 0 0 0 0 100 100 100 100
",
    ];
    for body in bodies {
        for opts in [
            &[][..],
            &[('r', None)][..],
            &[('l', Some("win95"))][..],
            &[('l', Some("win98")), ('r', None)][..],
        ] {
            let _ = emf(body, opts);
        }
    }
}

/// FNV-1a of the bytes (the golden outputs are fig2dev 3.2.9a's, `cmp`-identical).
fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325, |h, &x| {
        (h ^ u64::from(x)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Walks the records, returns their types.
fn record_types(b: &[u8]) -> Vec<u32> {
    let mut v = Vec::new();
    let mut i = 0;
    while i + 8 <= b.len() {
        v.push(word(b, i));
        i += (word(b, i + 4) as usize).max(8);
    }
    v
}

#[test]
fn texts_as_utf16_with_fonts() {
    let body = "4 0 0 50 -1 0 12 0.0 4 135 450 1200 1200 Hello world\\001\n\
4 1 4 49 -1 17 14 0.5 6 135 900 3000 2400 Centred caf\u{e9}\\001\n\
4 2 2 48 -1 3 10 0.0 2 135 600 4000 3400 Right\\001\n\
4 0 1 50 -1 2 12 1.5708 4 135 450 1200 4000 Bold\\001\n";
    let (b, _) = emf(body, &[]).unwrap();
    let t = record_types(&b);
    // four fonts, four texts (EMR_EXTCREATEFONTINDIRECTW 82, EMR_EXTTEXTOUTW 84)
    assert_eq!(t.iter().filter(|&&r| r == 82).count(), 4);
    assert_eq!(t.iter().filter(|&&r| r == 84).count(), 4);
    // "Hello" as UTF-16LE is in the file; the face name of font 0 is Times
    let utf16 = |s: &str| s.bytes().flat_map(|c| [c, 0]).collect::<Vec<u8>>();
    assert!(b.windows(10).any(|w| w == utf16("Hello").as_slice()));
    assert!(b.windows(10).any(|w| w == utf16("Times").as_slice()));
    // the bytes of an UTF-8 text, each sign-extended: 0xc3 0xa9 -> ffc3 ffa9
    assert!(b.windows(4).any(|w| w == [0xc3, 0xff, 0xa9, 0xff]));
    assert_eq!((fnv(&b), b.len()), (0x3082_460a_4966_708f, 2136));
}

#[test]
fn a_png_picture_is_a_stretchdibits_record() {
    const PNG: [u8; 104] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
        0x00, 0x23, 0x00, 0x00, 0x00, 0x0f, 0x01, 0x03, 0x00, 0x00, 0x00, 0x54, 0x79, 0xe4, 0x57, 0x00, 0x00, 0x00,
        0x06, 0x50, 0x4c, 0x54, 0x45, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xa5, 0xd9, 0x9f, 0xdd, 0x00, 0x00, 0x00,
        0x1d, 0x49, 0x44, 0x41, 0x54, 0x08, 0x99, 0x63, 0xf8, 0x0f, 0x04, 0x0f, 0x18, 0x0e, 0x30, 0x30, 0x30, 0x28,
        0xa0, 0x90, 0x40, 0xe1, 0x7f, 0x64, 0x91, 0x40, 0xd3, 0xc0, 0x24, 0x00, 0x5b, 0xfc, 0x36, 0x28, 0xfe, 0xef,
        0xec, 0x17, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let dir = std::env::temp_dir().join(format!("emf-pic-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("line.png");
    std::fs::write(&file, PNG).unwrap();
    let body = format!(
        "2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t1 {}\n\t 4200 2700 4200 1200 1200 1200 1200 2700 4200 2700\n",
        file.display()
    );
    let r = emf(&body, &[]);
    let _ = std::fs::remove_dir_all(&dir);
    let (b, msgs) = r.unwrap();
    assert!(msgs.iter().all(|m| !m.contains("unsupported")), "{msgs:?}");
    assert_eq!(record_types(&b).iter().filter(|&&r| r == 81).count(), 1);
    assert_eq!((fnv(&b), b.len()), (0x8416_61e5_684e_9b25, 492));
}

#[test]
fn pictures_other_than_png_are_unsupported_and_missing_ones_reported() {
    let body = "2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 /nonexistent/x.gif\n\t 0 0 900 0 900 900 0 900 0 0\n";
    let (b, msgs) = emf(body, &[]).unwrap();
    assert!(msgs.iter().any(|m| m.contains("No such picture file")), "{msgs:?}");
    assert!(!record_types(&b).contains(&81));
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test emf -- --ignored`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let reference = "/opt/homebrew/bin/fig2dev";
    if !std::path::Path::new(reference).exists() {
        return;
    }
    let bodies = [
        BOX,
        "2 1 3 4 4 7 50 -1 -1 5.000 0 2 -1 1 1 3\n\t2 0 1.00 120.00 180.00\n\t3 1 1.00 120.00 180.00\n\t 1200 4800 3000 5200 4000 4800\n",
        "2 4 1 3 4 6 50 -1 41 4.000 0 0 20 0 0 5\n\t 5000 17000 8000 17000 8000 18500 5000 18500 5000 17000\n",
        "5 1 0 3 0 7 50 -1 20 0.000 0 0 1 1 7000.000 3000.000 6000 3000 7000 2000 8000 3000\n\t1 1 1.00 150.00 200.00\n\t1 1 1.00 150.00 200.00\n",
        "1 1 0 2 0 5 50 -1 45 0.000 1 0.5236 11000 13000 1500 800 11000 13000 12500 13000\n",
    ];
    let dir = std::env::temp_dir().join(format!("emf-cmp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (n, body) in bodies.iter().enumerate() {
        for opts in [&[][..], &["-r"][..], &["-l", "win95"][..]] {
            let path = dir.join(format!("t{n}.fig"));
            std::fs::write(&path, format!("{HDR}{body}")).unwrap();
            let out = dir.join(format!("t{n}.emf"));
            let st = Command::new(reference)
                .args(["-L", "emf"])
                .args(opts)
                .arg(&path)
                .arg(&out)
                .output()
                .unwrap();
            assert!(st.status.success());
            let want = std::fs::read(&out).unwrap();
            let rf: Vec<(char, Option<&str>)> = match opts {
                ["-r"] => vec![('r', None)],
                ["-l", l] => vec![('l', Some(*l))],
                _ => vec![],
            };
            let mut ctx = Ctx {
                lang: "emf".into(),
                from: Some(path.to_string_lossy().into()),
                ..Ctx::default()
            };
            let mut dev = drivers::driver("emf").unwrap();
            for (c, a) in rf {
                dev.option(&mut ctx, c, a).unwrap();
            }
            let objects = readfp_fig(&mut ctx, format!("{HDR}{body}").as_bytes()).unwrap();
            let got = convert(&mut ctx, dev.as_mut(), &objects).unwrap().output;
            assert_eq!(got, want, "figure {n} {opts:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
