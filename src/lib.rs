extern crate chrono;
extern crate handlebars;
#[macro_use]
extern crate lazy_static;
extern crate pulldown_cmark;
extern crate regex;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

use chrono::prelude::*;
use error::{Error, Result};
use pulldown_cmark::{html, Parser};
use regex::Regex;
use std::fs::{read_dir, ReadDir};
use std::iter::Peekable;
use std::mem::replace;
use std::path::{Path, PathBuf};
use util::read_file;

mod error;
pub mod render;
pub mod util;

#[derive(Debug)]
pub struct Items {
    root_path: PathBuf,
    output_path: PathBuf,
    read_dir: ReadDir,
    current_index: Option<Item>,
    current_dir: Option<Box<Peekable<Items>>>,
    current_dir_items: Vec<Item>,
    default_layout: String,
}

impl Items {
    pub fn new(root_path: PathBuf, dir_path: PathBuf, output_path: PathBuf) -> Result<Items> {
        let read_dir = read_dir(&dir_path)?;
        let mut default_layout_file_path = dir_path;
        default_layout_file_path.set_extension("yml");

        let default_layout =
            get_default_layout(&default_layout_file_path).unwrap_or("default".to_string());

        Ok(Items {
            root_path,
            output_path,
            current_dir: None,
            current_index: None,
            current_dir_items: vec![],
            read_dir,
            default_layout,
        })
    }
    fn get_next_from_current_dir(&mut self) -> Option<Result<Item>> {
        loop {
            let next = if let Some(ref mut current_dir) = self.current_dir {
                current_dir.next()
            } else {
                None
            };
            match next {
                Some(next) => {
                    if let Ok(ref next) = next {
                        let path: &Path = next.path.as_ref();
                        if let Some("index") = path.file_stem().and_then(|stem| stem.to_str()) {
                            self.current_index = Some(next.clone());
                            continue;
                        } else {
                            self.current_dir_items.push(next.clone());
                        }
                    }
                    break Some(next);
                }
                None => {
                    if self.current_index.is_none() {
                        break None;
                    }
                    let index = self.current_index.clone().map(|mut i| {
                        i.items = replace(&mut self.current_dir_items, vec![]);
                        Ok(i)
                    });
                    self.current_dir = None;
                    self.current_index = None;
                    break index;
                }
            }
        }
    }

    fn get_next_from_read_dir(&mut self) -> Option<Result<Item>> {
        loop {
            let entry = self.read_dir.next();
            if entry.is_none() {
                break None;
            }

            let entry = entry.unwrap();
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    // skip dotfiles
                    if path.file_name()
                        .map(|f| f.to_str().map(|f| f.starts_with(".")).unwrap_or(false))
                        .unwrap_or(false)
                    {
                        continue;

                    // recurse into directories
                    } else if path.is_dir() {
                        let items = Items::new(
                            self.root_path.to_owned(),
                            path,
                            self.output_path.to_owned(),
                        );

                        match items {
                            Ok(items) => {
                                let mut items = items.peekable();
                                if items.peek().is_none() {
                                    continue;
                                } else {
                                    self.current_dir = Some(Box::new(items));
                                    break self.get_next_from_current_dir();
                                }
                            }
                            Err(e) => break Some(Err(e)),
                        }

                    // skip non-markdown files
                    // @todo: copy assets
                    } else if path.extension().map(|e| e != "md").unwrap_or(true) {
                        continue;
                    } else {
                        break Some(Item::new(
                            &path,
                            &self.root_path,
                            self.default_layout.to_owned(),
                            self.output_path.to_owned(),
                        ));
                    }
                }
                Err(e) => break Some(Err(Error::Io(e))),
            }
        }
    }
}

impl Iterator for Items {
    type Item = Result<Item>;
    fn next(&mut self) -> Option<Result<Item>> {
        let next_from_current_dir = self.get_next_from_current_dir();
        if next_from_current_dir.is_some() {
            next_from_current_dir
        } else {
            self.get_next_from_read_dir()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DefaultMetaData {
    layout: String,
}

fn get_default_layout(path: &PathBuf) -> Result<String> {
    let contents = read_file(path)?;
    let DefaultMetaData { layout } = serde_yaml::from_str(&contents)?;
    Ok(layout)
}

#[derive(Debug, Clone)]
pub struct Item {
    pub title: Option<String>,
    pub body: Option<String>,
    pub date: Option<Date<Utc>>,
    pub description: Option<String>,
    pub path: PathBuf,
    pub output_path: PathBuf,
    pub layout: String,
    pub input_file_path: PathBuf,
    pub items: Vec<Item>,
}

impl Item {
    pub fn new(
        input_file_path: &PathBuf,
        root_path: &PathBuf,
        default_layout: String,
        mut output_path: PathBuf,
    ) -> Result<Item> {
        let contents = read_file(&input_file_path)?;
        let mut contents = contents.split("---\n").skip(1);
        let front_matter = contents.next().ok_or(format!(
            "Error parsing yaml front matter: {:?}",
            &input_file_path
        ))?;

        let ItemMetaData {
            title,
            description,
            layout,
        } = serde_yaml::from_str(&front_matter)?;

        let ParsedPath { date, mut path } = parse_path(&input_file_path, &root_path)?;
        output_path.push(&path);

        let parser = Parser::new(contents
            .next()
            .ok_or("Error extracting markdown from file")?);
        let mut body = String::new();
        html::push_html(&mut body, parser);

        path.set_extension("");

        Ok(Item {
            title: Some(title),
            date,
            description,
            layout: layout.unwrap_or(default_layout),
            body: Some(body),
            path,
            output_path,
            input_file_path: input_file_path.to_owned(),
            items: vec![],
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ItemMetaData {
    title: String,
    description: Option<String>,
    layout: Option<String>,
}

struct ParsedPath {
    date: Option<Date<Utc>>,
    path: PathBuf,
}

fn parse_path(path: &PathBuf, root_path: &PathBuf) -> Result<ParsedPath> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(\d{4})-(\d{2})-(\d{2})-(.+)")
            .expect("Error creating regex");
    }
    let path = path.to_owned();
    let stem = path.file_stem()
        .ok_or("Error extracting file stem")?
        .to_str()
        .ok_or("Error casting file stem")?;
    let ParsedPath { path, date } = if let Some(captures) = RE.captures(stem) {
        let year = captures
            .get(1)
            .ok_or("Error parsing year from file name")?
            .as_str()
            .parse::<i32>()?;
        let month = captures
            .get(2)
            .ok_or("Error parsing month from file name")?
            .as_str()
            .parse::<u32>()?;
        let day = captures
            .get(3)
            .ok_or("Error parsing day from file name")?
            .as_str()
            .parse::<u32>()?;
        let slug = captures
            .get(4)
            .ok_or("Error parsing slug from file name")?
            .as_str();
        let mut path = path.parent()
            .ok_or("Error getting file's parent dir")?
            .to_path_buf();
        path.push(format!("{:04}", year));
        path.push(format!("{:02}", month));
        path.push(format!("{:02}", day));
        path.push(slug);
        ParsedPath {
            date: Some(Utc.ymd(year, month, day)),
            path,
        }
    } else {
        ParsedPath {
            date: None,
            path: path.to_owned(),
        }
    };

    let mut path = path.strip_prefix(root_path
        .to_str()
        .ok_or("Error casting current dir to str")?)?
        .to_path_buf();
    path.set_extension("html");

    Ok(ParsedPath { path, date })
}
