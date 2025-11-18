use anyhow::{Result};
use crate::SourceMediaAdapter;
use std::path::{PathBuf};
use crate::FileItem;


pub struct SonyAdapter;

impl SourceMediaAdapter for SonyAdapter {
    fn list_thumbnail(&self,  _source_media_location: &PathBuf,  _source_media_card: &PathBuf ) -> Result<Vec<FileItem>> {
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn list_high_quality(&self,  _source_media_location: &PathBuf,  _source_media_card: &PathBuf ) -> Result<Vec<FileItem>> {
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn get_related(&self, _source_media_location: &PathBuf, _source_media_file: &PathBuf) -> Result<Vec<FileItem>>{
        return Err(anyhow::anyhow!("Not implemented"))
    }
    fn name(&self) -> String {
        return "Sony-ILCEM4-1".to_string()
    }
}
