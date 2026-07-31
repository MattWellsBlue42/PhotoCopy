use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use walkdir::WalkDir;

mod app;

const MEDIA_EXTENSIONS: &[&str] = &["heic", "jpg", "jpeg", "png", "mov", "mp4"];

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();

    let app = app::App::new()?;

    match app.chosen {
        Some(path) => handle_repo_chosen(path)?,
        None => println!("cancelled"),
    }
    Ok(())
}

fn handle_repo_chosen(path: PathBuf) -> io::Result<()> {
    let files_info = grab_media_files(&path);
    copy_media_files(files_info);
    Ok(())
}

struct PathInfo {
    path: PathBuf,
    file_name: String,
    created_at: Timestamp
}

fn grab_media_files(root: &Path) -> Vec<PathInfo> {
    let mut paths_to_files = Vec::new();

    for entry in WalkDir::new(root) {
        // Shit to make it not angry
        let Ok(entry) = entry else { continue };

        // Make sure its a file
        if !entry.file_type().is_file() {
            continue;
        }

        // Check to make sure its a media and grab the file name
        let is_media = entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext|MEDIA_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()));
        let file_name = entry.file_name();

        if is_media && !file_name.is_empty() {
            // Grab Metadata for the created ts
            let meta_data = entry.metadata();
            let path_info = PathInfo {
                path: entry.clone().into_path(),
                file_name: file_name.to_string_lossy().trim().to_string(),
                created_at: jiff::Timestamp::try_from(meta_data.unwrap().created().unwrap()).unwrap()
            };
            paths_to_files.push(path_info);
        }
    }

    // I think this shit returns the var (kinda cool)
    paths_to_files
}

fn copy_media_files(paths_info: Vec<PathInfo>) {
    // Iterate through the items that we found from before
    for path_info in &paths_info {
        // let mut new_path = PathBuf::from("/Volumes/Photos");
        // Make the path from the destination directory and add the date to it
        let mut new_path = PathBuf::from(env::var("DEST_DIR").unwrap());
        new_path.push(path_info.created_at.in_tz("America/Phoenix").unwrap().strftime("%Y-%m-%d").to_string());

        // Make the path for it and check to make sure it exists
        let dest_dir = Path::new(&new_path);
        if dest_dir.is_dir() {
            println!("Path exists: {}", dest_dir.to_string_lossy())
        } else {
            // Dir didnt exist, create all of the dirs needed
            let create_dir = fs::create_dir_all(dest_dir);
            match create_dir {
                Ok(_result) => println!("Created dir: {}", dest_dir.to_string_lossy()),
                Err(e) => println!("{}", e)
            }
        }

        // Append the filename to the path so we have the full new path
        new_path.push(&path_info.file_name);
        let result = fs::copy(&path_info.path, new_path);

        match result {
            Ok(results) => println!("{}", results),
            Err(e) => println!("{}", e)
        }
    }
}
