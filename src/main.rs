use anyhow::{Result};
use clap::{Parser, ArgGroup};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::process;

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
/// Helper functions
/// ------------------------------------------------------------

fn for_each_file_type<F>(dir: &Path, mut f: F) -> Result<()>
where
    F: FnMut(&std::path::Path, String, String, Option<&str>) -> Result<()>,
{
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let ext = path
            .extension()
            .and_then(|e| e.to_str()); // Option<&str>

        let path_str = path
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();

        let filename = path
            .file_name()
            .unwrap()
            .to_string_lossy().into_owned();

        match f(&path, filename, path_str, ext){
            Ok(()) => {},
            Err(e) => { return Err(e); }
        }
    }
    Ok(())
}

fn create_simple_file(file_path:String, file_type:&str, item_type:&str) -> FileItem {
    FileItem{
        file_path:file_path,
        file_type:file_type.to_string(),
        item_type:item_type.to_string()
    }
}

fn get_gopro_video_part_id(filename:String) -> Result<u8> {
     return match filename.as_str().get(2..4).unwrap().parse::<u8>() {
        Ok(p) => Ok(p),
        Err(e) => { return Err(anyhow::anyhow!("Error parsing filename: {}",e)); }
    };
}

fn filter_top_level_dir<F>(source_dir: &Path, mut filter: F) -> Result<Vec<FileItem>>
where
    F:FnMut(&str, Option<&str>, &str)->Result<Option<FileItem>>,
{
    let mut items = Vec::<FileItem>::new();

    for_each_file_type(source_dir, |_path:&Path, filename: String, path_str: String, ext: Option<&str>| {
        if let Some(item) = filter(&filename, ext, &path_str)? {
            items.push(item);
        }
        Ok(())
    })
    .map_err(|err| anyhow::anyhow!("Error traversing directory: {}", err))?;

    Ok(items)
}


/// ------------------------------------------------------------
/// Camera adapters
/// ------------------------------------------------------------

trait SourceMediaAdapter {
    fn list_thumbnail(&self, source_media_location: &PathBuf, source_media_card: &PathBuf) -> Result<Vec<FileItem>>;
    fn list_high_quality(&self, source_media_location: &PathBuf, source_media_card: &PathBuf) -> Result<Vec<FileItem>>;
    fn name(&self) -> String;
}

struct GoProAdapter;
struct SonyAdapter;

impl SourceMediaAdapter for GoProAdapter {
    fn list_thumbnail( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, ) -> Result<Vec<FileItem>> {
        filter_top_level_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &str|{
            match ext {
                Some("THM") => {
                    if get_gopro_video_part_id(filename.to_string())? == 1 {
                        Ok(Some(create_simple_file(path.to_string(), "image", "video")))
                    } else {
                        Err(anyhow::anyhow!("Unable to parse video id file {}",path))
                    }
                }
                Some("JPG") => Ok(Some(create_simple_file(path.to_string(), "image", "image"))),
                Some("MP4") | Some("GPR") | Some("LRV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path)),
            }
        })
    }
    fn list_high_quality( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, ) -> Result<Vec<FileItem>> {
        filter_top_level_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &str|{
            match ext {
                Some("MP4") => {
                    if get_gopro_video_part_id(filename.to_string())? == 1 {
                        Ok(Some(create_simple_file(path.to_string(), "image", "video")))
                    } else {
                        Err(anyhow::anyhow!("Unable to parse video id file {}",path))
                    }
                }
                Some("JPG") => Ok(Some(create_simple_file(path.to_string(), "image", "image"))),
                Some("THM") | Some("GPR") | Some("LRV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path)),
            }
        })
    }

    fn name(&self) -> String {
        return "GoPro-Generic-1".to_string()
    }
}

/// For Sony: handle DCIM & PRIVATE/M4ROOT with custom subfolders
impl SourceMediaAdapter for SonyAdapter {
    fn list_thumbnail(&self,  _source_media_location: &PathBuf,  _source_media_card: &PathBuf ) -> Result<Vec<FileItem>> {
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn list_high_quality(&self,  _source_media_location: &PathBuf,  _source_media_card: &PathBuf ) -> Result<Vec<FileItem>> {
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

    let mut handler_locations: Vec<(PathBuf, String)> = Vec::new();

    for cam in cfg.source_media {
        let path: PathBuf = fs::canonicalize(cam.path.join(&cam.card_subdir))
            .unwrap_or_else(|e| fail_main(&mut output, format!("Error reading source media dir {:?}: {}", cam.path.join(cam.card_subdir), e)));
        handler_locations.push((path,cam.handler));
    }


    if let Some(path_buf) = cli.list_thumbnail.as_ref() {
        let path: &Path = path_buf.as_ref();
        let file = fs::canonicalize(path)
            .unwrap_or_else(|e| fail_main(&mut output, format!("error finding the absolute path of input file: {}", e)));
        let value : &(PathBuf, String) =  value_for_path(&file, &handler_locations)
            .unwrap_or_else(|| fail_main(&mut output,format!("Couldn't find handler responsible for a dir in the path of the input file")));
        let handler = get_adapter( &(value.1))?;
        output.file_list = Some(handler.list_thumbnail(&value.0,&file)
            .unwrap_or_else(|e| fail_main(&mut output, format!("handler {}: {}",handler.name(),e))));
        output.command_success=true;
        output.error_string=None;
    }else if let Some(path_buf) = cli.list_high_quality.as_ref(){
        let path: &Path = path_buf.as_ref();
        let file = fs::canonicalize(path)
            .unwrap_or_else(|e| fail_main(&mut output, format!("error finding the absolute path of input file: {}", e)));
        let value : &(PathBuf, String) =  value_for_path(&file, &handler_locations)
            .unwrap_or_else(|| fail_main(&mut output,format!("Couldn't find handler responsible for a dir in the path of the input file")));
        let handler = get_adapter( &(value.1))?;
        output.file_list = Some(handler.list_high_quality(&value.0,&file)
            .unwrap_or_else(|e| fail_main(&mut output, format!("handler {}: {}",handler.name(),e))));
        output.command_success=true;
        output.error_string=None;
    }else if let Some(_path_buf) = cli.get_related.as_ref(){

    }else{
        fail_main(&mut output, "Internal error: no action selected".to_string());
    }

    // Emit everything as JSON to stdout
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}
