use error::Result;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;

fn create_dir(path: PathBuf) -> Result<PathBuf> {
    let mut current = PathBuf::new();
    for path in path.components() {
        current.push(path.as_os_str());
        if !current.exists() {
            fs::create_dir(&current)?;
        }
    }
    Ok(current)
}

pub fn write(path: &PathBuf, contents: String) -> Result<()> {
    create_dir(
        path.parent()
            .ok_or("problem with file parent dir")?
            .to_path_buf(),
    )?;
    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())
        .expect("Error writing file");
    Ok(())
}

pub fn read_file(path: &PathBuf) -> Result<String> {
    let mut contents = String::new();
    File::open(path)?.read_to_string(&mut contents)?;
    Ok(contents)
}
