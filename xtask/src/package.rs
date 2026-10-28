//! `cargo xtask package` (plan P3): installable packages for the host system,
//! without code signing. macOS: `RustFig.app` + `.dmg`; Linux: tarball with
//! install script, `.deb` (needs `dpkg-deb`), desktop file and MIME type;
//! Windows: zip with the exe and a `.reg` file for the `.fig` association.
//! Every file is generated here from text; nothing binary is committed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const BUNDLE_ID: &str = "io.github.fabianhassler.rustfig";
pub const UTI: &str = "org.xfig.fig";

/// The workspace version from the root `Cargo.toml` (`[workspace.package]`).
pub fn workspace_version(manifest: &str) -> String {
    manifest
        .lines()
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("version")
                .and_then(|r| r.trim_start().strip_prefix('='))
                .map(|v| v.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "0.0.0".into())
}

/// macOS `Info.plist`: bundle id, version, icon and the `.fig` document type.
pub fn info_plist(version: &str, with_icon: bool) -> String {
    let icon = if with_icon {
        "\t<key>CFBundleIconFile</key>\n\t<string>RustFig</string>\n"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>RustFig</string>
	<key>CFBundleDisplayName</key>
	<string>RustFig</string>
	<key>CFBundleIdentifier</key>
	<string>{BUNDLE_ID}</string>
	<key>CFBundleExecutable</key>
	<string>rustfig</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleVersion</key>
	<string>{version}</string>
	<key>CFBundleShortVersionString</key>
	<string>{version}</string>
{icon}	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>CFBundleDocumentTypes</key>
	<array>
		<dict>
			<key>CFBundleTypeName</key>
			<string>xfig drawing</string>
			<key>CFBundleTypeRole</key>
			<string>Editor</string>
			<key>LSHandlerRank</key>
			<string>Default</string>
			<key>LSItemContentTypes</key>
			<array>
				<string>{UTI}</string>
			</array>
		</dict>
	</array>
	<key>UTExportedTypeDeclarations</key>
	<array>
		<dict>
			<key>UTTypeIdentifier</key>
			<string>{UTI}</string>
			<key>UTTypeDescription</key>
			<string>xfig drawing</string>
			<key>UTTypeConformsTo</key>
			<array>
				<string>public.data</string>
			</array>
			<key>UTTypeTagSpecification</key>
			<dict>
				<key>public.filename-extension</key>
				<array>
					<string>fig</string>
				</array>
				<key>public.mime-type</key>
				<string>image/x-xfig</string>
			</dict>
		</dict>
	</array>
</dict>
</plist>
"#
    )
}

/// Freedesktop `.desktop` entry.
pub fn desktop_file() -> String {
    "[Desktop Entry]\nType=Application\nName=RustFig\nGenericName=Vector drawing\n\
     Comment=Native reimplementation of xfig\nExec=rustfig %f\nIcon=rustfig\n\
     Terminal=false\nCategories=Graphics;2DGraphics;VectorGraphics;\n\
     MimeType=image/x-xfig;\nStartupWMClass=rustfig\n"
        .into()
}

/// shared-mime-info type for `*.fig` (xfig's own magic line `#FIG`).
pub fn mime_xml() -> String {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="image/x-xfig">
    <comment>xfig drawing</comment>
    <alias type="application/x-xfig"/>
    <alias type="text/x-xfig"/>
    <magic priority="50">
      <match type="string" offset="0" value="#FIG"/>
    </magic>
    <glob pattern="*.fig"/>
  </mime-type>
</mime-info>
"##
    .into()
}

/// Windows registry file associating `.fig` with `rustfig.exe` (per user).
/// `exe` is the install path; backslashes are doubled for the `.reg` syntax.
pub fn reg_file(exe: &str) -> String {
    let e = exe.replace('\\', "\\\\");
    format!(
        "Windows Registry Editor Version 5.00\r\n\r\n\
         [HKEY_CURRENT_USER\\Software\\Classes\\.fig]\r\n@=\"RustFig.Drawing\"\r\n\r\n\
         [HKEY_CURRENT_USER\\Software\\Classes\\RustFig.Drawing]\r\n@=\"xfig drawing\"\r\n\r\n\
         [HKEY_CURRENT_USER\\Software\\Classes\\RustFig.Drawing\\DefaultIcon]\r\n@=\"{e},0\"\r\n\r\n\
         [HKEY_CURRENT_USER\\Software\\Classes\\RustFig.Drawing\\shell\\open\\command]\r\n@=\"\\\"{e}\\\" \\\"%1\\\"\"\r\n"
    )
}

