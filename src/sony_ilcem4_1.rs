use anyhow::{Result};
use crate::SourceMediaInterface;
use std::path::{PathBuf};
use crate::FileItem;


pub struct SonyInterface;

impl SourceMediaInterface for SonyInterface {
    fn list_thumbnail(&self,  _source_media_location: &PathBuf,  _source_media_card: &PathBuf, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn list_high_quality(&self,  _source_media_location: &PathBuf,  _source_media_card: &PathBuf, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn get_related(&self, _source_media_location: &PathBuf, _source_media_file: &PathBuf, _known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn name(&self) -> String {
        return "Sony-ILCEM4-1".to_string()
    }
}
