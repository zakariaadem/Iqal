//! Build helper for Iqal's Windows programs: gives the .exe Iqal's icon
//! and a version description (the name Task Manager, Explorer and
//! Windows' security prompts show). Writes a compiled resource file, which
//! Microsoft's linker takes directly, so no resource compiler is needed.
//!
//! Use from a build script: `iqal_winres::embed("Iqal service", "iqal-daemon.exe")`. Does
//! nothing when not building for Windows with Microsoft's tools.

use std::path::PathBuf;

/// Iqal's icon at the sizes there are pictures for (PNG, which Windows
/// accepts inside icons).
const PICTURES: [(u32, &[u8]); 5] = [
    (16, include_bytes!("../../../extension/icon-16.png")),
    (32, include_bytes!("../../../extension/icon-32.png")),
    (48, include_bytes!("../../../extension/icon-48.png")),
    (96, include_bytes!("../../../extension/icon-96.png")),
    (128, include_bytes!("../../../extension/icon-128.png")),
];

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
/// US English, the language the version texts are in.
const LANGUAGE: u16 = 0x0409;

/// Embeds the icon (resource 1, so `LoadIconW(instance, 1)` finds it) and
/// version information describing the program `file_name` as `description`.
pub fn embed(description: &str, file_name: &str) {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let res = resources(description, &version, file_name);
    let path = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("iqal.res");
    std::fs::write(&path, res).expect("writing the resource file");
    println!("cargo:rustc-link-arg-bins={}", path.display());
}

/// The whole resource file.
pub fn resources(description: &str, version: &str, file_name: &str) -> Vec<u8> {
    // A resource file starts with an empty entry.
    let mut out = entry(0, 0, 0, &[]);
    let mut group = Vec::new();
    group.extend(0u16.to_le_bytes()); // reserved
    group.extend(1u16.to_le_bytes()); // icons
    group.extend((PICTURES.len() as u16).to_le_bytes());
    for (i, (size, png)) in PICTURES.iter().enumerate() {
        let id = i as u16 + 1;
        out.extend(entry(RT_ICON, id, LANGUAGE, png));
        // 256 pixels is written as 0.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        group.extend([side, side, 0, 0]);
        group.extend(1u16.to_le_bytes()); // planes
        group.extend(32u16.to_le_bytes()); // bits per pixel
        group.extend((png.len() as u32).to_le_bytes());
        group.extend(id.to_le_bytes());
    }
    out.extend(entry(RT_GROUP_ICON, 1, LANGUAGE, &group));
    out.extend(entry(RT_VERSION, 1, LANGUAGE, &version_info(description, version, file_name)));
    out
}

fn pad4(v: &mut Vec<u8>) {
    while v.len() % 4 != 0 {
        v.push(0);
    }
}

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect()
}

/// One resource: its header (type and name as numbers), then its data.
fn entry(kind: u16, name: u16, language: u16, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend((data.len() as u32).to_le_bytes());
    v.extend(32u32.to_le_bytes()); // header size
    v.extend([0xff, 0xff]);
    v.extend(kind.to_le_bytes());
    v.extend([0xff, 0xff]);
    v.extend(name.to_le_bytes());
    v.extend(0u32.to_le_bytes()); // data version
    // Moveable, pure; icons may also be discarded.
    let flags: u16 = if kind == RT_ICON || kind == RT_GROUP_ICON { 0x1030 } else { 0x0030 };
    v.extend(flags.to_le_bytes());
    v.extend(language.to_le_bytes());
    v.extend(0u32.to_le_bytes()); // version
    v.extend(0u32.to_le_bytes()); // characteristics
    v.extend(data);
    pad4(&mut v);
    v
}

/// A block of version information: its key, value and child blocks.
/// `value_length` counts characters for text values, bytes otherwise.
fn block(key: &str, value: &[u8], value_length: usize, text: bool, children: &[Vec<u8>]) -> Vec<u8> {
    let mut v = vec![0u8; 6];
    v.extend(utf16z(key));
    pad4(&mut v);
    v.extend(value);
    for child in children {
        pad4(&mut v);
        v.extend(child);
    }
    let length = v.len() as u16;
    v[0..2].copy_from_slice(&length.to_le_bytes());
    v[2..4].copy_from_slice(&(value_length as u16).to_le_bytes());
    v[4..6].copy_from_slice(&u16::from(text).to_le_bytes());
    v
}

fn string(key: &str, value: &str) -> Vec<u8> {
    block(key, &utf16z(value), value.encode_utf16().count() + 1, true, &[])
}

/// "1.2.3" as the two numbers Windows keeps versions in.
fn version_numbers(version: &str) -> (u32, u32) {
    let mut parts = version.split(['.', '-', '+']).map(|p| p.parse::<u32>().unwrap_or(0));
    let mut next = || parts.next().unwrap_or(0) & 0xffff;
    let (a, b, c) = (next(), next(), next());
    (a << 16 | b, c << 16)
}

fn version_info(description: &str, version: &str, file_name: &str) -> Vec<u8> {
    let (ms, ls) = version_numbers(version);
    let mut fixed = Vec::new();
    for value in [
        0xFEEF_04BDu32, // signature
        0x0001_0000,    // structure version
        ms,             // file version
        ls,
        ms, // product version
        ls,
        0x3f,       // flags that are set below
        0,          // flags
        0x0004_0004, // for Windows NT
        1,          // an application
        0,
        0, // date
        0,
    ] {
        fixed.extend(value.to_le_bytes());
    }
    let strings = block(
        "040904B0", // US English, Unicode
        &[],
        0,
        true,
        &[
            string("CompanyName", "Iqal"),
            string("FileDescription", description),
            string("FileVersion", version),
            string("InternalName", file_name),
            string("LegalCopyright", "GPL-3.0-or-later"),
            string("OriginalFilename", file_name),
            string("ProductName", "Iqal Blocker"),
            string("ProductVersion", version),
        ],
    );
    let string_info = block("StringFileInfo", &[], 0, true, &[strings]);
    let translation: Vec<u8> = [LANGUAGE, 0x04B0].iter().flat_map(|w| w.to_le_bytes()).collect();
    let var_info = block("VarFileInfo", &[], 0, true, &[block("Translation", &translation, 4, false, &[])]);
    block("VS_VERSION_INFO", &fixed, fixed.len(), false, &[string_info, var_info])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(version_numbers("0.1.0"), (1, 0));
        assert_eq!(version_numbers("1.2.3"), (0x0001_0002, 0x0003_0000));
    }

    #[test]
    fn entries_are_aligned() {
        let res = resources("Iqal", "0.1.0", "iqal.exe");
        assert_eq!(res.len() % 4, 0);
        // The empty first entry.
        assert_eq!(&res[0..8], &[0, 0, 0, 0, 32, 0, 0, 0]);
    }
}
