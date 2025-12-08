/* gnss_tracker_generic.rs - Generic handler for devices that create GNSS tracks and store the
 * items either with a single file or with the same filename and different extensions like with the
 * BasicAirData's "GPS Logger" Android app

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

pub struct GNSSTrackerGeneric;

const FILE_TYPES: JsonFileInfoTypes = JsonFileInfoTypes {
    file_type: FileGNSSTrack,
    item_type: ItemGNSSTrack,
};

impl SourceMediaInterface for GNSSTrackerGeneric {
    fn list_thumbnail(&self, _source_media_location: &Path,  source_media_card: &Path, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card,|_filename: &str, input_ext: Option<&str>, path: &PathBuf, path_str: &str|{
            let ext = input_ext.ok_or_else(|| anyhow!("Expected filter_dir to provide a file extension"))?;
            match ext.to_lowercase().as_str() {
                "gpx" => {
                    Ok(Some(create_simple_file(path_str.to_string(), FILE_TYPES)?))
                }
                "kml" => {
                    if ! path.with_extension("gpx").exists() {
                        Ok(Some(create_simple_file(path_str.to_string(), FILE_TYPES)?))
                    }else{
                        Ok(None)
                    }
                }
                "txt" => {
                    if ! path.with_extension("gpx").exists() && ! path.with_extension("kml").exists() {
                        Ok(Some(create_simple_file(path_str.to_string(), FILE_TYPES)?))
                    }else{
                        Ok(None)
                    }
                }
                _ => Err(anyhow!("Unrecognised extension '{}' in file '{}'", ext, path_str)),
            }
        })
    }
    fn list_high_quality(&self,  source_media_location: &Path,  source_media_card: &Path, known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        self.list_thumbnail(source_media_location, source_media_card, known_missing_files)
    }
    fn get_related(&self, _source_media_location: &Path, source_media_file: &Path, _known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        let mut items = Vec::<FileItem>::new();

        for extension in ["gpx", "kml", "txt"]{
            if let Ok(Some(item)) = create_simple_file_if_exists(&source_media_file.with_extension(extension), FILE_TYPES) {
                items.push(item);
            }
        }

        Ok(items)
    }
    fn name(&self) -> &'static str {
        "GNSS-Tracker-Generic"
    }
}
