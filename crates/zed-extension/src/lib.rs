//! Zed extension shim — locates or downloads the `bevy-color-lsp` binary.
//!
//! There are no authored public items; the only public symbol is the
//! `#[export_name = "init-extension"]` function emitted by
//! `zed::register_extension!`, which has no Rust doc slot.
#![warn(missing_docs)]
#![allow(missing_docs)]
// register_extension! emits a pub extern "C" fn with no doc slot
// `cargo_common_metadata`: readme/keywords/categories are managed at release time.
#![allow(clippy::cargo_common_metadata)]
// `multiple_crate_versions`: transitive dep conflict we don't control.
#![allow(clippy::multiple_crate_versions)]

use zed_extension_api::{
    self as zed, settings::LspSettings, Command, LanguageServerId, Result, Worktree,
};

const SERVER_ID: &str = "bevy-color-lsp";
const REPO: &str = "DavidDudson/zed-bevy-colors";
const BIN_NAME: &str = "bevy-color-lsp";

struct BevyColorExtension {
    cached_binary_path: Option<String>,
}

impl BevyColorExtension {
    fn binary_path(&mut self, id: &LanguageServerId, worktree: &Worktree) -> Result<String> {
        if let Ok(settings) = LspSettings::for_worktree(SERVER_ID, worktree) {
            if let Some(bin) = settings.binary {
                if let Some(path) = bin.path {
                    return Ok(path);
                }
            }
        }

        if let Some(path) = worktree.which(BIN_NAME) {
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if std::fs::metadata(path).is_ok() {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            REPO,
            zed::GithubReleaseOptions { require_assets: true, pre_release: false },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = asset_name(platform, arch)?;
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no asset named {asset_name} in {}", release.version))?;

        let version_dir = format!("bevy-color-lsp-{}", release.version);
        let bin_path = format!("{version_dir}/{BIN_NAME}{}", bin_suffix(platform));
        if std::fs::metadata(&bin_path).is_err() {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            zed::download_file(&asset.download_url, &version_dir, file_type(platform))
                .map_err(|e| format!("download failed: {e}"))?;
            zed::make_file_executable(&bin_path)?;
            cleanup_old_versions(&version_dir);
        }

        self.cached_binary_path = Some(bin_path.clone());
        Ok(bin_path)
    }
}

fn asset_name(platform: zed::Os, arch: zed::Architecture) -> Result<String> {
    let target = match (platform, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
        (zed::Os::Mac, zed::Architecture::X8664) => "x86_64-apple-darwin",
        (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
        (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-msvc",
        (os, arch) => return Err(format!("unsupported platform: {os:?}/{arch:?}")),
    };
    let ext = match platform {
        zed::Os::Windows => "zip",
        _ => "tar.gz",
    };
    Ok(format!("bevy-color-lsp-{target}.{ext}"))
}

const fn file_type(platform: zed::Os) -> zed::DownloadedFileType {
    match platform {
        zed::Os::Windows => zed::DownloadedFileType::Zip,
        _ => zed::DownloadedFileType::GzipTar,
    }
}

const fn bin_suffix(platform: zed::Os) -> &'static str {
    match platform {
        zed::Os::Windows => ".exe",
        _ => "",
    }
}

fn cleanup_old_versions(keep_dir: &str) {
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("bevy-color-lsp-") && name != keep_dir {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

impl zed::Extension for BevyColorExtension {
    fn new() -> Self {
        Self { cached_binary_path: None }
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        Ok(Command { command: self.binary_path(id, worktree)?, args: Vec::new(), env: Vec::new() })
    }
}

zed::register_extension!(BevyColorExtension);
