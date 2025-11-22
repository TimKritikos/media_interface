use anyhow::{Result};
use std::path::{Path,PathBuf};
use std::fs;
use crate::FileItem;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;

pub fn for_each_file_type<F>(dir: &Path, mut f: F) -> Result<()>
where
    F: FnMut(&std::path::Path, String, String, Option<&str>) -> Result<()>,
{
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let ext = path
            .extension()
            .and_then(|e| e.to_str()); // Option<&str>

        let path_str = path
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();

        let filename = path
            .file_name()
            .unwrap()
            .to_string_lossy().into_owned();

        match f(&path, filename, path_str, ext){
            Ok(()) => {},
            Err(e) => { return Err(e); }
        }
    }
    Ok(())
}

pub enum FileType{
   FileVideo,
   FileVideoPreview,
   #[allow(dead_code)]
   FileVideoRaw,

   FileImage,
   FileImagePreview,
   FileImageRaw,

   FileAudio,
}

pub enum ItemType{
    ItemVideo,
    ItemImage,
    ItemAudio,
}

pub struct JsonFileInfoTypes{
    pub file_type: FileType,
    pub item_type: ItemType,
}

pub fn create_simple_file_if_exists(file_path:&PathBuf, json_file_info: JsonFileInfoTypes) -> Option<FileItem> {
    if file_path.exists(){
        Some(create_simple_file(file_path.to_string_lossy().into_owned(),json_file_info))
    }else{
        None
    }
}

//pub fn create_simple_file_that_exists(file_path:&PathBuf, json_file_info: JsonFileInfoTypes) -> Result<FileItem> {
//    if file_path.exists(){
//        Ok(create_simple_file(file_path.to_string_lossy().into_owned(),json_file_info))
//    }else{
//        Err(anyhow::anyhow!("File {:?} expected to exist", file_path.to_string_lossy().into_owned()))
//    }
//}

pub fn create_part_file_if_exists(file_path:&PathBuf, json_file_info: JsonFileInfoTypes, part_count:u8, part_num:u8) -> Option<FileItem> {
    if file_path.exists(){
        Some(create_part_file(file_path.to_string_lossy().into_owned(),json_file_info,part_count,part_num))
    }else{
        None
    }
}

pub fn create_part_file_that_exists(file_path:&PathBuf, json_file_info: JsonFileInfoTypes, part_count:u8, part_num:u8, known_missing_files: &Vec<PathBuf>) -> Result<Option<FileItem>> {
    if file_path.exists(){
        Ok(Some(create_part_file(file_path.to_string_lossy().into_owned(),json_file_info,part_count,part_num)))
    }else{
        if known_missing_files.contains(&file_path){
            Ok(None)
        }else{
            Err(anyhow::anyhow!("File {:?} expected to exist", file_path.to_string_lossy().into_owned()))
        }
    }
}

pub fn create_simple_file(file_path:String, json_file_info: JsonFileInfoTypes) -> FileItem {
    FileItem{
        file_path:file_path,
        file_type:match json_file_info.file_type{
            FileVideo          => "video",
            FileVideoPreview  => "video-preview",
            FileVideoRaw      => "video-raw",
            FileImage          => "image",
            FileImagePreview  => "image-preview",
            FileImageRaw      => "image-raw",
            FileAudio          => "audio",
        }.to_string(),
        item_type:match json_file_info.item_type{
            ItemVideo => "video",
            ItemImage => "image",
            ItemAudio => "audio",
        }.to_string(),
        part_count:None,
        part_num:None,
    }
}

pub fn create_part_file(file_path:String, json_file_info: JsonFileInfoTypes, part_count:u8, part_num:u8) -> FileItem {
    let mut ret=create_simple_file(file_path,json_file_info);
    ret.part_count=Some(part_count);
    ret.part_num=Some(part_num);
    return ret;
}

pub fn filter_top_level_dir<F>(source_dir: &Path, mut filter: F) -> Result<Vec<FileItem>>
where
    F:FnMut(&str, Option<&str>, &str)->Result<Option<FileItem>>,
{
    let mut items = Vec::<FileItem>::new();

    for_each_file_type(source_dir, |_path:&Path, filename: String, path_str: String, ext: Option<&str>| {
        if let Some(item) = filter(&filename, ext, &path_str)? {
            items.push(item);
        }
        Ok(())
    })
    .map_err(|err| anyhow::anyhow!("Error traversing directory: {}", err))?;

    Ok(items)
}
