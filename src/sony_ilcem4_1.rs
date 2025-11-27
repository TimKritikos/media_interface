use anyhow::{Result};
use crate::SourceMediaInterface;
use std::path::{PathBuf};
use crate::FileItem;
use crate::helpers::*;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;
use std::fs;

//TODO: acknoledge known_missing_files

fn filetype(file: &PathBuf, source_media_location: &PathBuf) -> Result<crate::helpers::JsonFileInfoTypes> {
        if  file.parent().unwrap().parent().unwrap().file_name().unwrap().to_str().unwrap() == "DCIM" &&
            file.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap() == source_media_location &&
            file.parent().unwrap().file_name().unwrap().to_str().unwrap().ends_with("MSDCF") {
            match file.extension().unwrap().to_str().unwrap(){
                "JPG" => Ok(JsonFileInfoTypes{ file_type:FileImage   ,item_type:ItemImage }),
                "ARW" => Ok(JsonFileInfoTypes{ file_type:FileImageRaw,item_type:ItemImage }),
                _ => Err(anyhow::anyhow!("unexpected input file extension"))
            }
        }else if file.parent().unwrap().parent().unwrap().file_name().unwrap().to_str().unwrap() == "M4ROOT" &&
                 file.parent().unwrap().parent().unwrap().parent().unwrap().file_name().unwrap().to_str().unwrap() == "PRIVATE" &&
                 file.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap() == source_media_location{
            match file.parent().unwrap().file_name().unwrap().to_str().unwrap() {
                "CLIP" => {
                    match file.extension().unwrap().to_str().unwrap() {
                        "MP4" => Ok(JsonFileInfoTypes{ file_type:FileVideo   ,item_type:ItemVideo }),
                        "XML" => Ok(JsonFileInfoTypes{ file_type:FileMetadata,item_type:ItemVideo }),
                        _ => Err(anyhow::anyhow!("unexpected input file extension"))
                    }
                },
                "THMBNL" => {
                    match file.extension().unwrap().to_str().unwrap() {
                        "JPG" => Ok(JsonFileInfoTypes{ file_type:FileImagePreview   ,item_type:ItemVideo }),
                        _ => Err(anyhow::anyhow!("unexpected input file extension"))
                    }
                }
                _ => Err(anyhow::anyhow!("Invalid input file path"))
            }
        }else{
            Err(anyhow::anyhow!("Invalid input file path"))
        }
}

enum VideoFiles{
    Thumbnail,
    Video,
    Metadata,
}

fn get_video_id( file:&PathBuf, file_type:VideoFiles ) -> String {
    let input_filename = file.as_path().file_name().unwrap().to_string_lossy().into_owned();

    match file_type {
        VideoFiles::Thumbnail => input_filename[1..=4].to_string(),
        VideoFiles::Video     => input_filename[1..=4].to_string(),
        VideoFiles::Metadata  => input_filename[1..=4].to_string(),
    }
}

fn create_video_file( input_file:&PathBuf, id:&String, file_type:VideoFiles ) -> PathBuf {
    let m4root=input_file.parent().unwrap().parent().unwrap();
    match file_type{
        VideoFiles::Video     => m4root.join("CLIP")  .join(format!("C{}.MP4",id)),
        VideoFiles::Metadata  => m4root.join("CLIP")  .join(format!("C{}M01.XML",id)),
        VideoFiles::Thumbnail => m4root.join("THMBNL").join(format!("C{}T01.JPG",id)),
    }
}

pub struct SonyInterface;

