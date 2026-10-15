#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The picture readers on fig2dev's own test pictures (old/, skipped when
//! absent): the same 35 × 15 line drawing as GIF, JPEG, PCX, PNG, PPM,
//! TIFF, XBM and XPM, each gzipped, plus EPS and PDF.

use std::path::PathBuf;

use rustfig_fig2dev::object::{Pic, Point, P_EPS, P_PCX, P_PNG, P_XBM};
use rustfig_fig2dev::util::readpics::{
    open_stream, pic_type, read_bitmap_file, read_pic, uncompress_lzw, Compression, PicType, RGB_NUMCOLS,
};
use rustfig_fig2dev::Ctx;

fn data_dir() -> Option<PathBuf> {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../old/fig2dev-3.2.9a/fig2dev/tests/data");
    d.join("line.png.gz").exists().then_some(d)
}

fn ctx() -> Option<Ctx> {
    let dir = data_dir()?;
    Some(Ctx {
        dir: Some(dir),
        ..Ctx::default()
    })
}

/// The colour of each pixel.
fn pixels(pic: &Pic) -> Vec<[u8; 3]> {
    if pic.numcols > 256 {
        pic.bitmap.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()
    } else {
        pic.bitmap.iter().map(|&i| pic.cmap[usize::from(i)]).collect()
    }
}

fn read(ctx: &Ctx, name: &str) -> Pic {
    let stream = open_stream(ctx, name).unwrap_or_else(|e| panic!("{name}: {e}"));
    let mut pic = Pic {
        file: name.into(),
        ..Pic::default()
    };
    read_pic(ctx, &stream, &mut pic).unwrap_or_else(|e| panic!("{name}: {e}"));
    pic
}

#[test]
fn compressed_pictures_are_found_and_read() {
    let Some(ctx) = ctx() else {
        eprintln!("skipped: fig2dev's test data not present");
        return;
    };
    // "line.png" is found as line.png.gz, as fig2dev does.
    let s = open_stream(&ctx, "line.png").unwrap();
    assert_eq!(s.compression, Compression::Gzip);
    assert_eq!(pic_type(&s.data), Some(PicType::Png));
    // Asking for the compressed name works too.
    assert!(open_stream(&ctx, "line.png.gz").is_ok());

    let png = read(&ctx, "line.png");
    assert_eq!(png.subtype, P_PNG);
    assert_eq!(png.bit_size, Point::new(35, 15));
    assert!((png.hw_ratio - 15.0 / 35.0).abs() < 1e-6);
    let reference = pixels(&png);
    assert!(reference.contains(&[0, 0, 0]) && reference.contains(&[255, 255, 255]));

    // The lossless formats give the same pixels.
    for name in ["line.gif", "line.pcx", "line.ppm", "line.tif", "line.xpm"] {
        let pic = read(&ctx, name);
        assert_eq!(pic.bit_size, Point::new(35, 15), "{name}");
        assert!(pic.numcols <= 256 || pic.numcols == RGB_NUMCOLS, "{name}");
        assert_eq!(pixels(&pic), reference, "{name}");
    }
    assert_eq!(read(&ctx, "line.pcx").subtype, P_PCX);

    // JPEG: the size, and roughly the same picture.
    let jpg = read(&ctx, "line.jpg");
    assert_eq!(jpg.bit_size, Point::new(35, 15));
    let close = pixels(&jpg)
        .iter()
        .zip(&reference)
        .filter(|(a, b)| a[0].abs_diff(b[0]) < 64)
        .count();
    assert!(close > reference.len() * 9 / 10);

    // XBM: packed bits, set where the picture is black.
    let xbm = read(&ctx, "line.xbm");
    assert_eq!(xbm.subtype, P_XBM);
    assert_eq!(xbm.numcols, 0);
    assert_eq!(xbm.bit_size, Point::new(35, 15));
    let row = 35usize.div_ceil(8);
    assert_eq!(xbm.bitmap.len(), row * 15);
    for y in 0..15 {
        for x in 0..35 {
            let bit = xbm.bitmap[y * row + x / 8] & (0x80 >> (x % 8)) != 0;
            assert_eq!(bit, reference[y * 35 + x] == [0, 0, 0], "xbm pixel {x},{y}");
        }
    }
    let s = open_stream(&ctx, "line.xbm").unwrap();
    assert_eq!(read_bitmap_file(&s.data).unwrap().data, xbm.bitmap);

    // EPS and PDF: the bounding box.
    let eps = read(&ctx, "line.eps");
    assert_eq!(eps.subtype, P_EPS);
    assert_eq!(eps.bit_size, Point::new(31, 13));
    let pdf = read(&ctx, "line.pdf");
    assert_eq!(pdf.subtype, P_EPS);
    assert_eq!(pdf.bit_size, Point::new(31, 13));
}

#[test]
fn missing_and_junk_pictures_are_errors() {
    let dir = std::env::temp_dir().join(format!("rustfig-fig2dev-util-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ctx = Ctx {
        dir: Some(dir.clone()),
        ..Ctx::default()
    };
    assert!(open_stream(&ctx, "nonexistent.png").is_err());

    std::fs::write(dir.join("junk.png"), b"not a picture at all").unwrap();
    let s = open_stream(&ctx, "junk.png").unwrap();
    let mut pic = Pic::default();
    assert!(read_pic(&ctx, &s, &mut pic).is_err());

    std::fs::write(dir.join("bad.gif.gz"), b"\x1f\x8b\x08\x00garbage").unwrap();
    assert!(open_stream(&ctx, "bad.gif").is_err());

    std::fs::write(
        dir.join("trunc.png"),
        b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\xff\xff\0\0\xff\xff",
    )
    .unwrap();
    let s = open_stream(&ctx, "trunc.png").unwrap();
    assert!(read_pic(&ctx, &s, &mut pic).is_err());

    std::fs::write(dir.join("x.bz2"), b"BZh").unwrap();
    assert!(open_stream(&ctx, "x").unwrap_err().contains("bzip2"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `.Z` files: compare with the system's `compress`, where there is one.
#[test]
fn lzw_matches_compress() {
    let dir = std::env::temp_dir().join(format!("rustfig-fig2dev-lzw-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // Enough varied data to need wider codes and, with 9 bits, clear codes.
    let mut seed = 7u32;
    let text: Vec<u8> = (0..200_000)
        .map(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            b"abcdefghij \n0123456789"[(seed >> 16) as usize % 22]
        })
        .collect();
    let file = dir.join("data.txt");
    for bits in ["9", "12", "16"] {
        std::fs::write(&file, &text).unwrap();
        let ok = std::process::Command::new("compress")
            .args(["-f", "-b", bits])
            .arg(&file)
            .status();
        let Ok(status) = ok else {
            eprintln!("skipped: no compress program");
            break;
        };
        assert!(status.success());
        let z = std::fs::read(dir.join("data.txt.Z")).unwrap();
        assert_eq!(uncompress_lzw(&z).unwrap(), text, "{bits} bits");
        let ctx = Ctx {
            dir: Some(dir.clone()),
            ..Ctx::default()
        };
        assert_eq!(open_stream(&ctx, "data.txt").unwrap().data, text);
        std::fs::remove_file(dir.join("data.txt.Z")).unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
}
