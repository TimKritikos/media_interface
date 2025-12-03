/* helpers.rs - Reusable code for all handlers

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
use std::path::{Path, PathBuf};
use std::fs;
use crate::FileItem;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;

pub fn osstr_to_str(os: &std::ffi::OsStr) -> Result<&str> {
    os.to_str().ok_or_else(|| anyhow!("Invalid UTF-8 in {:?}", os))
}

pub fn get_extension_str(file:&Path) -> Result<&str> {
    osstr_to_str(file.extension().ok_or_else(|| anyhow!("File has no extension"))?)
}

pub fn for_each_file_type<F>(dir: &Path, mut f: F) -> Result<()>
where
    F: FnMut(&PathBuf, String, String, Option<&str>) -> Result<()>,
{
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let ext = get_extension_str(&path).ok();

        let path_str = osstr_to_str(path.as_os_str())?.to_string();

        let filename = path.file_name().ok_or_else(|| anyhow!("Failed to get filename"))?;
        let filename_str = osstr_to_str(filename)?.to_string();

        match f(&path, filename_str, path_str, ext){
            Ok(()) => {},
            Err(e) => { return Err(e); }
        }
    }
    Ok(())
}

#[allow(clippy::enum_variant_names)]
pub enum FileType{
   FileVideo,
   FileVideoPreview,
   #[allow(dead_code)]
   FileVideoRaw,

   FileImage,
   FileImagePreview,
   FileImageRaw,

   FileAudio,

   FileMetadata,

   FileGNSSTrack,
}

#[allow(clippy::enum_variant_names)]
#[derive(PartialEq)]
pub enum ItemType{
    ItemVideo,
    ItemImage,
    ItemAudio,
    ItemGNSSTrack,
}

#[allow(clippy::enum_variant_names)]
pub struct JsonFileInfoTypes{
    pub file_type: FileType,
    pub item_type: ItemType,
}

pub fn create_simple_file_if_exists(file_path:&Path, json_file_info: JsonFileInfoTypes) -> Result<Option<FileItem>> {
    if file_path.exists(){
        Ok(Some(create_simple_file(file_path.to_string_lossy().into_owned(), json_file_info)?))
    }else{
        Ok(None)
    }
}

//pub fn create_simple_file_that_exists(file_path:&PathBuf, json_file_info: JsonFileInfoTypes, known_missing_files: &Vec<PathBuf>) -> Result<Option<FileItem>> {
//    if file_path.exists(){
//        Ok(Some(create_simple_file(file_path.to_string_lossy().into_owned(), json_file_info)?))
//    }else{
//        if known_missing_files.contains(&file_path){
//            Ok(None)
//        }else{
//            Err(anyhow::anyhow!("File {:?} expected to exist", file_path.to_string_lossy().into_owned()))
//        }
//    }
//}

pub fn create_part_file_if_exists(file_path:&Path, json_file_info: JsonFileInfoTypes, part_count:u8, part_num:u8, metadata_file:Option<String>) -> Option<FileItem> {
    if file_path.exists(){
        Some(create_part_file(file_path.to_string_lossy().into_owned(), json_file_info, part_count, part_num, metadata_file))
    }else{
        None
    }
}

pub fn create_part_file_that_exists(file_path:&PathBuf, json_file_info: JsonFileInfoTypes, part_count:u8, part_num:u8, metadata_file:Option<String>, known_missing_files: &[PathBuf]) -> Result<Option<FileItem>> {
    if file_path.exists(){
        Ok(Some(create_part_file(file_path.to_string_lossy().into_owned(), json_file_info, part_count, part_num, metadata_file)))
    }else if known_missing_files.contains(file_path){
        Ok(None)
    }else{
        Err(anyhow::anyhow!("File {:?} expected to exist", file_path.to_string_lossy().into_owned()))
    }
}

pub fn create_simple_file(file_path:String, json_file_info: JsonFileInfoTypes) -> Result<FileItem> {
    if json_file_info.item_type == ItemType::ItemVideo { // TODO: Make this a compile time check
        return Err(anyhow::anyhow!("Internal error: Tried to generate simple file for video item"));
    }
    Ok(create_simple_file_unchecked(file_path, json_file_info))
}

#[allow(clippy::redundant_field_names)]
fn create_simple_file_unchecked(file_path:String, json_file_info: JsonFileInfoTypes) -> FileItem {
    FileItem{
        file_path:file_path,
        file_type:match json_file_info.file_type{
            FileVideo         => "video",
            FileVideoPreview  => "video-preview",
            FileVideoRaw      => "video-raw",
            FileImage         => "image",
            FileImagePreview  => "image-preview",
            FileImageRaw      => "image-raw",
            FileAudio         => "audio",
            FileMetadata      => "metadata",
            FileGNSSTrack     => "gnss-track"
        }.to_string(),
        item_type:match json_file_info.item_type{
            ItemVideo     => "video",
            ItemImage     => "image",
            ItemAudio     => "audio",
            ItemGNSSTrack => "gnss-track",
        }.to_string(),
        part_count :    None,
        part_num :      None,
        metadata_file : None,
    }
}


pub fn create_part_file(file_path:String, json_file_info: JsonFileInfoTypes, part_count:u8, part_num:u8, metadata_file:Option<String>) -> FileItem {
    let mut ret = create_simple_file_unchecked(file_path, json_file_info);
    ret.part_count = Some(part_count);
    ret.part_num = Some(part_num);
    ret.metadata_file = metadata_file;
    ret
}

pub fn filter_dir<F>(source_dir: &Path, mut filter: F) -> Result<Vec<FileItem>>
where
    F:FnMut(&str, Option<&str>, &PathBuf, &str)->Result<Option<FileItem>>,
{
    let mut items = Vec::<FileItem>::new();

    for_each_file_type(source_dir,
        |path:&PathBuf, filename: String, path_str: String, ext: Option<&str>| {
            if let Some(item) = filter(&filename, ext, path, &path_str)? {
                items.push(item);
            }
            Ok(())
        }
    )
    .map_err(|err| anyhow::anyhow!("Error filtering dir '{}': {}",source_dir.display(), err))?;

    Ok(items)
}
