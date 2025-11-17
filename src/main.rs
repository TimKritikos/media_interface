use anyhow::{Result};
use clap::{Parser, ArgGroup};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    /// Path to config json file. If none is supplied, a file named "interface_config.json" in the
    /// location of the executable is used.
    #[arg(short='c', long="config")]
    config: Option<PathBuf>,

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
    fn list_low_quality(&self, source_media_location: &PathBuf, source_media_card: &PathBuf) -> Result<Vec<FileItem>>;
    fn name(&self) -> String;
}

struct GoProAdapter;
struct SonyAdapter;

/// For GoPro: top-level directory, all files mixed — just use generic grouping
impl SourceMediaAdapter for GoProAdapter {
    fn list_low_quality(&self,  _source_media_location: &PathBuf,  source_media_card: &PathBuf) -> Result<Vec<FileItem>> {
        let mut ret: Vec<FileItem> = Vec::<FileItem>::new();

        let paths = match fs::read_dir(source_media_card) {
            Ok(p) => p,
            Err(e) => { return Err(anyhow::anyhow!("Error opening provided card dir: {}", e))}
        };

        for path in paths{
            if let Some(name) = path.unwrap().path().file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".THM") {
                    let part_id = match name.get(2..4).unwrap().parse::<u32>() {
                        Ok(p) => p,
                        Err(e) => { return Err(anyhow::anyhow!("Error parsing filename: {}",e)); }
                    };
                    if part_id == 1 {
                        ret.push(FileItem{
                            file_path:name.to_string(),
                            file_type:"image".to_string(),
                            item_type:"video".to_string()
                        });
                    }else{
                        eprintln!("WARNING: Unable to parse file {}", name);
                    }
                }else if name.ends_with(".JPG") {
                    ret.push(FileItem{
                        file_path:name.to_string(),
                        file_type:"image".to_string(),
                        item_type:"image".to_string()
                    });
                }
            }else{
                return Err(anyhow::anyhow!("Unknown error in traversin directory"));
            }
        }
        return Ok(ret)
    }
    fn name(&self) -> String {
        return "GoPro-Generic-1".to_string()
    }
}

/// For Sony: handle DCIM & PRIVATE/M4ROOT with custom subfolders
impl SourceMediaAdapter for SonyAdapter {
    fn list_low_quality(&self,  _source_media_location: &PathBuf,  _source_media_card: &PathBuf ) -> Result<Vec<FileItem>> {
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn name(&self) -> String {
        return "Sony-ILCEM4-1".to_string()
    }
}

/// Map camera type string to actual adapter object
fn get_adapter(t: &str) -> Result<Box<dyn SourceMediaAdapter>> {
    Ok(match t {
        "GoPro-Generic-1" => Box::new(GoProAdapter),
        "Sony-ILCEM4-1" => Box::new(SonyAdapter),
        unknown  => anyhow::bail!("Unknown camera type: {}", unknown)
    })
}

fn value_for_path<'a, T>(
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
    item_type: String
}
fn fail_main( mut data: FailJsonOutput, error: String ) -> Result<()> {
    data.error_string=Some(error.clone());
    data.file_list=None;
    println!("{}", serde_json::to_string(&data)?);
    return Err(anyhow::anyhow!("{}", error));
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

    let config_file_path = match cli.config {
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
    let data = match std::fs::read_to_string(&config_file_path){
        Ok(p) => p,
        Err(e) =>  { return fail_main(output, format!("Failed to read config file {:?}: {}", config_file_path, e))}
    };

    let cfg: Config = serde_json::from_str(&data)?;

    let mut handler_locations: Vec<(PathBuf, String)> = Vec::new();

    for cam in cfg.source_media {
        let path: PathBuf = match fs::canonicalize(cam.path.join(&cam.card_subdir)) {
            Ok(p) => p,
            Err(e) => { return fail_main(output, format!("Error reading source media dir {:?}: {}", cam.path.join(cam.card_subdir), e))}
        };
        handler_locations.push((path,cam.handler));
    }


    if let Some(path_buf) = cli.low_quality_list.as_ref() {
        let path: &Path = path_buf.as_ref();
        let file: PathBuf = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(e) => { return fail_main(output, format!("error finding the absolute path of input file: {}",e)); }
        };

        let value : &(PathBuf, String) = match value_for_path(&file, &handler_locations) {
            Some(p) => p,
            None => { return fail_main(output,format!("Couldn't find handler responsible for a dir in the path of the input file")) }
        };
        let handler = get_adapter( &(value.1))?;
        match handler.list_low_quality(&value.0,&file) {
            Ok(p) => {
                output.file_list=Some(p);
            }
            Err(e) => { return fail_main(output, format!("handler {}: {}",handler.name(),e)); }
        };
        output.command_success=true;
        output.error_string=None;
    }else if let Some(_path_buf) = cli.high_quality_list.as_ref(){

    }else if let Some(_path_buf) = cli.get_related.as_ref(){

    }else{
        return fail_main(output, "Internal error: no action selected".to_string());
    }

    // Emit everything as JSON to stdout
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}
