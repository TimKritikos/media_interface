/* gopro_hero_generic_1.rs - Handler for GoPro Hero style cameras where the files are layied out in
 * the top level directory

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
use bitflags::bitflags;
use crate::SourceMediaInterface;
use std::path::{PathBuf,Path};
use crate::helpers::*;
use crate::FileItem;
use crate::helpers::ItemType::*;
use crate::helpers::FileType::*;

////////////////////////////////////////
//       GoPro Specific helpers       //
////////////////////////////////////////

fn get_gopro_video_part_id(filename:String) -> Result<u8> {
    return match filename.as_str().get(2..4).ok_or_else(|| anyhow!("Couldn't parse gorpo video part id"))?.parse::<u8>() {
        Ok(p) => Ok(p),
        Err(e) => { return Err(anyhow!("Error parsing filename: {}", e)); }
    };
}

bitflags!{
    #[derive(PartialEq)]
    struct GoProVideoFileType: u8 {
        const LowBitrateVideo             = 1 << 0;
        const HighBitrateH265Video        = 1 << 1;
        const HighBitrateH264Video        = 1 << 2;
        const WavAudio                    = 1 << 3;
        const ThumbnailPhoto_of_H264Video = 1 << 4;
        const ThumbnailPhoto_of_H265Video = 1 << 5;
    }
}

enum GoProPhotoFileType{
    JpegPhoto,
    RawPhoto,
}

fn create_gopro_photo_file(input_file:&Path, file_type: GoProPhotoFileType ) -> Result<PathBuf> {

    let input_filename = input_file.file_name().ok_or_else(|| anyhow!("Couldn't get filename of reference photo file"))?.to_string_lossy();

    let (name, _) = input_filename.rsplit_once('.').ok_or_else(|| anyhow!("Failed to split gopro style filename from it's extension {:?}", input_filename))?;
    if name.len() < 2 {
        return Err(anyhow!("Input gopro style filename without the extension was not long enough {:?}", name));
    }
    let new_extension = match file_type {
        GoProPhotoFileType::JpegPhoto => "JPG",
        GoProPhotoFileType::RawPhoto => "GPR",
    };

    let input_dirname = input_file.parent().context("Couldn't get file's parent directory")?;

    Ok(input_dirname.join(format!("{name}.{new_extension}")))
}

fn create_gopro_video_file(input_file:&Path, part:u8, file_type: &GoProVideoFileType ) -> Result<PathBuf> {

    let input_filename = input_file.file_name().ok_or_else(|| anyhow!("Couldn't get filename of reference photo file"))?.to_string_lossy();

    let (name, _) = input_filename.rsplit_once('.').ok_or_else(|| anyhow!("Failed to split gopro style filename from it's extension {:?}", input_filename))?;

    if name.len() < 5 { // minimal length, GX/L + NN + One character media id
        return Err(anyhow!("Input gopro style filename without the extension was not long enough {:?}", name));
    }

    let media_id = &name[4..];

    let new_prefix = match *file_type {
        GoProVideoFileType::LowBitrateVideo => Ok("GL"),
        GoProVideoFileType::HighBitrateH264Video => Ok("GH"),
        GoProVideoFileType::HighBitrateH265Video => Ok("GX"),
        GoProVideoFileType::WavAudio => Ok("GX"),
        GoProVideoFileType::ThumbnailPhoto_of_H264Video => Ok("GH"),
        GoProVideoFileType::ThumbnailPhoto_of_H265Video => Ok("GX"),
        _ => Err(anyhow!("expected one and only one type")),
    }?;

    let new_part = format!("{:02}", part);

    let new_extension = match *file_type {
        GoProVideoFileType::LowBitrateVideo => Ok("LRV"),
        GoProVideoFileType::HighBitrateH264Video => Ok("MP4"),
        GoProVideoFileType::HighBitrateH265Video => Ok("MP4"),
        GoProVideoFileType::WavAudio => Ok("WAV"),
        GoProVideoFileType::ThumbnailPhoto_of_H264Video => Ok("THM"),
        GoProVideoFileType::ThumbnailPhoto_of_H265Video => Ok("THM"),
        _ => Err(anyhow!("expected one and only one type")),
    }?;

    let input_dirname = input_file.parent().context("Couldn't get file's parent directory")?;

    Ok(input_dirname.join(format!("{new_prefix}{new_part}{media_id}.{new_extension}")))
}

pub struct GoProInterface;

////////////////////////////////////////
//         File parsing code          //
////////////////////////////////////////

struct PartCount{
    existing_parts_count:u8,
    all_parts_count:u8,
}

fn count_gopro_parts( base_file:&Path, known_missing_files: &[PathBuf] ) -> Result<PartCount> {

    let mut parts:PartCount = PartCount{existing_parts_count:0, all_parts_count:0};

    for part in 1..=99 {

        let file_h265 = create_gopro_video_file(base_file, part, &GoProVideoFileType::HighBitrateH265Video)?;
        let file_h264 = create_gopro_video_file(base_file, part, &GoProVideoFileType::HighBitrateH264Video)?;

        if file_h264.exists() || file_h265.exists() {
            parts.existing_parts_count+=1;
            parts.all_parts_count+=1;
        }else if known_missing_files.contains(&file_h264) || known_missing_files.contains(&file_h265) {
            parts.all_parts_count+=1;
        }else if part == 0 {
            return Err(anyhow!("Iniital video file not found"));
        }else{
            break;
        }
    }

    Ok(parts)
}

fn filetype(ext: &str) -> Result<JsonFileInfoTypes> {
    match ext {
        "THM" => Ok(JsonFileInfoTypes{ file_type:FileImagePreview, item_type:ItemVideo }),
        "MP4" => Ok(JsonFileInfoTypes{ file_type:FileVideo,        item_type:ItemVideo }),
        "LRV" => Ok(JsonFileInfoTypes{ file_type:FileVideoPreview, item_type:ItemVideo }),
        "WAV" => Ok(JsonFileInfoTypes{ file_type:FileAudio,        item_type:ItemVideo }),

        "JPG" => Ok(JsonFileInfoTypes{ file_type:FileImage,        item_type:ItemImage }),
        "GPR" => Ok(JsonFileInfoTypes{ file_type:FileImageRaw,     item_type:ItemImage }),
        _ => Err(anyhow!("unkown file extension {:?} trying to determain file type", ext)),
    }
}

impl SourceMediaInterface for GoProInterface {
    //TODO: handle case where the thumbnail is in the known missing files and the item needs to be
    //represented by something else
    fn list_thumbnail( &self, _source_media_location: &Path, source_media_card: &Path, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card, |filename: &str, input_ext: Option<&str>, path: &PathBuf, path_str: &str| {
            let ext = input_ext.ok_or_else(|| anyhow!("Expected filter_dir to porivde a file extension"))?;
            match ext {
                "THM" => {
                    let part_id = get_gopro_video_part_id(filename.to_string())?;
                    if part_id != 1 {
                        for n in 1..part_id{
                            let n_file = create_gopro_video_file(path, n, &GoProVideoFileType::LowBitrateVideo)?; // TODO: It could be the case that we are missing the LRV but the MP4 is there in which case it's better to return a high quality equivelant of the first part of the video than either a later low quality or none at all
                            if ! known_missing_files.contains(&n_file){
                                return Ok(None);
                            }
                        }
                    }

                    let ret = create_simple_file(path_str.to_string(), filetype(ext)?, Some(path.with_extension("MP4").to_string_lossy().into_owned()))?;

                    Ok(Some(ret))
                }
                "JPG" => Ok(Some(create_simple_file(path_str.to_string(), filetype(ext)?, None)?)),
                "MP4" | "GPR" | "LRV" | "WAV" => Ok(None),
                _ => Err(anyhow!("Unexpected file {}", path_str)),
            }
        })
    }
    fn list_high_quality( &self, _source_media_location: &Path, source_media_card: &Path, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>> {
        filter_dir(source_media_card,|filename: &str, input_ext: Option<&str>, path: &PathBuf, path_str: &str|{
            let ext = input_ext.ok_or_else(|| anyhow!("Expected filter_dir to porivde a file extension"))?;
            match ext {
                "MP4" => {
                    let part_id = get_gopro_video_part_id(filename.to_string())?;
                    if part_id != 1 {
                        for n in 1..part_id{
                            let h264_file = create_gopro_video_file(path, n, &GoProVideoFileType::HighBitrateH264Video)?;
                            let h265_file = create_gopro_video_file(path, n, &GoProVideoFileType::HighBitrateH264Video)?;
                            if ! known_missing_files.contains(&h265_file)|| ! known_missing_files.contains(&h264_file){ //TODO: Same warning as in list_thumbnail about missing files
                                return Ok(None);
                            }
                        }
                    }

                    let part_count = count_gopro_parts(path, &known_missing_files)?;

                    let ret = create_part_file(path_str.to_string(), filetype(ext)?, part_count.existing_parts_count, 1, Some(path_str.to_string()));

                    Ok(Some(ret))
                }
                "GPR" | "JPG" => {
                    if ext == "GPR" || !create_gopro_photo_file(path, GoProPhotoFileType::RawPhoto)?.exists() {
                        return Ok(Some(create_simple_file(path_str.to_string(), filetype(ext)?, None)?));
                    }
                    Ok(None)
                }
                "THM" | "LRV" | "WAV" => Ok(None),
                _ => Err(anyhow!("Unexpected file {}", path_str)),
            }
        })
    }
    fn get_related(&self, _source_media_location: &Path, source_media_file: &Path, known_missing_files: Vec<PathBuf>) -> Result<Vec<FileItem>>{
        let mut items = Vec::<FileItem>::new();

        let ext = get_extension_str(source_media_file)?;

        match ext {
            "THM"|"MP4"|"WAV"|"LRV" => {

                let part_count = count_gopro_parts(source_media_file, &known_missing_files)?;

                let mut existing_part_number:u8 = 1;
                for part in 1..=part_count.all_parts_count {

                    let file_types = [
                        GoProVideoFileType::HighBitrateH264Video,
                        GoProVideoFileType::HighBitrateH265Video,
                        GoProVideoFileType::LowBitrateVideo,
                        GoProVideoFileType::ThumbnailPhoto_of_H265Video,
                        GoProVideoFileType::ThumbnailPhoto_of_H264Video,
                        GoProVideoFileType::WavAudio,
                    ];

                    let mut found_types = GoProVideoFileType::empty();

                    for file_type_enum in file_types {
                        let file = create_gopro_video_file(source_media_file, part, &file_type_enum)?;
                        let extension = get_extension_str(&file)?;

                        if let Some(item) = create_part_file_if_exists(&file, filetype(extension)?, part_count.existing_parts_count, existing_part_number, None) {
                            items.push(item);
                            found_types |= file_type_enum;
                        }else if known_missing_files.contains(&file){
                            found_types |= file_type_enum;
                        }
                    }
                    if found_types != GoProVideoFileType::empty() {
                        existing_part_number+=1;
                    }
                    if ! (found_types.contains(GoProVideoFileType::HighBitrateH264Video) ^ found_types.contains(GoProVideoFileType::HighBitrateH265Video) ){
                        return Err(anyhow!("expected either an H265 GX video or an H264 GL video. Got either both or none"));
                    }
                    if ! (found_types.contains(GoProVideoFileType::ThumbnailPhoto_of_H264Video) ^ found_types.contains(GoProVideoFileType::ThumbnailPhoto_of_H265Video)) {
                        return Err(anyhow!("expected either an H265 GX video thumbnail or an H264 GL video thumbnail. Got either both or none"));
                    }
                    if ! found_types.contains(GoProVideoFileType::LowBitrateVideo){
                        return Err(anyhow!("expected a low bitrate LRV video file"));
                    }
                }
            },
            "JPG" | "GPR" => {
                for file_type_enum in [GoProPhotoFileType::JpegPhoto, GoProPhotoFileType::RawPhoto] {
                    let file = create_gopro_photo_file(source_media_file, file_type_enum)?;
                    let extension = get_extension_str(&file)?;
                    if let Some(v) = create_simple_file_if_exists(&file, filetype(extension)?, None)? {
                        items.push(v);
                    }
                }
            }
            _ => {
                return Err(anyhow!("Invalid input file"));
            }
        };
        Ok(items)
    }

    fn name(&self) -> &'static str {
        "GoPro-Hero-Generic-1"
    }
}
