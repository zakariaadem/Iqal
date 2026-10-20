//! Gives the Windows program its icon and version description.

fn main() {
    iqal_winres::embed("Iqal command line", "iqal-cli.exe");
}
