//! The setup program carries Iqal's files inside it: the folder named by
//! IQAL_SETUP_PAYLOAD (filled by scripts/build-windows-setup.ps1) becomes
//! a list of (path, contents). Without it the setup has nothing to install
//! and says so. Also embeds the icon, the version description, and a
//! manifest asking Windows for administrator rights.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="Iqal.Setup" version="1.0.0.0"/>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>
"#;

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn main() {
    iqal_winres::embed("Iqal Blocker setup", "iqal-setup.exe");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));

    println!("cargo:rerun-if-env-changed=IQAL_SETUP_PAYLOAD");
    let mut code = String::from("/// The files to install: (path in the Iqal folder, contents).\npub static FILES: &[(&str, &[u8])] = &[\n");
    if let Some(root) = std::env::var_os("IQAL_SETUP_PAYLOAD").map(PathBuf::from) {
        let mut list = Vec::new();
        files(&root, &mut list);
        list.sort();
        for path in list {
            let relative = path.strip_prefix(&root).expect("inside the payload").to_string_lossy().replace('/', "\\");
            let absolute = std::fs::canonicalize(&path).expect("payload file");
            println!("cargo:rerun-if-changed={}", absolute.display());
            let _ = writeln!(code, "    ({relative:?}, include_bytes!({:?})),", absolute.display().to_string());
        }
    }
    code.push_str("];\n");
    std::fs::write(out_dir.join("payload.rs"), code).expect("writing payload.rs");

    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let manifest = out_dir.join("setup.manifest");
    std::fs::write(&manifest, MANIFEST).expect("writing the manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());
    println!("cargo:rustc-link-arg-bins=/MANIFESTUAC:level='requireAdministrator'");
}
