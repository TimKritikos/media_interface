use anyhow::{Result};
use crate::SourceMediaInterface;
use std::path::{PathBuf};
use crate::helpers::*;
use crate::FileItem;

////////////////////////////////////////
//       GoPro Specific helpers       //
////////////////////////////////////////

fn get_gopro_video_part_id(filename:String) -> Result<u8> {
    return match filename.as_str().get(2..4).unwrap().parse::<u8>() {
        Ok(p) => Ok(p),
        Err(e) => { return Err(anyhow::anyhow!("Error parsing filename: {}",e)); }
    };
}

enum GoProVideoFileType{
    LowBitrateVideo,
    HighBitrateVideo,
    WavAudio,
    ThumbnailPhoto,
}

enum GoProPhotoFileType{
    JpegPhoto,
    RawPhoto,
}

fn create_gopro_photo_file(input_file:&PathBuf, file_type: GoProPhotoFileType ) -> Result<PathBuf> {

    let input_filename=input_file.as_path().file_name().unwrap().to_string_lossy().into_owned();

    let (name, _) = input_filename.rsplit_once('.').ok_or_else(|| anyhow::anyhow!("Failed to split gopro style filename from it's extension {:?}",input_filename))?;
    if name.len() < 1 { // minimal length, GX/L + NN + One character media id
        return Err(anyhow::anyhow!("Input gopro style filename without the extension was not long enough {:?}",name));
    }
    let new_extension = match file_type {
        GoProPhotoFileType::JpegPhoto => "JPG",
        GoProPhotoFileType::RawPhoto => "GPR",
    };
    Ok(input_file.parent().unwrap().join(format!("{name}.{new_extension}")))
}

fn create_gopro_video_file(input_file:&PathBuf, part:u8, file_type: GoProVideoFileType ) -> Result<PathBuf> {

    let input_filename=input_file.as_path().file_name().unwrap().to_string_lossy().into_owned();

    let (name, _) = input_filename.rsplit_once('.').ok_or_else(|| anyhow::anyhow!("Failed to split gopro style filename from it's extension {:?}",input_filename))?;

    if name.len() < 5 { // minimal length, GX/L + NN + One character media id
        return Err(anyhow::anyhow!("Input gopro style filename without the extension was not long enough {:?}",name));
    }

    let media_id = &name[4..];

    let new_prefix = match file_type {
        GoProVideoFileType::LowBitrateVideo => "GL",
        GoProVideoFileType::HighBitrateVideo => "GX",
        GoProVideoFileType::WavAudio => "GX",
        GoProVideoFileType::ThumbnailPhoto => "GX",
    };

    let new_part = format!("{:02}", part);

    let new_extension = match file_type {
        GoProVideoFileType::LowBitrateVideo => "LRV",
        GoProVideoFileType::HighBitrateVideo => "MP4",
        GoProVideoFileType::WavAudio => "WAV",
        GoProVideoFileType::ThumbnailPhoto => "THM",
    };

    Ok(input_file.parent().unwrap().join(format!("{new_prefix}{new_part}{media_id}.{new_extension}")))
}

pub struct GoProInterface;

////////////////////////////////////////
//         File parsing code          //
////////////////////////////////////////

fn filetype(ext: &str) -> Result<crate::helpers::JsonFileInfoTypes<'_>> {
    match ext {
        "THM" => Ok(JsonFileInfoTypes{ file_type:"image-preview",item_type:"video" }),
        "MP4" => Ok(JsonFileInfoTypes{ file_type:"video",        item_type:"video" }),
        "LRV" => Ok(JsonFileInfoTypes{ file_type:"video-preview",item_type:"video" }),
        "WAV" => Ok(JsonFileInfoTypes{ file_type:"audio"        ,item_type:"video" }),

        "JPG" => Ok(JsonFileInfoTypes{ file_type:"image"        ,item_type:"image" }),
        "GPR" => Ok(JsonFileInfoTypes{ file_type:"image-raw"    ,item_type:"image" }),
        _ => Err(anyhow::anyhow!("unkown file extension {:?} trying to determain file type", ext)),
    }
}

