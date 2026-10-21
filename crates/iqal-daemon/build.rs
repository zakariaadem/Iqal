//! Gives the Windows program its icon and version description.

fn main() {
    iqal_winres::embed("Iqal Blocker service", "iqal-daemon.exe");
}
