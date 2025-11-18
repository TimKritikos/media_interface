use anyhow::{Result};
use std::path::{Path};
use std::fs;
use crate::FileItem;

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

pub fn create_simple_file(file_path:String, file_type:&str, item_type:&str) -> FileItem {
    FileItem{
        file_path:file_path,
        file_type:file_type.to_string(),
        item_type:item_type.to_string(),
        part_count:None,
        part_num:None,
    }
}

pub fn create_part_file(file_path:String, file_type:&str, item_type:&str, part_count:u8, part_num:u8) -> FileItem {
    FileItem{
        file_path:file_path,
        file_type:file_type.to_string(),
        item_type:item_type.to_string(),
        part_count:Some(part_count),
        part_num:Some(part_num),
    }
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
