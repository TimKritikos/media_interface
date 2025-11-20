use anyhow::{Result};
use crate::SourceMediaInterface;
use std::path::{PathBuf};
use crate::helpers::*;
use crate::FileItem;

pub struct GenericSingleFileItem;

fn filetype(ext: &str) -> Result<crate::helpers::JsonFileInfoTypes<'_>> {
    match ext {
        "JPG"|"jpg" => Ok(JsonFileInfoTypes{ file_type:"image",item_type:"image" }),
        "MP4"|"mp4" => Ok(JsonFileInfoTypes{ file_type:"video",item_type:"video" }),
        "WAV"|"wav" => Ok(JsonFileInfoTypes{ file_type:"audio",item_type:"audio" }),
        _ => Err(anyhow::anyhow!("unkown file extension {:?} trying to determain file type", ext)),
    }
}

impl SourceMediaInterface for GenericSingleFileItem {
    fn list_thumbnail(&self, _source_media_location: &PathBuf,  source_media_card: &PathBuf, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        filter_top_level_dir(source_media_card.as_path(),|_filename: &str, ext: Option<&str>, path: &str|{
            match ext {
                Some("jpg") | Some("JPG") |
                Some("mp4") | Some("MP4") |
                Some("wav") | Some("WAV") => Ok(Some(create_simple_file(path.to_string(), filetype(ext.unwrap())?))),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path)),
            }
        })
    }
    fn list_high_quality(&self,  source_media_location: &PathBuf,  source_media_card: &PathBuf, known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        self.list_thumbnail(source_media_location,source_media_card,known_missing_files)
    }
    fn get_related(&self, _source_media_location: &PathBuf, source_media_file: &PathBuf, _known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        Ok(vec![create_simple_file(source_media_file.to_string_lossy().into_owned(), filetype(source_media_file.extension().unwrap().to_string_lossy().as_ref())?)])
    }
    fn name(&self) -> String {
        return "Generic-Single-File-Items".to_string()
    }
}
