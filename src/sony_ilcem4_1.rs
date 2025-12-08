/* sony_ilcem4_1.rs - Handler logic for Sony ILCEM4 cameras

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

use anyhow::{Result, anyhow, Context};
use crate::SourceMediaInterface;
use std::path::{PathBuf,Path};
use crate::FileItem;
use crate::helpers::*;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;
use std::fs;

fn filetype(file: &Path, source_media_location: &Path) -> Result<JsonFileInfoTypes> {
    let extension = get_extension_str(file)?;
    let file_str = file.to_string_lossy();
    let parent_folder = file.parent().context("File has no parent directory")?;
    let grandparent_folder = parent_folder.parent().context("File has no grandparent directory")?;
    let grandparent_name = osstr_to_str(grandparent_folder.file_name().ok_or_else(|| anyhow!("Failed to get name of grandparent folder"))?)?;

    if grandparent_name == "DCIM"{
        let parent_folder_name = osstr_to_str(parent_folder.file_name()
            .ok_or_else(|| anyhow!("failed to get name of parent folder"))?)?;

        let expected_source_media_location = grandparent_folder.parent().context("Traversing path backwards, expected to reach card dir but failed")?
                                                               .parent().context("Traversing path backwards, expected to reach source media dir but failed")?;

        if parent_folder_name.ends_with("MSDCF") && expected_source_media_location == source_media_location {
            return match extension{
                "JPG" => Ok(JsonFileInfoTypes{ file_type:FileImage,    item_type:ItemImage }),
                "ARW" => Ok(JsonFileInfoTypes{ file_type:FileImageRaw, item_type:ItemImage }),
                _ => Err(anyhow!("unexpected input file extension '{}' in file '{}'", extension, file_str))
            }
        }

    }

    if grandparent_name == "M4ROOT" {
        let private_folder = grandparent_folder.parent().context("Traversing path backwards, expected to reach PRIVATE folder but failed")?;
        let private_folder_name = osstr_to_str(private_folder.file_name().ok_or_else(|| anyhow!("failed to get filename of what's expected to be the PRIVATE folder"))?)?;
        let expected_source_media_location = private_folder.parent().context("Traversing path backwards, expected to reach card dir but failed")?
                                                           .parent().context("Traversing path backwards, expected to reach source media dir but failed")?;

        if private_folder_name == "PRIVATE" && expected_source_media_location == source_media_location {
            let m4root_subfolder_name = osstr_to_str(parent_folder.file_name().ok_or_else(|| anyhow!("failed to get filename of what's expected to be the M4ROOT folder"))?)?;
            return match m4root_subfolder_name {
                "CLIP" => {
                    match extension {
                        "MP4" => Ok(JsonFileInfoTypes{ file_type:FileVideo,    item_type:ItemVideo }),
                        "XML" => Ok(JsonFileInfoTypes{ file_type:FileMetadata, item_type:ItemVideo }),
                        _ => Err(anyhow!("unexpected input file extension '{}' in file '{}'", extension, file_str))
                    }
                },
                "THMBNL" => {
                    match extension {
                        "JPG" => Ok(JsonFileInfoTypes{ file_type:FileImagePreview, item_type:ItemVideo }),
                        _ => Err(anyhow!("unexpected input file extension '{}' in file '{}'", extension, file_str))
                    }
                }
                _ => Err(anyhow!("File '{}' in M4ROOT directory has an invalid subfolder name '{}'", file_str, m4root_subfolder_name))
            }
        }
    }

    Err(anyhow!("File path not in expected directory structure '{}'", file_str))
}

enum VideoFiles{
    Thumbnail,
    Video,
    Metadata,
}

fn get_video_id( file:&Path, file_type:VideoFiles ) -> Result<String> {
    let input_filename = file.file_name().ok_or_else(|| anyhow!("Couldn't get filename of video file"))?.to_string_lossy();

    Ok( match file_type {
        VideoFiles::Thumbnail => input_filename[1..=4].to_string(),
        VideoFiles::Video     => input_filename[1..=4].to_string(),
        VideoFiles::Metadata  => input_filename[1..=4].to_string(),
    } )
}

fn create_video_file( input_file:&Path, id:&String, file_type:VideoFiles ) -> Result<PathBuf> {
    let m4root = input_file.parent().context("Traversing path backwards, expected to reach m4root subfolder but failed")?
                           .parent().context("Traversing path backwards, expected to reach m4root dir but failed")?;
    Ok ( match file_type{
        VideoFiles::Video     => m4root.join("CLIP")  .join(format!("C{}.MP4", id)),
        VideoFiles::Metadata  => m4root.join("CLIP")  .join(format!("C{}M01.XML", id)),
        VideoFiles::Thumbnail => m4root.join("THMBNL").join(format!("C{}T01.JPG", id)),
    } )
}

pub struct SonyInterface;

impl SourceMediaInterface for SonyInterface {
    //TODO: handle case where the thumbnail is in the known missing files and the item needs to be represented by something else

    fn list_thumbnail(&self,  source_media_location: &Path,  source_media_card: &Path, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        let mut files = Vec::<FileItem>::new();
        let dcim = source_media_card.join("DCIM/");
        if dcim.exists(){
            for imagedir in fs::read_dir(dcim)? {
                let mut image_set = filter_dir(&imagedir?.path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
                    match ext {
                        Some("ARW") => {
                            if ! path.with_extension("JPG").exists(){
                                Ok(Some(create_simple_file(path_str.to_string(), filetype(path, source_media_location)?, None)?))
                            }else{
                                Ok(None)
                            }
                        }
                        Some("JPG") => {
                            Ok(Some(create_simple_file(path_str.to_string(), filetype(path, source_media_location)?, None)?))
                        }
                        Some(_) | None => Err(anyhow!("Unexpected file {}", path_str)),
                    }
                })?;
                 files.append(&mut image_set);
            }
        }
        let mut videos = filter_dir(source_media_card.join("PRIVATE/M4ROOT/THMBNL/").as_path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
            match ext {
                Some("JPG") => {
                    Ok(Some(create_part_file(path_str.to_string(), filetype(path, source_media_location)?, 1, 1, None)))
                }
                Some(_) | None => Err(anyhow!("Unexpected file {}", path_str)),
            }
        })?;
        files.append(&mut videos);

        Ok(files)
    }
    fn list_high_quality(&self,  source_media_location: &Path, source_media_card: &Path, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        let mut files = Vec::<FileItem>::new();
        let dcim = source_media_card.join("DCIM/");
        if dcim.exists(){
            for imagedir in fs::read_dir(source_media_card.join(dcim))? {
                 let mut image_set = filter_dir(&imagedir?.path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
                    match ext {
                        Some("JPG") => {
                            if ! path.with_extension("ARW").exists(){
                                Ok(Some(create_simple_file(path_str.to_string(), filetype(path, source_media_location)?, None)?))
                            }else{
                                Ok(None)
                            }
                        }
                        Some("ARW") => {
                            Ok(Some(create_simple_file(path_str.to_string(), filetype(path, source_media_location)?, None)?))
                        }
                        Some(_) | None => Err(anyhow!("Unexpected file {}", path_str)),
                    }
                })?;
                 files.append(&mut image_set);
            }
        }
        let mut videos = filter_dir(source_media_card.join("PRIVATE/M4ROOT/CLIP/").as_path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
            match ext {
                Some("MP4") => {
                    Ok(Some(create_part_file(path_str.to_string(), filetype(path, source_media_location)?, 1, 1, None)))
                }
                Some("XML") => Ok(None),
                Some(_) | None => Err(anyhow!("Unexpected file {}", path_str)),
            }
        })?;
        files.append(&mut videos);

        Ok(files)
    }
    fn get_related(&self, source_media_location: &Path, source_media_file: &Path, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        let mut items = Vec::<FileItem>::new();

        let input_file_types = filetype(source_media_file, source_media_location)?;

        match input_file_types.item_type{
            ItemImage => {
                let arw_path = source_media_file.with_extension("ARW");
                let jpg_path = source_media_file.with_extension("JPG");
                for i in [arw_path, jpg_path] {
                    if let Some(v) = create_simple_file_if_exists(&i, filetype(&i, source_media_location)?, None)? {
                        items.push(v);
                    }
                }

                Ok(items)
            }
            ItemVideo => {
                let video_type = match input_file_types.file_type{
                    FileVideo => VideoFiles::Video,
                    FileImagePreview => VideoFiles::Thumbnail,
                    FileMetadata => VideoFiles::Metadata,
                    _ => { return Err(anyhow!("Internal error"))}
                };

                let video_id = get_video_id(source_media_file, video_type)?;

                for i in [VideoFiles::Metadata, VideoFiles::Video, VideoFiles::Thumbnail] {
                    let file = create_video_file(source_media_file, &video_id, i)?;
                    if let Some(item) = create_part_file_that_exists(&file, filetype(&file, source_media_location)?, 1, 1, None, &known_missing_files)?{
                        items.push(item);
                    }
                }

                Ok(items)
            }
            _ => {
                Err(anyhow!("Internal error"))
            }
        }
    }
    fn name(&self) -> &'static str {
        "Sony-ILCEM4-1"
    }
}