/// Debian `control` file.
pub fn deb_control(version: &str, arch: &str, size_kb: u64) -> String {
    format!(
        "Package: rustfig\nVersion: {version}\nSection: graphics\nPriority: optional\n\
         Architecture: {arch}\nInstalled-Size: {size_kb}\n\
         Maintainer: Fabian Hassler <hassler@physik.rwth-aachen.de>\n\
         Depends: libc6, libgtk-3-0 | zenity\n\
         Description: Rust-native reimplementation of xfig\n \
         RustFig opens and saves Fig files and exports with the ported fig2dev drivers.\n"
    )
}

fn install_script() -> String {
    "#!/bin/sh\n# Installs RustFig for the current user (or system-wide with PREFIX=/usr/local, as root).\n\
     set -e\nPREFIX=${PREFIX:-$HOME/.local}\nd=$(dirname \"$0\")\n\
     install -Dm755 \"$d/rustfig\" \"$PREFIX/bin/rustfig\"\n\
     # fig2dev under its own name only where no fig2dev is installed\n\
     command -v fig2dev >/dev/null 2>&1 || ln -sf rustfig \"$PREFIX/bin/fig2dev\"\n\
     install -Dm644 \"$d/rustfig.desktop\" \"$PREFIX/share/applications/rustfig.desktop\"\n\
     install -Dm644 \"$d/rustfig.xml\" \"$PREFIX/share/mime/packages/rustfig.xml\"\n\
     [ -f \"$d/rustfig.png\" ] && install -Dm644 \"$d/rustfig.png\" \"$PREFIX/share/icons/hicolor/256x256/apps/rustfig.png\"\n\
     update-mime-database \"$PREFIX/share/mime\" 2>/dev/null || true\n\
     update-desktop-database \"$PREFIX/share/applications\" 2>/dev/null || true\n\
     echo \"RustFig installed in $PREFIX/bin\"\n"
        .into()
}