impl SourceMediaInterface for SonyInterface {
    fn list_thumbnail(&self,  source_media_location: &PathBuf,  source_media_card: &PathBuf, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        let mut files=Vec::<FileItem>::new();
        let dcim=source_media_card.join("DCIM/");
        if dcim.exists(){
            for imagedir in fs::read_dir(dcim)? {
                 let mut image_set=filter_dir(&imagedir?.path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
                    match ext {
                        Some("ARW") => {
                            if ! path.with_extension("JPG").exists(){
                                Ok(Some(create_simple_file(path_str.to_string(), filetype(path, source_media_location)?)?))
                            }else{
                                Ok(None)
                            }
                        }
                        Some("JPG") => {
                            Ok(Some(create_simple_file(path_str.to_string(), filetype(path, source_media_location)?)?))
                        }
                        Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path_str)),
                    }
                })?;
                 files.append(&mut image_set);
            }
        }
        let mut videos=filter_dir(source_media_card.join("PRIVATE/M4ROOT/THMBNL/").as_path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
            match ext {
                Some("JPG") => {
                    Ok(Some(create_part_file(path_str.to_string(), filetype(path, source_media_location)?,1,1,None)))
                }
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path_str)),
            }
        })?;
        files.append(&mut videos);
        return Ok(files);
    }
    fn list_high_quality(&self,  source_media_location: &PathBuf, source_media_card: &PathBuf, _known_missing_files: Vec<PathBuf> ) -> Result<Vec<FileItem>> {
        let mut files=Vec::<FileItem>::new();
        let dcim=source_media_card.join("DCIM/");
        if dcim.exists(){
            for imagedir in fs::read_dir(source_media_card.join(dcim))? {
                 let mut image_set=filter_dir(&imagedir?.path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
                    match ext {
                        Some("JPG") => {
                            if ! path.with_extension("ARW").exists(){
                                Ok(Some(create_simple_file(path_str.to_string(), filetype(path, source_media_location)?)?))
                            }else{
                                Ok(None)
                            }
                        }
                        Some("ARW") => {
                            Ok(Some(create_simple_file(path_str.to_string(), filetype(path,source_media_location)?)?))
                        }
                        Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path_str)),
                    }
                })?;
                 files.append(&mut image_set);
            }
        }
        let mut videos=filter_dir(source_media_card.join("PRIVATE/M4ROOT/CLIP/").as_path(),|_filename: &str, ext: Option<&str>, path:&PathBuf, path_str: &str|{
            match ext {
                Some("MP4") => {
                    Ok(Some(create_part_file(path_str.to_string(), filetype(path, source_media_location)?,1,1,None)))
                }
                Some("XML") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path_str)),
            }
        })?;
        files.append(&mut videos);
        return Ok(files);
    }
    fn get_related(&self, source_media_location: &PathBuf, source_media_file: &PathBuf, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        let mut items = Vec::<FileItem>::new();

        let input_file_types=filetype(source_media_file,source_media_location)?;

        match input_file_types.item_type{
            ItemImage => {
                let arw_path=source_media_file.with_extension("ARW");
                let jpg_path=source_media_file.with_extension("JPG");
                for i in [arw_path, jpg_path] {
                    if let Some(v) = create_simple_file_if_exists(&i, filetype(&i, source_media_location)?).unwrap() {
                        items.push(v);
                    }
                }
                return Ok(items);
            }
            ItemVideo => {
                let video_type = match input_file_types.file_type{
                    FileVideo => VideoFiles::Video,
                    FileImagePreview => VideoFiles::Thumbnail,
                    FileMetadata => VideoFiles::Metadata,
                    _ => { return Err(anyhow::anyhow!("Internal error"))}
                };

                let video_id = get_video_id(source_media_file,video_type);

                for i in [VideoFiles::Metadata, VideoFiles::Video, VideoFiles::Thumbnail] {
                    let file=create_video_file(source_media_file, &video_id, i);
                    if let Some(item) = create_part_file_that_exists(&file, filetype(&file, source_media_location)?, 1,1,None, &known_missing_files)?{
                        items.push(item);
                    }
                }
                return Ok(items);
            }
            _ => {
                Err(anyhow::anyhow!("Internal error"))
            }
        }
    }
    fn name(&self) -> String {
        return "Sony-ILCEM4-1".to_string()
    }
}
