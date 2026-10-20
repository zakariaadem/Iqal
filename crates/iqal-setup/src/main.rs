//! iqal-setup: installs Iqal on Windows, updates it, and removes it again.
//!
//! - `iqal-setup.exe`: installs (or updates) into Program Files\Iqal.
//! - `iqal-setup.exe --uninstall`: removes Iqal (Settings > Apps runs this).
//!   Refused while a block is locked: removing Iqal would end the lock.
//! - `--quiet` with either: no questions or messages; the exit code says
//!   whether it worked.
//!
//! The setup keeps a copy of itself in the Iqal folder for removing.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod dialog;
#[cfg(windows)]
mod system;

#[cfg(windows)]
include!(concat!(env!("OUT_DIR"), "/payload.rs"));

#[cfg(not(windows))]
fn main() {
    eprintln!("iqal-setup installs Iqal on Windows; on Linux use the iqal package");
}

#[cfg(windows)]
fn main() {
    use iqal_i18n::{t, ta};

    iqal_i18n::init_with(iqal_i18n::system_language(), false);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let quiet = args.iter().any(|a| a == "--quiet");
    let has = |name: &str| args.iter().any(|a| a == name);

    // The copy that removes the Iqal folder after the uninstaller ended.
    if let Some(i) = args.iter().position(|a| a == "--remove-folder") {
        if let (Some(folder), Some(pid)) = (args.get(i + 1), args.get(i + 2).and_then(|p| p.parse().ok())) {
            system::remove_folder_after(std::path::Path::new(folder), pid);
        }
        return;
    }

    let version = env!("CARGO_PKG_VERSION");
    let folder = system::install_folder();
    let data = system::data_folder();
    let result = if has("--uninstall") {
        if !quiet {
            let question = ta("uninstall-detail", &[("folder", data.display().to_string().into())]);
            let buttons = [(2, t("action-cancel")), (1, t("uninstall-action"))];
            if dialog::ask(dialog::Icon::Warning, &t("uninstall-title"), &question, &buttons) != Some(1) {
                return;
            }
        }
        system::uninstall(&folder).map(|()| (t("uninstall-done"), String::new())).map_err(|e| match e {
            system::Failure::Locked => (t("uninstall-locked"), t("uninstall-locked-detail")),
            system::Failure::Other(why) => (t("uninstall-failed"), why),
        })
    } else {
        let update = system::installed(&folder);
        if !quiet {
            let (title, detail, action) = if update {
                (ta("setup-update-title", &[("version", version.into())]), t("setup-update-detail"), t("action-update"))
            } else {
                (
                    ta("setup-install-title", &[("version", version.into())]),
                    ta("setup-install-detail", &[("folder", folder.display().to_string().into())]),
                    t("action-install"),
                )
            };
            if dialog::ask(dialog::Icon::Iqal, &title, &detail, &[(2, t("action-cancel")), (1, action)]) != Some(1) {
                return;
            }
        }
        system::install(&folder, FILES)
            .map(|()| if update { (t("setup-updated"), String::new()) } else { (t("setup-installed"), t("setup-installed-detail")) })
            .map_err(|e| match e {
                system::Failure::Locked => (t("setup-locked"), t("setup-locked-detail")),
                system::Failure::Other(why) => (t("setup-failed"), why),
            })
    };
    match result {
        Ok((title, detail)) => {
            if !quiet {
                dialog::message(dialog::Icon::Iqal, &title, &detail);
            }
            // Removing: the folder holding this program goes once it ends.
            if has("--uninstall") {
                system::remove_folder_later(&folder);
            }
        }
        Err((title, detail)) => {
            if quiet {
                eprintln!("{title}: {detail}");
            } else {
                dialog::message(dialog::Icon::Error, &title, &detail);
            }
            std::process::exit(1);
        }
    }
}
