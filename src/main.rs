use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// ------------------------------------------------------------
/// Command line interface
/// ------------------------------------------------------------
#[derive(Parser)]
struct Cli {
    /// Path to config JSON
    #[arg(short, long)]
    config: PathBuf
}

/// ------------------------------------------------------------
/// Config structures
/// ------------------------------------------------------------
#[derive(Debug, Deserialize)]
struct Config {
    source_media: Vec<SourceMediaEntry>
}

#[derive(Debug, Deserialize)]
struct SourceMediaEntry {
    handler: String,
    path: PathBuf,
}

/// ------------------------------------------------------------
/// Output structures
/// ------------------------------------------------------------
#[derive(Debug, Clone, Serialize)]
enum AssetKind {
    Photo,
    Video,
    Other
}

#[derive(Debug, Clone, Serialize)]
struct AssetItem {
    id: String,
    kind: AssetKind,
    /// All related files
    files: Vec<PathBuf>,
    /// The best file to show as preview
    representative: PathBuf
}

/// ------------------------------------------------------------
/// Shared utilities
/// ------------------------------------------------------------
fn ext_lower(p: &Path) -> Option<String> {
    p.extension().and_then(OsStr::to_str).map(|s| s.to_lowercase())
}

fn normalize_base(stem: &str) -> String {
    let re = Regex::new(r"(?i)([_\-\.]\d{1,3}$|[_\-](l|h)$|_thm$)").unwrap();
    re.replace(stem, "").to_string()
}

fn pick_kind(files: &[PathBuf]) -> AssetKind {
    let (mut img, mut vid) = (false, false);

    for f in files {
        if let Some(ext) = ext_lower(f) {
            match ext.as_str() {
                "jpg" | "jpeg" | "arw" | "png" | "dng" => img = true,
                "mp4" | "mov" | "mkv" => vid = true,
                _ => {}
            }
        }
    }
    match (img, vid) {
        (true, false) => AssetKind::Photo,
        (false, true) => AssetKind::Video,
        (true, true) => AssetKind::Other,
        _ => AssetKind::Other
    }
}

fn pick_representative(files: &[PathBuf], kind: &AssetKind) -> PathBuf {
    // prefer thumbnails
    for p in files {
        if ext_lower(p).as_deref() == Some("thm") {
            return p.clone();
        }
    }
    // prefer jpeg for photos
    if matches!(kind, AssetKind::Photo | AssetKind::Other) {
        for p in files {
            if matches!(ext_lower(p).as_deref(), Some("jpg" | "jpeg")) {
                return p.clone();
            }
        }
    }
    // prefer low bitrate video
    if matches!(kind, AssetKind::Video | AssetKind::Other) {
        let re = Regex::new(r"(?i)([_\-]l$|_low|-low)").unwrap();
        for p in files {
            if let Some(stem) = p.file_stem().and_then(OsStr::to_str) {
                if re.is_match(stem) {
                    return p.clone();
                }
            }
        }
        // fallback to any video
        for p in files {
            if matches!(ext_lower(p).as_deref(), Some("mp4" | "mov")) {
                return p.clone();
            }
        }
    }

    // fallback
    files[0].clone()
}

/// ------------------------------------------------------------
/// Generic grouping logic (used by all adapters)
/// ------------------------------------------------------------
fn group_by_normalized_name(root: &Path) -> Result<Vec<AssetItem>> {
    let mut groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for entry in WalkDir::new(root).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() {
            if let Some(stem) = p.file_stem().and_then(OsStr::to_str) {
                let base = normalize_base(stem);
                groups.entry(base).or_default().push(p.to_path_buf());
            }
        }
    }

    let mut out = Vec::new();
    for (base, files) in groups {
        let kind = pick_kind(&files);
        let representative = pick_representative(&files, &kind);
        out.push(AssetItem { id: base, kind, files, representative });
    }
    Ok(out)
}

/// ------------------------------------------------------------
/// Camera adapters
/// ------------------------------------------------------------

trait CameraAdapter {
    fn scan(&self, root: &Path) -> Result<Vec<AssetItem>>;
    fn name(&self) -> String;
}

struct GoProAdapter;
struct SonyAdapter;

/// For GoPro: top-level directory, all files mixed — just use generic grouping
impl CameraAdapter for GoProAdapter {
    fn scan(&self, root: &Path) -> Result<Vec<AssetItem>> {
        group_by_normalized_name(root)
    }
    fn name(&self) -> String {
        return String::from("GoPro handler!");
    }
}

/// For Sony: handle DCIM & PRIVATE/M4ROOT with custom subfolders
impl CameraAdapter for SonyAdapter {
    fn scan(&self, root: &Path) -> Result<Vec<AssetItem>> {
        let mut groups = HashMap::<String, Vec<PathBuf>>::new();

        let add_dir = |groups: &mut HashMap<String, Vec<PathBuf>>, dir: &Path| {
            if dir.exists() {
                for entry in WalkDir::new(dir).min_depth(1).into_iter().filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(stem) = p.file_stem().and_then(OsStr::to_str) {
                            let base = normalize_base(stem);
                            groups.entry(base).or_default().push(p.to_path_buf());
                        }
                    }
                }
            }
        };

        add_dir(&mut groups, &root.join("DCIM"));
        let m4 = root.join("PRIVATE").join("M4ROOT");
        add_dir(&mut groups, &m4.join("CLIP"));
        add_dir(&mut groups, &m4.join("THMBNL"));

        let mut out = Vec::new();
        for (base, files) in groups {
            let kind = pick_kind(&files);
            let representative = pick_representative(&files, &kind);
            out.push(AssetItem { id: base, kind, files, representative });
        }
        Ok(out)
    }
    fn name(&self) -> String {
        return String::from("Sony handler!");
    }
}

/// Map camera type string to actual adapter object
fn get_adapter(t: &str) -> Result<Box<dyn CameraAdapter>> {
    Ok(match t {
        "GoPro-Generic-1" => Box::new(GoProAdapter),
        "Sony-ILCEM4" => Box::new(SonyAdapter),
        unknown  => anyhow::bail!("Unknown camera type: {}", unknown)
    })
}

/// ------------------------------------------------------------
/// Main
/// ------------------------------------------------------------
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config file
    let data = std::fs::read_to_string(&cli.config)
        .with_context(|| format!("Failed to read config file {:?}", cli.config))?;

    let cfg: Config = serde_json::from_str(&data)?;

    let mut results: HashMap<String, Vec<AssetItem>> = HashMap::new();

    for cam in cfg.source_media {
        let handler = get_adapter(&cam.handler)?;
        let path = cam.path;
        println!("Using {} at {:?}",handler.name(),path);
        //let mut items = Vec::new();

        //for path in cam.paths {
        //    let path = path.canonicalize()?;
        //    let mut scanned = adapter.scan(&path)
        //        .with_context(|| format!("Scanning camera '{}' at {:?}", cam.name, path))?;
        //    items.append(&mut scanned);
        //}

        //#results.insert(cam.name.clone(), items);
    }

    // Emit everything as JSON to stdout
    let json = serde_json::to_string_pretty(&results)?;
    println!("{}", json);

    Ok(())
}
