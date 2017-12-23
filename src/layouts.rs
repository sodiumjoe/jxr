use handlebars::Handlebars;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use walkdir::WalkDir;
use Error;

pub struct Layouts {
    pub layouts: HashMap<String, Handlebars>,
    pub listings: HashMap<String, Handlebars>,
}

impl Layouts {
    pub fn new(path: &PathBuf) -> Result<Layouts, Error> {
        let mut layouts = HashMap::new();
        let mut listings = HashMap::new();
        for entry in WalkDir::new(path).into_iter() {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                if file_name == "listing.hbs" {
                    let mut contents = String::new();
                    File::open(path.to_owned())?
                        .read_to_string(&mut contents)?;
                    let key = path.parent()
                        .ok_or("Error getting listings file parent")?
                        .to_str()
                        .ok_or("Error getting listings file parent")?
                        .to_string();
                    let mut handlebars = Handlebars::new();
                    handlebars.register_template_string(&key, contents)?;
                    listings.insert(key, handlebars);
                }
            }
            if let Some(ext) = path.extension() {
                if ext == "hbs" {
                    let mut contents = String::new();
                    File::open(path.to_owned())?
                        .read_to_string(&mut contents)?;
                    let key = path.file_stem()
                        .ok_or("Error getting layout file")?
                        .to_str()
                        .ok_or("Error getting layout file")?
                        .to_string();
                    let mut handlebars = Handlebars::new();
                    handlebars.register_template_string(&key, contents)?;
                    layouts.insert(key, handlebars);
                }
            }
        }
        Ok(Layouts { layouts, listings })
    }
}
