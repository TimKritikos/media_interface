use anyhow::{Result};
use clap::{Parser, ArgGroup};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process;
use std::fs;

mod gopro_hero_generic_1;
mod sony_ilcem4_1;
mod helpers;

/// ------------------------------------------------------------
/// Command line interface
/// ------------------------------------------------------------
#[derive(Parser)]
#[clap(author, version, about)]
#[command(group(
    ArgGroup::new("action")
        .required(true)
        .args(&["list_thumbnail", "list_high_quality", "get_related"])
))]
struct Cli {
    /// Path to config json file. If none is supplied, a file named "interface_config.json" in the
    /// location of the executable is used.
    #[arg(short='c', long="config")]
    config: Option<PathBuf>,

    /// Print a JSON object with a list of files and info representing items under the given
    /// directory, prefering the lowest quality representation of the item
    #[arg(short='l', long="list-thumbnail", value_name="dir path" )]
    list_thumbnail: Option<PathBuf>,

    /// Print a JSON object with a list of files and info representing items under the given
    /// directory, prefering the highest quality representation of the item
    #[arg(short='L', long="list-high-quality", value_name="dir path")]
    list_high_quality: Option<PathBuf>,

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
    data_type: String,
    source_media: Vec<SourceMediaEntry>
}

#[derive(Debug, Deserialize)]
struct SourceMediaEntry {
    handler: String,
    card_subdir: PathBuf,
    path: PathBuf,
}


/// ------------------------------------------------------------
/// Source media adapters
/// ------------------------------------------------------------
trait SourceMediaAdapter {
    fn list_thumbnail(&self, source_media_location: &PathBuf, source_media_card: &PathBuf) -> Result<Vec<FileItem>>;
    fn list_high_quality(&self, source_media_location: &PathBuf, source_media_card: &PathBuf) -> Result<Vec<FileItem>>;
    fn get_related(&self, source_media_location: &PathBuf, source_media_file: &PathBuf) -> Result<Vec<FileItem>>;
    fn name(&self) -> String;
}


fn get_handler(t: &str) -> Result<Box<dyn SourceMediaAdapter>> {
    Ok(match t {
        "GoPro-Hero-Generic-1" => Box::new(gopro_hero_generic_1::GoProAdapter),
        "Sony-ILCEM4-1" => Box::new(sony_ilcem4_1::SonyAdapter),
        unknown  => anyhow::bail!("Unknown camera type: {}", unknown)
    })
}

fn get_handler_and_dir<'a, T>(
    file: &Path,
    dirs: &'a [(PathBuf, T)],
    ) -> Option<&'a (PathBuf, T)> {

    dirs.iter()
        .find(|(dir, _)| file.starts_with(dir))
}

/// ------------------------------------------------------------
/// Main
/// ------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct FailJsonOutput {
    data_type: &'static str,
    version: &'static str,
    command_success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_list: Option<Vec<FileItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_string: Option<String>
}

#[derive(Serialize, Deserialize)]
struct FileItem {
    file_path: String,
    file_type: String,
    item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    part_count: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    part_num: Option<u8>,
}

fn fail_main( data: &mut FailJsonOutput, error: String ) -> ! {
    data.error_string=Some(error.clone());
    data.file_list=None;
    println!("{}", serde_json::to_string(&data).unwrap_or_else(|_| "Failed to serialise json".to_string()));
    eprintln!("{}", error);
    process::exit(1);
}

fn main() -> Result<()> {
    let mut output = FailJsonOutput{
        data_type: "source_media_interface_api",
        version: env!("CARGO_PKG_VERSION"),
        command_success: false,
        file_list: None,
        error_string: Some("Uninitialised error message".to_string())
    };

    let cli = Cli::parse();

    let config_file_path:PathBuf = match cli.config {
        Some(p) => p,
        None => {
            let invoked_path = PathBuf::from(env::args().next().unwrap());

            let absolute_invoked_path = if invoked_path.is_absolute() {
                invoked_path
            } else {
                env::current_dir().unwrap().join(invoked_path)
            };

            absolute_invoked_path.parent().unwrap().join(PathBuf::from("interface_config.json"))
        }
    };

    // Load config file
    let data = std::fs::read_to_string(&config_file_path)
        .unwrap_or_else(|e| fail_main(&mut output, format!("Failed to read config file {:?}: {}", config_file_path, e)));

    let cfg: Config = serde_json::from_str(&data)?;

    if cfg.data_type != "source_media_config" {
        fail_main(&mut output, format!("Invalid data type on the config file: {}", cfg.data_type));
    }

    let _ = env::set_current_dir(&config_file_path.parent().unwrap());

    let mut handlers: Vec<(PathBuf, String)> = Vec::new();

    for cam in cfg.source_media {
        let path: PathBuf = fs::canonicalize(cam.path.join(&cam.card_subdir))
            .unwrap_or_else(|e| fail_main(&mut output, format!("Error reading source media dir {:?}: {}", cam.path.join(cam.card_subdir), e)));
        handlers.push((path,cam.handler));
    }

    if let Some(input_file) = cli.list_thumbnail.as_ref() {
        handle_action_with_input(&mut output, input_file, &handlers, |handler, base, file| handler.list_thumbnail(base, file));
    }else if let Some(input_file) = cli.list_high_quality.as_ref() {
        handle_action_with_input(&mut output, input_file, &handlers, |handler, base, file| handler.list_high_quality(base, file));
    }else if let Some(input_file) = cli.get_related.as_ref() {
        handle_action_with_input(&mut output, input_file, &handlers, |handler, base, file| handler.get_related(base, file));
    }else{
        fail_main(&mut output, "Internal error: no action selected".into());
    }

    // Emit everything as JSON to stdout
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}

fn handle_action_with_input<F>( mut output: &mut FailJsonOutput, input_file: &PathBuf, handlers: &[(PathBuf, String)], action: F, ) where
    F: Fn(&dyn SourceMediaAdapter, &PathBuf, &PathBuf) -> Result<Vec<FileItem>>,
{
    let input_path = input_file.as_path();

    let file = fs::canonicalize(input_path)
        .unwrap_or_else(|e| fail_main(&mut output, format!("error finding the absolute path of input file: {}", e)));

    let handler_and_dir = get_handler_and_dir(&file, handlers)
        .unwrap_or_else(|| fail_main(&mut output, "Couldn't find handler responsible for a dir in the path of the input file".into()));

    let handler = get_handler(&handler_and_dir.1)
        .unwrap_or_else(|e| fail_main(&mut output, format!("couldn't load handler {}: {}", handler_and_dir.1, e)));

    output.file_list = Some(
        action(handler.as_ref(), &handler_and_dir.0, &file)
            .unwrap_or_else(|e| fail_main(&mut output, format!("handler {}: {}", handler.name(), e)))
    );

    output.command_success = true;
    output.error_string = None;
}

