//! Getting ck-web: the bundle for this system (ck-web and Chromium's files, built by the
//! `ck-web release` workflow), downloaded from its GitHub release the first time you post,
//! checked against the SHA-256 here, and unpacked in ck's data folder.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

/// The ck-web this ck talks to: its release is `ck-web-v{VERSION}`.
pub const VERSION: &str = "0.3.0";
const RELEASES: &str = "https://github.com/gold-silver-copper/ck/releases/download";
/// Each system's bundle, with its SHA-256 (from the release's `.sha256` files).
const BUNDLES: &[(&str, &str)] = &[
    ("linux-x86_64", "cbbccff6428ed777b064e2bee9d98b800d418f62e05f7beaea6d16b0f0723137"),
    ("linux-aarch64", "22e0508cc60ffa55410a6472c223186c72b74b5ec4c4f7d2e937e92b0e2fcbee"),
    ("macos-x86_64", "fe9e16c6f6e50942921f98ee7b64f868cccc926c282a1cb95c366705191af307"),
    ("macos-aarch64", "b114bd0e3e87d3d9c879e9cac2cdc060bae8ee06e18ebc816a1d2fdece867efb"),
];
/// ck-web's program in a bundle: on macOS, in its app.
pub const PROGRAM: &str = if cfg!(target_os = "macos") { "ck-web.app/Contents/MacOS/ck-web" } else { "ck-web" };
/// About how big a bundle is, in megabytes, to say before downloading it.
pub const SIZE_MB: u64 = 150;

/// This system's bundle, if there's one: its file name and SHA-256.
pub fn bundle() -> Option<(String, &'static str)> {
    let system = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    BUNDLES.iter().find(|(s, _)| *s == system).map(|(s, sum)| (format!("ck-web-{VERSION}-{s}.tar.gz"), *sum))
}

/// Where bundles are unpacked: a folder per version.
fn dir() -> Option<PathBuf> {
    crate::store::Store::dir().map(|d| d.join("web"))
}

/// ck-web as downloaded, if it is.
pub fn installed() -> Option<PathBuf> {
    let path = dir()?.join(VERSION).join(PROGRAM);
    path.is_file().then_some(path)
}

/// Download this system's bundle, check it, and unpack it (older versions' go); where
/// ck-web is now. `progress` hears the bytes so far and the whole size.
pub fn install(progress: impl FnMut(u64, Option<u64>)) -> Result<PathBuf> {
    let (name, sum) = bundle().context("ck-web isn't built for this system yet")?;
    let base = dir().context("no data folder to put ck-web in")?;
    // Named as this ck's temp, so another ck's cleanup leaves it be.
    let archive = base.join(format!("{name}.ck-{}-0.tmp", std::process::id()));
    crate::http::download_with_progress(&format!("{RELEASES}/ck-web-v{VERSION}/{name}"), &archive, progress)?;
    let unpacked = (|| -> Result<()> {
        ensure!(sha256(&archive)? == sum, "The download isn't ck-web {VERSION} as this ck knows it (its checksum differs); not using it");
        let file = std::fs::File::open(&archive)?;
        crate::atomic::unpack(flate2::read::GzDecoder::new(file), &base.join(VERSION))
    })();
    let _ = std::fs::remove_file(&archive);
    unpacked?;
    crate::atomic::remove_others(&base, &base.join(VERSION));
    installed().context("ck-web wasn't in the download")
}

fn sha256(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    std::io::copy(&mut std::fs::File::open(path)?, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}
