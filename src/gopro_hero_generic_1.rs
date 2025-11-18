use anyhow::{Result};
use crate::SourceMediaAdapter;
use std::path::{PathBuf};
use crate::helpers::*;
use crate::FileItem;

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

fn create_gopro_photo_filename(input_filename:String, file_type: GoProPhotoFileType ) -> Option<String> {
    let (name, _) = input_filename.rsplit_once('.')?;
    if name.len() < 1 { // minimal length, GX/L + NN + One character media id
        return None;
    }
    let new_extension = match file_type {
        GoProPhotoFileType::JpegPhoto => "JPG",
        GoProPhotoFileType::RawPhoto => "GPR",
    };
    Some(format!("{name}.{new_extension}"))
}

fn create_gopro_video_filename(input_filename:String, part:u8, file_type: GoProVideoFileType ) -> Option<String> {
    let (name, _) = input_filename.rsplit_once('.')?;

    if name.len() < 5 { // minimal length, GX/L + NN + One character media id
        return None;
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

    Some(format!("{new_prefix}{new_part}{media_id}.{new_extension}"))
}

pub struct GoProAdapter;

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

        let path = source_media_file.as_path();

        let ext = path
            .extension()
            .and_then(|e| e.to_str());

        match ext {
            Some("THM")|Some("MP4")|Some("WAV")|Some("LRV") => {
                let mut part_count:u8 = 0;
                //Get part count
                for part in 1..=99 {
                    let test_file_path=source_media_file.parent().unwrap().join(create_gopro_video_filename(path.file_name().unwrap().to_string_lossy().into_owned(),part,GoProVideoFileType::HighBitrateVideo).unwrap());
                    if test_file_path.exists(){
                        part_count+=1;
                    }else{
                        if part==1{
                            return Err(anyhow::anyhow!("Iniital video file not found"));
                        }else{
                            break;
                        }
                    }
                }

                for part in 1..=part_count {
                    //TODO: Maybe emove the check and calculations for the first one
                    let test_file_path=source_media_file.parent().unwrap().join(create_gopro_video_filename(path.file_name().unwrap().to_string_lossy().into_owned(),part,GoProVideoFileType::HighBitrateVideo).unwrap());
                    if !test_file_path.exists(){
                            return Err(anyhow::anyhow!("MP4 file that got found later could not be reopened. This should never happen"))
                    }
                    items.push(create_part_file(test_file_path.to_string_lossy().into_owned(),"video","video",part_count,part));

                    let lrv_filename=source_media_file.parent().unwrap().join(create_gopro_video_filename(path.file_name().unwrap().to_string_lossy().into_owned(),part,GoProVideoFileType::LowBitrateVideo).unwrap());
                    if !lrv_filename.exists(){
                        return Err(anyhow::anyhow!("MP4 file found but not the related LRV"))
                    }
                    items.push(create_part_file(lrv_filename.to_string_lossy().into_owned(),"video-preview","video",part_count,part));

                    let thm_filename=source_media_file.parent().unwrap().join(create_gopro_video_filename(path.file_name().unwrap().to_string_lossy().into_owned(),part,GoProVideoFileType::ThumbnailPhoto).unwrap());
                    if !thm_filename.exists(){
                        return Err(anyhow::anyhow!("MP4 file found but not the related THM"))
                    }
                    items.push(create_part_file(thm_filename.to_string_lossy().into_owned(),"photo-preview","video",part_count,part));

                    let wav_filename=source_media_file.parent().unwrap().join(create_gopro_video_filename(path.file_name().unwrap().to_string_lossy().into_owned(),part,GoProVideoFileType::WavAudio).unwrap());
                    if wav_filename.exists(){
                        items.push(create_part_file(wav_filename.to_string_lossy().into_owned(),"audio","video",part_count,part));
                    }
                }
            },
            Some("JPG")|Some("GPR") => {
                let jpeg_filename=source_media_file.parent().unwrap().join(create_gopro_photo_filename(path.file_name().unwrap().to_string_lossy().into_owned(),GoProPhotoFileType::JpegPhoto).unwrap());
                if !jpeg_filename.exists(){
                    return Err(anyhow::anyhow!("Jpeg photo not found"));
                }
                items.push(create_simple_file(jpeg_filename.to_string_lossy().into_owned(),"photo","photo"));

                let gpr_filename=source_media_file.parent().unwrap().join(create_gopro_photo_filename(path.file_name().unwrap().to_string_lossy().into_owned(),GoProPhotoFileType::RawPhoto).unwrap());
                if !gpr_filename.exists(){
                    return Err(anyhow::anyhow!("raw GPR photo not found"));
                }
                items.push(create_simple_file(gpr_filename.to_string_lossy().into_owned(),"photo-raw","photo"));
            }
            _ => {return Err(anyhow::anyhow!("Invalid input file"));}
        };
        return Ok(items);
    }

    fn name(&self) -> String {
        return "GoPro-Generic-1".to_string()
    }
}