impl SourceMediaInterface for GoProInterface {
    fn list_thumbnail( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>> {
        filter_top_level_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &str|{
            match ext {
                Some("THM") => {
                    let part_id = get_gopro_video_part_id(filename.to_string())?;
                    if part_id != 1 {
                        for n in 1..part_id{
                            let n_file=create_gopro_video_file(&PathBuf::from(path),n,GoProVideoFileType::HighBitrateVideo)?;
                            if ! known_missing_files.contains(&n_file){
                                return Ok(None);
                            }
                        }
                    }
                    return Ok(Some(create_simple_file(path.to_string(), filetype(ext.unwrap())?)));
                }
                Some("JPG") => Ok(Some(create_simple_file(path.to_string(), filetype(ext.unwrap())?))),
                Some("MP4") | Some("GPR") | Some("LRV") | Some("WAV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path)),
            }
        })
    }
    fn list_high_quality( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>> {
        filter_top_level_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &str|{
            match ext {
                Some("MP4") => {
                    let part_id = get_gopro_video_part_id(filename.to_string())?;
                    if part_id != 1 {
                        for n in 1..part_id{
                            let n_file=create_gopro_video_file(&PathBuf::from(path),n,GoProVideoFileType::HighBitrateVideo)?;
                            if ! known_missing_files.contains(&n_file){
                                return Ok(None);
                            }
                        }
                    }
                    return Ok(Some(create_simple_file(path.to_string(), filetype(ext.unwrap())?)));
                }
                Some("JPG") => Ok(Some(create_simple_file(path.to_string(), filetype(ext.unwrap())?))),
                Some("THM") | Some("GPR") | Some("LRV") | Some("WAV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path)),
            }
        })
    }
    fn get_related(&self, _source_media_location: &PathBuf, source_media_file: &PathBuf, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        let mut items = Vec::<FileItem>::new();

        let ext = source_media_file
            .as_path()
            .extension()
            .and_then(|e| e.to_str());

        match ext {
            Some("THM")|Some("MP4")|Some("WAV")|Some("LRV") => {

                let mut part_count:u8 = 0;

                for part in 1..=99 {
                    let file = create_gopro_video_file(source_media_file,part,GoProVideoFileType::HighBitrateVideo)?;
                    if file.exists() || known_missing_files.contains(&file) {
                        part_count+=1;
                    }else if part_count==0 {
                        return Err(anyhow::anyhow!("Iniital video file not found"));
                    }else{
                        break;
                    }
                }

                for part in 1..=part_count {
                    let video_types = [
                        (GoProVideoFileType::HighBitrateVideo, false),
                        (GoProVideoFileType::LowBitrateVideo,  false),
                        (GoProVideoFileType::ThumbnailPhoto,   false),
                        (GoProVideoFileType::WavAudio,         true ),
                    ];

                    for (file_type_enum, optional) in video_types {
                        let file = create_gopro_video_file(source_media_file, part, file_type_enum)?;
                        if optional {
                            if let Some(item) = create_part_file_if_exists(&file, filetype(file.extension().unwrap().to_str().unwrap())?, part_count, part) {
                                items.push(item);
                            }
                        } else {
                            if let Some(item) = create_part_file_that_exists(&file, filetype(file.extension().unwrap().to_str().unwrap())?, part_count, part, &known_missing_files)?{
                                items.push(item);
                            }
                        }
                    }
                }
            },
            Some("JPG")|Some("GPR") => {
                for file_type_enum in [GoProPhotoFileType::JpegPhoto,GoProPhotoFileType::RawPhoto] {
                    let file = create_gopro_photo_file(source_media_file, file_type_enum)?;
                    if let Some(v) = create_simple_file_if_exists(&file, filetype(file.extension().unwrap().to_str().unwrap())?) {
                        items.push(v);
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid input file"));
            }
        };
        return Ok(items);
    }

    fn name(&self) -> String {
        return "GoPro-Hero-Generic-1".to_string()
    }
}
