use anyhow::{Result};
use crate::SourceMediaInterface;
use std::path::{PathBuf};
use crate::helpers::*;
use crate::FileItem;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;

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

struct PartCount{
    existing_parts_count:u8,
    all_parts_count:u8,
}

fn count_gopro_parts( base_file:&PathBuf, known_missing_files: &Vec<PathBuf> ) -> Result<PartCount> {

    let mut parts:PartCount=PartCount{existing_parts_count:0, all_parts_count:0};

    for part in 1..=99 {
        let file = create_gopro_video_file(base_file,part,GoProVideoFileType::HighBitrateVideo)?;
        if file.exists() {
            parts.existing_parts_count+=1;
            parts.all_parts_count+=1;
        }else if known_missing_files.contains(&file) {
            parts.all_parts_count+=1;
        }else if part==0 {
            return Err(anyhow::anyhow!("Iniital video file not found"));
        }else{
            break;
        }
    }

    return Ok(parts);
}

fn filetype(ext: &str) -> Result<crate::helpers::JsonFileInfoTypes> {
    match ext {
        "THM" => Ok(JsonFileInfoTypes{ file_type:FileImagePreview ,item_type:ItemVideo }),
        "MP4" => Ok(JsonFileInfoTypes{ file_type:FileVideo        ,item_type:ItemVideo }),
        "LRV" => Ok(JsonFileInfoTypes{ file_type:FileVideoPreview ,item_type:ItemVideo }),
        "WAV" => Ok(JsonFileInfoTypes{ file_type:FileAudio        ,item_type:ItemVideo }),

        "JPG" => Ok(JsonFileInfoTypes{ file_type:FileImage        ,item_type:ItemImage }),
        "GPR" => Ok(JsonFileInfoTypes{ file_type:FileImageRaw     ,item_type:ItemImage }),
        _ => Err(anyhow::anyhow!("unkown file extension {:?} trying to determain file type", ext)),
    }
}

impl SourceMediaInterface for GoProInterface {
    //TODO: handle case where the thumbnail is in the known missing files and the item needs to be
    //represented by something else
    fn list_thumbnail( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &PathBuf, path_str: &str|{
            match ext {
                Some("THM") => {
                    let part_id = get_gopro_video_part_id(filename.to_string())?;
                    if part_id != 1 {
                        for n in 1..part_id{
                            let n_file=create_gopro_video_file(&path,n,GoProVideoFileType::HighBitrateVideo)?;
                            if ! known_missing_files.contains(&n_file){
                                return Ok(None);
                            }
                        }
                    }

                    let part_count = count_gopro_parts(&path, &known_missing_files).unwrap();

                    return Ok(Some(create_part_file(path_str.to_string(), filetype(ext.unwrap())?, part_count.existing_parts_count, 1, Some(path.with_extension("MP4").to_string_lossy().into_owned()))));
                }
                Some("JPG") => Ok(Some(create_simple_file(path_str.to_string(), filetype(ext.unwrap())?)?)),
                Some("MP4") | Some("GPR") | Some("LRV") | Some("WAV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path_str)),
            }
        })
    }
    fn list_high_quality( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &PathBuf, path_str: &str|{
            match ext {
                Some("MP4") => {
                    let part_id = get_gopro_video_part_id(filename.to_string())?;
                    if part_id != 1 {
                        for n in 1..part_id{
                            let n_file=create_gopro_video_file(&path,n,GoProVideoFileType::HighBitrateVideo)?;
                            if ! known_missing_files.contains(&n_file){
                                return Ok(None);
                            }
                        }
                    }

                    let part_count = count_gopro_parts(&path, &known_missing_files).unwrap();

                    return Ok(Some(create_part_file(path_str.to_string(), filetype(ext.unwrap())?, part_count.existing_parts_count, 1, Some(path_str.to_string()))));
                }
                Some("GPR") | Some ("JPG") => {
                    if ext == Some("GPR") || !create_gopro_photo_file(path, GoProPhotoFileType::RawPhoto).unwrap().exists() {
                        return Ok(Some(create_simple_file(path_str.to_string(), filetype(ext.unwrap())?)?))
                    }
                    return Ok(None);
                }
                Some("THM") | Some("LRV") | Some("WAV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path_str)),
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

                let part_count = count_gopro_parts(source_media_file, &known_missing_files).unwrap();

                let mut existing_part_number:u8 = 1;
                for part in 1..=part_count.all_parts_count {
                    let video_types = [
                        (GoProVideoFileType::HighBitrateVideo, false),
                        (GoProVideoFileType::LowBitrateVideo,  false),
                        (GoProVideoFileType::ThumbnailPhoto,   false),
                        (GoProVideoFileType::WavAudio,         true ),
                    ];

                    let mut existed=false;
                    for (file_type_enum, optional) in video_types {
                        let file = create_gopro_video_file(source_media_file, part, file_type_enum)?;
                        if optional {
                            if let Some(item) = create_part_file_if_exists(&file, filetype(file.extension().unwrap().to_str().unwrap())?, part_count.existing_parts_count, existing_part_number, None) {
                                items.push(item);
                                existed=true;
                            }
                        } else {
                            if let Some(item) = create_part_file_that_exists(&file, filetype(file.extension().unwrap().to_str().unwrap())?, part_count.existing_parts_count, existing_part_number,None, &known_missing_files)?{
                                items.push(item);
                                existed=true;
                            }
                        }
                    }
                    if existed {
                        existing_part_number+=1;
                    }
                }
            },
            Some("JPG")|Some("GPR") => {
                for file_type_enum in [GoProPhotoFileType::JpegPhoto,GoProPhotoFileType::RawPhoto] {
                    let file = create_gopro_photo_file(source_media_file, file_type_enum)?;
                    if let Some(v) = create_simple_file_if_exists(&file, filetype(file.extension().unwrap().to_str().unwrap())?).unwrap() {
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
