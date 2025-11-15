use anyhow::{Context, Result};
use clap::{Parser, ArgGroup};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fs;

/// ------------------------------------------------------------
/// Command line interface
/// ------------------------------------------------------------
#[derive(Parser)]
#[clap(author, version, about)]
#[command(group(
    ArgGroup::new("action")
        .required(true)
        .args(&["low_quality_list", "high_quality_list", "get_related"])
))]
struct Cli {
    /// Path to config JSON
    #[arg(short='c', long="config", required=true)]
    config: PathBuf,

    /// Print a JSON object with a list of files and info representing items under the given
    /// directory, prefering the lowest quality representation of the item
    #[arg(short='l', long="list-thumbnail", value_name="dir path" )]
    low_quality_list: Option<PathBuf>,

    /// Print a JSON object with a list of files and info representing items under the given
    /// directory, prefering the highest quality representation of the item
    #[arg(short='L', long="list-high-quality", value_name="dir path")]
    high_quality_list: Option<PathBuf>,

    /// Given a file this will output a JSON object with a list of all files in the item that
    /// represent the file
    #[arg(short='g', long="get-related", num_args=1, value_name="file path")]
    get_related: Option<PathBuf>,
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
    card_subdir: PathBuf,
    path: PathBuf,
}

/// ------------------------------------------------------------
/// Camera adapters
/// ------------------------------------------------------------

trait SourceMediaAdapter {
    fn list_items(&self) -> Vec<FileItem>;
}

struct GoProAdapter;
struct SonyAdapter;

/// For GoPro: top-level directory, all files mixed — just use generic grouping
impl SourceMediaAdapter for GoProAdapter {
    fn list_items(&self) -> Vec<FileItem> {
        let mut ret: Vec<FileItem> = Vec::<FileItem>::new();
        ret.push(FileItem{
            file_path:"no_file".to_string(),
            file_type:"no_file".to_string(),
            item_type:"no_file".to_string()
        });
        return ret
    }
}

/// For Sony: handle DCIM & PRIVATE/M4ROOT with custom subfolders
impl SourceMediaAdapter for SonyAdapter {
    fn list_items(&self) -> Vec<FileItem> {
        return Vec::<FileItem>::new();
    }
}

/// Map camera type string to actual adapter object
fn get_adapter(t: &str) -> Result<Box<dyn SourceMediaAdapter>> {
    Ok(match t {
        "GoPro-Generic-1" => Box::new(GoProAdapter),
        "Sony-ILCEM4" => Box::new(SonyAdapter),
        unknown  => anyhow::bail!("Unknown camera type: {}", unknown)
    })
}

fn value_for_path<'a, T>(
    file: &Path,
    dirs: &'a [(PathBuf, T)],
    ) -> Option<&'a T> {

    dirs.iter()
        .find(|(dir, _)| file.starts_with(dir))
        .map(|(_, v)| v)
}

/// ------------------------------------------------------------
/// Main
/// ------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct fail_json_output {
    data_type: &'static str,
    version: &'static str,
    command_success: bool,
    file_list: Vec<FileItem>
}

#[derive(Serialize, Deserialize)]
struct FileItem {
    file_path: String,
    file_type: String,
    item_type: String
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config file
    let data = std::fs::read_to_string(&cli.config)
        .with_context(|| format!("Failed to read config file {:?}", cli.config))?;

    let cfg: Config = serde_json::from_str(&data)?;

    let mut handler_locations: Vec<(PathBuf, String)> = Vec::new();

    for cam in cfg.source_media {
        let path: PathBuf = match fs::canonicalize(cam.path.join(cam.card_subdir)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        };

        handler_locations.push((path,cam.handler));
    }

    let mut output = fail_json_output{
        data_type: "source_media_interface_api",
        version: env!("CARGO_PKG_VERSION"),
        command_success: false,
        file_list: Vec::<FileItem>::new()
    };

    if let Some(path_buf) = cli.low_quality_list.as_ref() {
        let path: &Path = path_buf.as_ref();
        let file: PathBuf = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        };

        if let Some(value) = value_for_path(&file, &handler_locations) {
            let handler = get_adapter(&value)?;
            output.file_list=handler.list_items();
            output.command_success=true;
        } else {
        }
    }

    // Emit everything as JSON to stdout
    let json = serde_json::to_string(&output)?;
    println!("{}", json);

    Ok(())
}
