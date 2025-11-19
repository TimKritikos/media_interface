use anyhow::{Result};
use crate::SourceMediaAdapter;
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

pub struct GoProAdapter;

////////////////////////////////////////
//         File parsing code          //
////////////////////////////////////////

impl SourceMediaAdapter for GoProAdapter {
    fn list_thumbnail( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, ) -> Result<Vec<FileItem>> {
        filter_top_level_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &str|{
            match ext {
                Some("THM") => {
                    if get_gopro_video_part_id(filename.to_string())? == 1 {
                        Ok(Some(create_simple_file(path.to_string(), "image", "video")))
                    } else {
                        Err(anyhow::anyhow!("Unable to parse video id file {}",path))
                    }
                }
                Some("JPG") => Ok(Some(create_simple_file(path.to_string(), "image", "image"))),
                Some("MP4") | Some("GPR") | Some("LRV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path)),
            }
        })
    }
    fn list_high_quality( &self, _source_media_location: &PathBuf, source_media_card: &PathBuf, ) -> Result<Vec<FileItem>> {
        filter_top_level_dir(source_media_card.as_path(),|filename: &str, ext: Option<&str>, path: &str|{
            match ext {
                Some("MP4") => {
                    if get_gopro_video_part_id(filename.to_string())? == 1 {
                        Ok(Some(create_simple_file(path.to_string(), "image", "video")))
                    } else {
                        Err(anyhow::anyhow!("Unable to parse video id file {}",path))
                    }
                }
                Some("JPG") => Ok(Some(create_simple_file(path.to_string(), "image", "image"))),
                Some("THM") | Some("GPR") | Some("LRV") => Ok(None),
                Some(_) | None => Err(anyhow::anyhow!("Unexpected file {}", path)),
            }
        })
    }
    fn get_related(&self, _source_media_location: &PathBuf, source_media_file: &PathBuf) -> Result<Vec<FileItem>>{
        let mut items = Vec::<FileItem>::new();

        let ext = source_media_file
            .as_path()
            .extension()
            .and_then(|e| e.to_str());

        match ext {
            Some("THM")|Some("MP4")|Some("WAV")|Some("LRV") => {

                let mut part_count:u8 = 0;

                for part in 1..=99 {
                    if create_gopro_video_file(source_media_file,part,GoProVideoFileType::HighBitrateVideo)?.exists() {
                        part_count+=1;
                    }else if part_count==0 {
                        return Err(anyhow::anyhow!("Iniital video file not found"));
                    }else{
                        break;
                    }
                }

                for part in 1..=part_count {
                    //TODO: Maybe emove the check and calculations for the first one
                    let mp4_file_path=create_gopro_video_file(source_media_file,part,GoProVideoFileType::HighBitrateVideo)?;
                    items.push(create_part_file_that_exists(mp4_file_path,"video","video",part_count,part)?);

                    let lrv_filename=create_gopro_video_file(source_media_file,part,GoProVideoFileType::LowBitrateVideo)?;
                    items.push(create_part_file_that_exists(lrv_filename,"video-preview","video",part_count,part)?);

                    let thm_filename=create_gopro_video_file(source_media_file,part,GoProVideoFileType::ThumbnailPhoto)?;
                    items.push(create_part_file_that_exists(thm_filename,"photo-preview","video",part_count,part)?);

                    let wav_filename=create_gopro_video_file(source_media_file,part,GoProVideoFileType::WavAudio)?;
                    if let Some(v) = create_part_file_if_exists(wav_filename,"audio","video",part_count,part){
                        items.push(v);
                    }
                }
            },
            Some("JPG")|Some("GPR") => {
                let jpeg_file=create_gopro_photo_file(source_media_file,GoProPhotoFileType::JpegPhoto)?;
                if let Some(v) = create_simple_file_if_exists(jpeg_file,"photo","photo"){
                    items.push(v);
                }

                let gpr_file=create_gopro_photo_file(source_media_file,GoProPhotoFileType::RawPhoto)?;
                if let Some(v) = create_simple_file_if_exists(gpr_file,"photo-raw","photo") {
                    items.push(v);
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid input file"));
            }
        };
        return Ok(items);
    }

    fn name(&self) -> String {
        return "GoPro-Generic-1".to_string()
    }
}
