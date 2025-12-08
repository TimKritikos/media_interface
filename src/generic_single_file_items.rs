/* single_file_items.rs - Generic handler logic for devices that use a single file for each item

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

use anyhow::{Result, anyhow};
use crate::SourceMediaInterface;
use std::path::{PathBuf,Path};
use crate::helpers::*;
use crate::FileItem;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;

pub struct GenericSingleFileItem;

fn filetype(ext: &str) -> Result<JsonFileInfoTypes> {
    match ext.to_lowercase().as_str() {
        "jpg"  => Ok(JsonFileInfoTypes{ file_type:FileImage, item_type:ItemImage }),
        "png"  => Ok(JsonFileInfoTypes{ file_type:FileImage, item_type:ItemImage }),
        "mp4"  => Ok(JsonFileInfoTypes{ file_type:FileVideo, item_type:ItemVideo }),
        "wav"  => Ok(JsonFileInfoTypes{ file_type:FileAudio, item_type:ItemAudio }),
        "3gpp" => Ok(JsonFileInfoTypes{ file_type:FileAudio, item_type:ItemAudio }),
        _ => Err(anyhow!("unknown file extension {:?} trying to determain file type", ext)),
    }
}

impl SourceMediaInterface for GenericSingleFileItem {
    fn list_thumbnail(&self, _source_media_location: &Path,  source_media_card: &Path, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card,|_filename: &str, input_ext: Option<&str>, _path: &PathBuf, path_str: &str|{
            let ext = input_ext.ok_or_else(|| anyhow!("Expected filter_dir to provide a file extension"))?;
            let types = filetype(ext)?;
            match types.file_type{
                FileVideo | FileAudio => Ok(Some(create_part_file(path_str.to_string(), types, 1, 1, None))),
                FileImage => Ok(Some(create_simple_file(path_str.to_string(), types, None)?)),
                _ => Err(anyhow!("Unrecognised extension '{}' in file '{}'", ext, path_str)),
            }
        })
    }
    fn list_high_quality(&self,  source_media_location: &Path,  source_media_card: &Path, known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        self.list_thumbnail(source_media_location, source_media_card, known_missing_files)
    }
    fn get_related(&self, _source_media_location: &Path, source_media_file: &Path, _known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        let extension = get_extension_str(source_media_file)?;
        let types = filetype(extension)?;
        match types.file_type{
            FileVideo => Ok(vec![create_part_file(source_media_file.to_string_lossy().into_owned(), types, 1, 1, None)]),
            FileImage => Ok(vec![create_simple_file(source_media_file.to_string_lossy().into_owned(), types, None)?]),
            _ => Err(anyhow!("unexpected file type")),
        }
    }
    fn name(&self) -> &'static str {
        "Generic-Single-File-Items"
    }
}