// ---- PNG without a compression library (stored deflate blocks) ----

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// RGBA pixels (`size` square) as an uncompressed PNG.
pub fn png_from_rgba(rgba: &[u8], size: u32) -> Option<Vec<u8>> {
    let row = size as usize * 4;
    if size == 0 || rgba.len() != row * size as usize {
        return None;
    }
    let mut raw = Vec::with_capacity(rgba.len() + size as usize);
    for r in rgba.chunks(row) {
        raw.push(0);
        raw.extend_from_slice(r);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        let n = b.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(b);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    Some(out)
}

// ---- driver ----

fn sh(cmd: &mut Command) -> Result<(), String> {
    eprintln!("$ {cmd:?}");
    let s = cmd.status().map_err(|e| format!("{cmd:?}: {e}"))?;
    if s.success() {
        Ok(())
    } else {
        Err(format!("command failed: {cmd:?}"))
    }
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The app icon's PNG at `size`, asked of the built `rustfig` (`--dump-icon`).
fn icon_png(exe: &Path, size: u32) -> Option<Vec<u8>> {
    let out = Command::new(exe)
        .arg("--dump-icon")
        .arg(size.to_string())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    png_from_rgba(&out.stdout, size)
}

fn copy_docs(root: &Path, dir: &Path) -> Result<(), String> {
    for f in [
        "README.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "LICENSE-xfig",
        "NOTICE",
        "ASSETS.md",
    ] {
        fs::copy(root.join(f), dir.join(f)).map_err(|e| format!("{f}: {e}"))?;
    }
    Ok(())
}

pub fn run(root: &Path, mut cargo: Command) -> Result<(), String> {
    let version = workspace_version(&fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default());
    sh(cargo
        .current_dir(root)
        .args(["build", "--release", "-p", "rustfig", "-p", "rustfig-cli"]))?;
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let rel = target.join("release");
    let out = target.join("package");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    if cfg!(target_os = "macos") {
        macos(root, &rel, &out, &version)
    } else if cfg!(target_os = "windows") {
        windows(root, &rel, &out, &version)
    } else {
        linux(root, &rel, &out, &version)
    }?;
    eprintln!("packages in {}", out.display());
    Ok(())
}

fn macos(root: &Path, rel: &Path, out: &Path, version: &str) -> Result<(), String> {
    let exe = rel.join("rustfig");
    let app = out.join("RustFig.app");
    let (macos_dir, res) = (app.join("Contents/MacOS"), app.join("Contents/Resources"));
    fs::create_dir_all(&macos_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&res).map_err(|e| e.to_string())?;
    fs::copy(&exe, macos_dir.join("rustfig")).map_err(|e| format!("rustfig: {e}"))?;
    #[cfg(unix)]
    std::os::unix::fs::symlink("rustfig", macos_dir.join("fig2dev")).map_err(|e| e.to_string())?;
    // Icon: iconset of PNGs from the app's own drawing, then iconutil.
    let set = out.join("RustFig.iconset");
    let mut icon = false;
    if Command::new("iconutil").arg("--help").output().is_ok() {
        fs::create_dir_all(&set).map_err(|e| e.to_string())?;
        icon = true;
        for (name, px) in [
            ("icon_16x16", 16),
            ("icon_16x16@2x", 32),
            ("icon_32x32", 32),
            ("icon_32x32@2x", 64),
            ("icon_128x128", 128),
            ("icon_128x128@2x", 256),
            ("icon_256x256", 256),
            ("icon_256x256@2x", 512),
            ("icon_512x512", 512),
            ("icon_512x512@2x", 1024),
        ] {
            match icon_png(&exe, px) {
                Some(png) => fs::write(set.join(format!("{name}.png")), png).map_err(|e| e.to_string())?,
                None => {
                    icon = false;
                    break;
                }
            }
        }
        if icon {
            icon = sh(Command::new("iconutil")
                .args(["-c", "icns", "-o"])
                .arg(res.join("RustFig.icns"))
                .arg(&set))
            .is_ok();
        }
        if icon {
            let _ = fs::remove_dir_all(&set);
        }
    }
    if !icon {
        eprintln!("warning: no app icon (iconutil missing or the icon could not be rendered)");
    }
    write(&app.join("Contents/Info.plist"), &info_plist(version, icon))?;
    // Sign the whole bundle ad hoc (no certificate): the linker signed only
    // the binary, and a bundle whose signature misses its resources is
    // reported as "damaged" by Gatekeeper after a download.
    sh(Command::new("codesign")
        .args(["--force", "--deep", "--sign", "-"])
        .arg(&app))?;
    let dmg = out.join(format!("RustFig-{version}-macos.dmg"));
    let stage = out.join("dmg");
    fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    sh(Command::new("cp").arg("-R").arg(&app).arg(&stage))?;
    #[cfg(unix)]
    std::os::unix::fs::symlink("/Applications", stage.join("Applications")).map_err(|e| e.to_string())?;
    copy_docs(root, &stage)?;
    sh(Command::new("hdiutil")
        .args([
            "create",
            "-volname",
            "RustFig",
            "-fs",
            "HFS+",
            "-format",
            "UDZO",
            "-ov",
            "-srcfolder",
        ])
        .arg(&stage)
        .arg(&dmg))?;
    let _ = fs::remove_dir_all(&stage);
    Ok(())
}

fn linux(root: &Path, rel: &Path, out: &Path, version: &str) -> Result<(), String> {
    let exe = rel.join("rustfig");
    // The architecture in the name: x86-64 and ARM64 builds go into one release.
    let name = format!(
        "rustfig-{version}-linux-{}",
        if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x86_64"
        }
    );
    let dir = out.join(&name);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::copy(&exe, dir.join("rustfig")).map_err(|e| format!("rustfig: {e}"))?;
    write(&dir.join("rustfig.desktop"), &desktop_file())?;
    write(&dir.join("rustfig.xml"), &mime_xml())?;
    write(&dir.join("install.sh"), &install_script())?;
    let png = icon_png(&exe, 256);
    match &png {
        Some(p) => fs::write(dir.join("rustfig.png"), p).map_err(|e| e.to_string())?,
        None => eprintln!("warning: no app icon"),
    }
    copy_docs(root, &dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for f in ["rustfig", "install.sh"] {
            let p = dir.join(f);
            let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o755));
        }
    }
    sh(Command::new("tar")
        .arg("czf")
        .arg(out.join(format!("{name}.tar.gz")))
        .arg("-C")
        .arg(out)
        .arg(&name))?;
    // .deb
    if Command::new("dpkg-deb").arg("--version").output().is_err() {
        eprintln!("warning: dpkg-deb not found, no .deb");
        return Ok(());
    }
    let pkg = out.join("deb");
    let size_kb = fs::metadata(&exe).map(|m| m.len() / 1024).unwrap_or(0);
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "amd64"
    };
    write(&pkg.join("DEBIAN/control"), &deb_control(version, arch, size_kb))?;
    let bin = pkg.join("usr/bin");
    fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    fs::copy(&exe, bin.join("rustfig")).map_err(|e| e.to_string())?;
    // No /usr/bin/fig2dev: Debian's fig2dev package owns that name (and
    // xfig depends on it); `rustfig fig2dev …` runs the built-in one.
    write(&pkg.join("usr/share/applications/rustfig.desktop"), &desktop_file())?;
    write(&pkg.join("usr/share/mime/packages/rustfig.xml"), &mime_xml())?;
    if let Some(p) = png {
        let d = pkg.join("usr/share/icons/hicolor/256x256/apps");
        fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        fs::write(d.join("rustfig.png"), p).map_err(|e| e.to_string())?;
    }
    let doc = pkg.join("usr/share/doc/rustfig");
    fs::create_dir_all(&doc).map_err(|e| e.to_string())?;
    copy_docs(root, &doc)?;
    sh(Command::new("dpkg-deb")
        .args(["--root-owner-group", "--build"])
        .arg(&pkg)
        .arg(out.join(format!("rustfig_{version}_{arch}.deb"))))?;
    let _ = fs::remove_dir_all(&pkg);
    Ok(())
}

