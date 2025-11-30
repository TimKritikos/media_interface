use anyhow::{Result};
use crate::SourceMediaInterface;
use std::path::{PathBuf};
use crate::helpers::*;
use crate::FileItem;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;

pub struct GNSSTrackerGeneric;

impl SourceMediaInterface for GNSSTrackerGeneric {
    fn list_thumbnail(&self, _source_media_location: &PathBuf,  source_media_card: &PathBuf, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card.as_path(),|_filename: &str, ext: Option<&str>, path: &PathBuf, path_str: &str|{
            match ext {
                Some(a) => {
                    match a.to_lowercase().as_str() {
                        "gpx" => {
                            Ok(Some(create_simple_file(path_str.to_string(), JsonFileInfoTypes{ file_type:FileGNSSTrack,item_type:ItemGNSSTrack })?))
                        }
                        "kml" => {
                            if ! path.with_extension("gpx").exists() {
                                Ok(Some(create_simple_file(path_str.to_string(), JsonFileInfoTypes{ file_type:FileGNSSTrack,item_type:ItemGNSSTrack })?))
                            }else{
                                Ok(None)
                            }
                        }
                        "txt" => {
                            if ! path.with_extension("gpx").exists() && ! path.with_extension("kml").exists() {
                                Ok(Some(create_simple_file(path_str.to_string(), JsonFileInfoTypes{ file_type:FileGNSSTrack,item_type:ItemGNSSTrack })?))
                            }else{
                                Ok(None)
                            }
                        }
                        _ => Err(anyhow::anyhow!("Unrecognised extension \"{}\" in file {}",a.to_lowercase(), path_str)),
                    }
                }
                None => Err(anyhow::anyhow!("File has no extension {}", path_str)),
            }
        })
    }
    fn list_high_quality(&self,  source_media_location: &PathBuf,  source_media_card: &PathBuf, known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        self.list_thumbnail(source_media_location,source_media_card,known_missing_files)
    }
    fn get_related(&self, _source_media_location: &PathBuf, source_media_file: &PathBuf, _known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        let mut items = Vec::<FileItem>::new();

        for extension in ["gpx", "kml", "txt"]{
            if let Ok(Some(item)) = create_simple_file_if_exists(&source_media_file.with_extension(extension), JsonFileInfoTypes{ file_type:FileGNSSTrack,item_type:ItemGNSSTrack }) {
                items.push(item);
            }
        }

        return Ok(items)
    }
    fn name(&self) -> String {
        return "GNSS-tracker-generic".to_string()
    }
}
