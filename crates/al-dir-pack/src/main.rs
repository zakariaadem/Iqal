//! Packages the browser extension:
//!
//! * `dist/extension/al-dir.xpi`: Firefox (unsigned; Mozilla signs it),
//! * `dist/extension/al-dir.crx`: Chromium-family browsers, CRX3 signed
//!   with our RSA key (the key determines the extension id),
//! * `dist/extension/chromium-id` and `dist/extension/version`.
//!
//! Usage: `cargo run -p al-dir-pack -- [--key PATH]`. The key defaults to
//! `~/.config/al-dir/extension-key.pem` and is created if missing. Keep it
//! private and backed up: losing it changes the extension id.
//! Signing uses the `openssl` command.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

fn main() {
    if let Err(e) = run() {
        eprintln!("al-dir-pack: {e}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src = root.join("extension");
    let out = root.join("dist/extension");
    let mut key = dirs_config().join("al-dir/extension-key.pem");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--key" => key = args.next().map(PathBuf::from).ok_or_else(|| io::Error::other("--key needs a path"))?,
            other => return Err(io::Error::other(format!("unknown argument {other}"))),
        }
    }
    fs::create_dir_all(&out)?;

    let version = manifest_version(&src.join("manifest.chromium.json"))?;
    let xpi = zip_extension(&src, "manifest.firefox.json")?;
    fs::write(out.join("al-dir.xpi"), &xpi)?;

    ensure_key(&key)?;
    let zip = zip_extension(&src, "manifest.chromium.json")?;
    let (crx, id) = crx3(&zip, &key)?;
    fs::write(out.join("al-dir.crx"), crx)?;
    fs::write(out.join("chromium-id"), &id)?;
    fs::write(out.join("version"), &version)?;
    println!("version {version}\nchromium id {id}\nwritten to {}", out.display());
    Ok(())
}

fn dirs_config() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn manifest_version(path: &Path) -> io::Result<String> {
    let text = fs::read_to_string(path)?;
    let start = text.find("\"version\"").ok_or_else(|| io::Error::other("manifest has no version"))?;
    let rest = &text[start + 9..];
    let open = rest.find('"').ok_or_else(|| io::Error::other("bad version"))? + 1;
    let close = rest[open..].find('"').ok_or_else(|| io::Error::other("bad version"))?;
    Ok(rest[open..open + close].to_string())
}

/// All extension files, with the chosen manifest stored as manifest.json.
fn zip_extension(src: &Path, manifest: &str) -> io::Result<Vec<u8>> {
    let mut files = Vec::new();
    collect(src, src, &mut files)?;
    files.retain(|(name, _)| !(name.starts_with("manifest.") && name.ends_with(".json")));
    files.push(("manifest.json".into(), fs::read(src.join(manifest))?));
    files.sort();
    Ok(zip_store(&files))
}

fn collect(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(base, &path, out)?;
        } else {
            let name = path.strip_prefix(base).expect("under base").to_string_lossy().replace('\\', "/");
            out.push((name, fs::read(&path)?));
        }
    }
    Ok(())
}

/// A zip archive with uncompressed ("stored") entries.
fn zip_store(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let header = |sig: u32, central: bool| {
            let mut h = Vec::new();
            h.extend_from_slice(&sig.to_le_bytes());
            if central {
                h.extend_from_slice(&20u16.to_le_bytes()); // version made by
            }
            h.extend_from_slice(&20u16.to_le_bytes()); // version needed
            h.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 names
            h.extend_from_slice(&0u16.to_le_bytes()); // stored
            h.extend_from_slice(&0u16.to_le_bytes()); // time
            h.extend_from_slice(&0x21u16.to_le_bytes()); // date 1980-01-01
            h.extend_from_slice(&crc.to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(name.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes()); // extra
            if central {
                h.extend_from_slice(&[0; 8]); // comment len, disk, int attrs, ext attrs (part)
                h.extend_from_slice(&[0; 2]);
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h.extend_from_slice(name.as_bytes());
            h
        };
        out.extend(header(0x0403_4b50, false));
        out.extend_from_slice(data);
        central.extend(header(0x0201_4b50, true));
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // disk numbers
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn ensure_key(key: &Path) -> io::Result<()> {
    if key.exists() {
        return Ok(());
    }
    if let Some(dir) = key.parent() {
        fs::create_dir_all(dir)?;
    }
    openssl(&["genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out", &key.to_string_lossy()])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(key, fs::Permissions::from_mode(0o600))?;
    }
    eprintln!("created new extension key {} (keep it private and backed up)", key.display());
    Ok(())
}

fn openssl(args: &[&str]) -> io::Result<Vec<u8>> {
    let out = Command::new("openssl").args(args).output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!("openssl {}: {}", args[0], String::from_utf8_lossy(&out.stderr))));
    }
    Ok(out.stdout)
}

/// Protobuf: one length-delimited field.
fn field(number: u32, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    varint(&mut out, (number as u64) << 3 | 2);
    varint(&mut out, bytes.len() as u64);
    out.extend_from_slice(bytes);
    out
}

fn varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/// Builds a CRX3 file; returns it and the extension id.
fn crx3(zip: &[u8], key: &Path) -> io::Result<(Vec<u8>, String)> {
    let key = key.to_string_lossy();
    let public_der = openssl(&["pkey", "-in", &key, "-pubout", "-outform", "DER"])?;
    let crx_id: Vec<u8> = Sha256::digest(&public_der)[..16].to_vec();
    let id: String = crx_id.iter().flat_map(|b| [b >> 4, b & 0xf]).map(|n| (b'a' + n) as char).collect();

    let signed_data = field(1, &crx_id);
    let mut to_sign = b"CRX3 SignedData\x00".to_vec();
    to_sign.extend_from_slice(&(signed_data.len() as u32).to_le_bytes());
    to_sign.extend_from_slice(&signed_data);
    to_sign.extend_from_slice(zip);

    let tmp = std::env::temp_dir().join(format!("al-dir-pack-{}", std::process::id()));
    fs::write(&tmp, &to_sign)?;
    let signature = openssl(&["dgst", "-sha256", "-sign", &key, &tmp.to_string_lossy()]);
    let _ = fs::remove_file(&tmp);
    let signature = signature?;

    let mut proof = field(1, &public_der);
    proof.extend(field(2, &signature));
    let mut header = field(2, &proof); // sha256_with_rsa
    header.extend(field(10000, &signed_data));

    let mut crx = b"Cr24".to_vec();
    crx.extend_from_slice(&3u32.to_le_bytes());
    crx.extend_from_slice(&(header.len() as u32).to_le_bytes());
    crx.extend_from_slice(&header);
    crx.extend_from_slice(zip);
    Ok((crx, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn varints() {
        let mut v = Vec::new();
        varint(&mut v, 10000 << 3 | 2);
        assert_eq!(v, [0x82, 0xf1, 0x04]);
    }
}