fn windows(root: &Path, rel: &Path, out: &Path, version: &str) -> Result<(), String> {
    let name = format!("rustfig-{version}-windows");
    let dir = out.join(&name);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::copy(rel.join("rustfig.exe"), dir.join("rustfig.exe")).map_err(|e| format!("rustfig.exe: {e}"))?;
    fs::copy(rel.join("rustfig-cli.exe"), dir.join("rustfig-cli.exe")).map_err(|e| format!("rustfig-cli.exe: {e}"))?;
    // The .reg points at C:\Program Files\RustFig; edit the path if installed elsewhere.
    write(
        &dir.join("fig-association.reg"),
        &reg_file(r"C:\Program Files\RustFig\rustfig.exe"),
    )?;
    write(
        &dir.join("INSTALL.txt"),
        "Copy this folder to C:\\Program Files\\RustFig (or anywhere), then double-click\r\n\
         fig-association.reg to open .fig files with RustFig (edit the path inside first\r\n\
         if you chose another folder). The program is not code-signed: Windows SmartScreen\r\n\
         may ask for confirmation on the first start.\r\n",
    )?;
    copy_docs(root, &dir)?;
    let zip = out.join(format!("{name}.zip"));
    // tar (bsdtar) ships with Windows 10+ and writes zip with -a.
    sh(Command::new("tar")
        .arg("-a")
        .arg("-c")
        .arg("-f")
        .arg(&zip)
        .arg("-C")
        .arg(out)
        .arg(&name))
}

#[cfg(test)]
mod tests {
    /// The Scoop and winget manifests in `packaging/` name the workspace
    /// version (bump them with the release; their hashes are the release
    /// zip's SHA-256).
    #[test]
    fn windows_package_manifests_follow_the_version() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let v = super::workspace_version(&std::fs::read_to_string(root.join("Cargo.toml")).unwrap());
        let scoop = std::fs::read_to_string(root.join("packaging/scoop/rustfig.json")).unwrap();
        assert!(
            scoop.contains(&format!("\"version\": \"{v}\"")),
            "packaging/scoop/rustfig.json is not {v}"
        );
        assert!(scoop.contains(&format!("/v{v}/rustfig-{v}-windows.zip")));
        let dir = root.join(format!("packaging/winget/FabianHassler.RustFig/{v}"));
        for f in ["", ".installer", ".locale.en-US"] {
            let text = std::fs::read_to_string(dir.join(format!("FabianHassler.RustFig{f}.yaml")))
                .unwrap_or_else(|e| panic!("winget manifests for {v}: {e}"));
            assert!(text.contains(&format!("PackageVersion: {v}\n")), "{f}");
        }
    }

    use super::*;

    #[test]
    fn plist_has_document_type_and_id() {
        let p = info_plist("1.2.3", true);
        assert!(p.contains(BUNDLE_ID) && p.contains("CFBundleDocumentTypes"));
        assert!(p.contains("<string>fig</string>") && p.contains("<string>Editor</string>"));
        assert!(p.contains("1.2.3") && p.contains("CFBundleIconFile"));
        assert!(!info_plist("1", false).contains("CFBundleIconFile"));
    }

    #[test]
    fn linux_files() {
        assert!(desktop_file().contains("MimeType=image/x-xfig;"));
        assert!(mime_xml().contains("*.fig"));
        let c = deb_control("0.1.0", "amd64", 10);
        assert!(c.starts_with("Package: rustfig\nVersion: 0.1.0\n") && c.ends_with('\n'));
    }

    #[test]
    fn reg_escapes_paths() {
        let r = reg_file(r"C:\x\rustfig.exe");
        assert!(r.contains(r#"@="\"C:\\x\\rustfig.exe\" \"%1\"""#));
    }

    #[test]
    fn version_and_png() {
        assert_eq!(workspace_version("[workspace.package]\nversion = \"0.4.2\"\n"), "0.4.2");
        let png = png_from_rgba(&[255; 2 * 2 * 4], 2).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        assert!(png_from_rgba(&[0; 3], 2).is_none());
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }
}
