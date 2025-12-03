/* main.rs

   This file is part of the media-interface project

   Copyright (c) 2025 Efthymios Kritikos

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <http://www.gnu.org/licenses/>.  */

use anyhow::{Result};
use clap::{Parser, ArgGroup};
use serde::{Deserialize, Serialize};
use std::path::{PathBuf,Path};
use std::process;
use std::fs;

mod gopro_hero_generic_1;
mod sony_ilcem4_1;
mod generic_single_file_items;
mod helpers;
mod gnss_tracker_generic;

/////////////////////////////////
// Command line interface data //
/////////////////////////////////
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

//////////////////////
// config file data //
//////////////////////
#[derive(Debug, Deserialize)]
struct Config {
    data_type: String,
    source_media: Vec<SourceMediaEntry>,
    errata: Option<Errata>,
}

#[derive(Debug, Deserialize)]
struct Errata {
    known_missing_files: Option<Vec<PathBuf>>,
}

#[derive(Debug, Deserialize)]
struct SourceMediaEntry {
    handler: String,
    card_subdir: PathBuf,
    path: PathBuf,
}

//////////////////
// Handler data //
//////////////////
trait SourceMediaInterface {
    fn list_thumbnail(&self, source_media_location: &Path, source_media_card: &Path, known_missing_file: Vec<PathBuf>) -> Result<Vec<FileItem>>;
    fn list_high_quality(&self, source_media_location: &Path, source_media_card: &Path, known_missing_file: Vec<PathBuf>) -> Result<Vec<FileItem>>;
    fn get_related(&self, source_media_location: &Path, source_media_file: &Path, known_missing_file: Vec<PathBuf>) -> Result<Vec<FileItem>>;
    fn name(&self) -> String;
}

fn get_handler(id: &str) -> Result<Box<dyn SourceMediaInterface>> {
    let factories: Vec<fn() -> Box<dyn SourceMediaInterface>> = vec![
        || Box::new(gopro_hero_generic_1::GoProInterface),
        || Box::new(sony_ilcem4_1::SonyInterface),
        || Box::new(generic_single_file_items::GenericSingleFileItem),
        || Box::new(gnss_tracker_generic::GNSSTrackerGeneric),
    ];

    for factory in factories {
        let instance = factory();
        if instance.name() == id {
            return Ok(instance);
        }
    }

    Err(anyhow::anyhow!("Unknown handler ID '{}'", id))
}

struct HandlerMapEntry{
    name: String,
    location: PathBuf,
}

////////////////////////////////
// Output JSON structure data //
////////////////////////////////
#[derive(Serialize, Deserialize)]
struct OutputJson {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_file: Option<String>,
}

//////////
// Main //
//////////
fn fail_main( data: &mut OutputJson, error: String ) -> ! {
    data.error_string=Some(error.clone());
    data.file_list=None;
    println!("{}", serde_json::to_string(&data).unwrap_or_else(|_| "Failed to serialise json".to_string()));
    eprintln!("{}", error);
    process::exit(1);
}

fn main() -> Result<()> {
    let mut output = OutputJson{
        data_type: "source_media_interface_api",
        version: env!("CARGO_PKG_VERSION"),
        command_success: false,
        file_list: None,
        error_string: Some("Uninitialised error message".to_string())
    };

    let cli = Cli::parse();

    //Get config file location
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

    // Load handler data from config data
    let mut handlers: Vec<HandlerMapEntry> = Vec::new();
    for cam in cfg.source_media {
        let path: PathBuf = config_file_path.parent().unwrap().join(&cam.path).join(&cam.card_subdir);
        let absolute_path: PathBuf = fs::canonicalize(&path)
            .unwrap_or_else(|e| fail_main(&mut output, format!("Error reading source media dir {:?}: {}", &path, e)));
        handlers.push(HandlerMapEntry{location:absolute_path,name:cam.handler});
    }

    let mut known_missing_files: Vec<PathBuf> = Vec::new();
    if let Some(errata) = cfg.errata && let Some(known_missing_files_input) = errata.known_missing_files {
        for file_input in known_missing_files_input{
            let path: PathBuf = config_file_path.parent().unwrap().to_path_buf();
            let absolute_path: PathBuf = fs::canonicalize(&path)
                .unwrap_or_else(|e| fail_main(&mut output, format!("Error reading errata missing file {:?}: {}", &path, e))).join(&file_input);
            known_missing_files.push(absolute_path);
        }
    }

    // execute the appropriate code of the appropriate handler
    if let Some(input_file) = cli.list_thumbnail.as_ref() {

        handle_action_with_input(&mut output, input_file, handlers, known_missing_files, true,
            |handler, base, file, known_missing_files| handler.list_thumbnail(base, file, known_missing_files));

    }else if let Some(input_file) = cli.list_high_quality.as_ref() {

        handle_action_with_input(&mut output, input_file, handlers, known_missing_files, true,
            |handler, base, file, known_missing_files| handler.list_high_quality(base, file, known_missing_files));

    }else if let Some(input_file) = cli.get_related.as_ref() {

        handle_action_with_input(&mut output, input_file, handlers, known_missing_files, false,
            |handler, base, file, known_missing_files| handler.get_related(base, file, known_missing_files));

    }else{
        fail_main(&mut output, "Internal error: no action selected".into());
    }

    // Output response from handler as json
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}

fn handle_action_with_input<F>(output: &mut OutputJson, input_file: &Path, handlers: Vec<HandlerMapEntry>, known_missing_files: Vec<PathBuf>, arg_is_card: bool, action: F, ) where
    F: Fn(&dyn SourceMediaInterface, &PathBuf, &PathBuf, Vec<PathBuf>) -> Result<Vec<FileItem>>,
{
    let file = fs::canonicalize(input_file)
        .unwrap_or_else(|e| fail_main(output, format!("error finding the absolute path of input file: {}", e)));

    let handler_entry = handlers.iter()
        .find(|entry| file.starts_with(&entry.location))
        .unwrap_or_else(|| fail_main(output, "Couldn't find handler responsible for a dir in the path of the input file".to_string()));

    let handler = get_handler(&handler_entry.name)
        .unwrap_or_else(|e| fail_main(output, format!("couldn't load handler {}: {}", handler_entry.name, e)));

    if arg_is_card && file.parent().unwrap() != handler_entry.location {
        fail_main(output, "List path entered is not a card directory".to_string());
    }

    output.file_list = Some(
        action(handler.as_ref(), &handler_entry.location, &file, known_missing_files)
            .unwrap_or_else(|e| fail_main(output, format!("handler {}: {}", handler.name(), e)))
    );

    output.command_success = true;
    output.error_string = None;
}

