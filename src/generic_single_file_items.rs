use anyhow::{Result};
use crate::SourceMediaInterface;
use std::path::{PathBuf};
use crate::helpers::*;
use crate::FileItem;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;

pub struct GenericSingleFileItem;

fn filetype(ext: &str) -> Result<crate::helpers::JsonFileInfoTypes> {
    match ext.to_lowercase().as_str() {
        "jpg"   => Ok(JsonFileInfoTypes{ file_type:FileImage,item_type:ItemImage }),
        "png"   => Ok(JsonFileInfoTypes{ file_type:FileImage,item_type:ItemImage }),
        "mp4"   => Ok(JsonFileInfoTypes{ file_type:FileVideo,item_type:ItemVideo }),
        "wav"   => Ok(JsonFileInfoTypes{ file_type:FileAudio,item_type:ItemAudio }),
        "3gpp"  => Ok(JsonFileInfoTypes{ file_type:FileAudio,item_type:ItemAudio }),
        _ => Err(anyhow::anyhow!("unkown file extension {:?} trying to determain file type", ext)),
    }
}

impl SourceMediaInterface for GenericSingleFileItem {
    fn list_thumbnail(&self, _source_media_location: &PathBuf,  source_media_card: &PathBuf, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card.as_path(),|_filename: &str, ext: Option<&str>, _path: &PathBuf, path_str: &str|{
            match ext {
                Some(a) => {
                    match a.to_lowercase().as_str() {
                        "jpg" | "png" | "mp4" | "wav" | "3gpp"
                          => {
                              let types=filetype(ext.unwrap())?;
                              match types.file_type{
                                FileVideo | FileAudio => Ok(Some(create_part_file(path_str.to_string(), types,1,1,None))),
                                FileImage => Ok(Some(create_simple_file(path_str.to_string(), types)?)),
                                _ => Err(anyhow::anyhow!("unexpected file type")),
                              }
                          }
                        _ => Err(anyhow::anyhow!("Unrecognised extension {}", path_str)),
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
        let types=filetype(source_media_file.extension().unwrap().to_string_lossy().as_ref())?;
        match types.file_type{
            FileVideo => Ok(vec![create_part_file(source_media_file.to_string_lossy().into_owned(), types,1,1,None)]),
            FileImage => Ok(vec![create_simple_file(source_media_file.to_string_lossy().into_owned(), types)?]),
            _ => Err(anyhow::anyhow!("unexpected file type")),
        }
    }
    fn name(&self) -> String {
        return "Generic-Single-File-Items".to_string()
    }
}
