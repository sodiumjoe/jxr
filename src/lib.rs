extern crate chrono;
extern crate handlebars;
#[macro_use]
extern crate lazy_static;
extern crate pulldown_cmark;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;
extern crate walkdir;

use chrono::prelude::*;
use error::Error;
use handlebars::Handlebars;
use layouts::Layouts;
use markdown::MarkdownPaths;
use pulldown_cmark::{html, Parser};
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;
use walkdir::WalkDir;

mod markdown;
mod error;
pub mod layouts;

#[derive(Debug, Serialize)]
pub struct Context {
    title: Option<String>,
    body: Option<String>,
    date: Option<String>,
    path: String,
}

pub enum Item {
    Document(DocumentData),
    Listing(ListingData),
}

impl Item {
    pub fn write(&self, path: &PathBuf) -> Result<(), Error> {
        match *self {
            Item::Document(ref d) => d.write(path),
            Item::Listing(ref l) => l.write(path),
        }
    }
    pub fn get_path(&self) -> &PathBuf {
        match *self {
            Item::Document(ref d) => &d.path,
            Item::Listing(ref l) => &l.path,
        }
    }
    pub fn get_file_path(&self) -> &PathBuf {
        match *self {
            Item::Document(ref d) => &d.file_path,
            Item::Listing(ref l) => &l.file_path,
        }
    }
}

#[derive(Debug)]
pub struct DocumentData {
    pub file_path: PathBuf,
    pub path: PathBuf,
    pub date: Option<String>,
    pub contents: String,
    pub title: String,
}

impl DocumentData {
    pub fn new(
        file_path: PathBuf,
        root_path: &PathBuf,
        layouts: &HashMap<String, Handlebars>,
        defaults: &HashMap<String, String>,
    ) -> Result<DocumentData, Error> {
        let mut contents = String::new();
        File::open(file_path.to_owned())?.read_to_string(&mut contents)?;
        let mut contents = contents.split("---\n").skip(1);

        let front_matter = contents.next().ok_or("Error parsing yaml front matter")?;
        let ItemMetaData { title, layout } = serde_yaml::from_str(&front_matter)?;

        let parser = Parser::new(contents
            .next()
            .ok_or("Error extracting markdown from file")?);
        let mut body = String::new();
        html::push_html(&mut body, parser);

        let default_key = file_path
            .parent()
            .ok_or("Error searching for defaults.yml")?
            .to_str()
            .ok_or("Error searching for defaults.yml")?
            .to_string();
        let default = defaults.get(&default_key);
        let default = default.unwrap_or(&"default".to_string()).to_string();
        let layout = layout.unwrap_or(default);
        let ParsedPath { date, path } = parse_path(&file_path, &root_path)?;
        let date =
            date.map(|date| (format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())));
        let mut context = Context {
            title: Some(title.to_owned()),
            body: Some(body),
            date: date.to_owned(),
            path: path.to_owned().to_str().ok_or("path")?.to_string(),
        };
        context.body = Some(layouts
            .get(&layout)
            .ok_or("Error getting layout defined in file")?
            .render(layout.as_str(), &context)?);
        let contents = layouts
            .get("layout")
            .ok_or("Error getting root layout")?
            .render("layout", &context)?;

        Ok(DocumentData {
            file_path,
            path,
            date,
            contents,
            title,
        })
    }

    pub fn write(&self, output_path: &PathBuf) -> Result<(), Error> {
        let file_path = output_path.join(&self.path);
        create_dir(&file_path
            .parent()
            .ok_or("problem with file parent dir")?
            .to_path_buf())?;
        let mut file = File::create(file_path)?;
        file.write_all(self.contents.as_bytes())
            .expect("Error writing file");
        Ok(())
    }
}

pub struct ListingData {
    pub file_path: PathBuf,
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Serialize)]
pub struct ListingContext {
    items: Vec<Context>,
}

impl ListingData {
    pub fn new(
        file_path: String,
        root_path: &PathBuf,
        layout: Handlebars,
        root_layout: &Handlebars,
        items: &Vec<Item>,
    ) -> Result<ListingData, Error> {
        // let mut listing_items = Vec::new();
        let listing_items: Result<Vec<Context>, Error> = items
            .into_iter()
            .filter(|&item| item.get_file_path().starts_with(&file_path))
            .filter_map(|ref item| match *item {
                &Item::Document(ref item) => Some(item),
                _ => None,
            })
            .map(|item| {
                Ok(Context {
                    title: Some(item.title.to_owned()),
                    path: item.path.to_str().ok_or("path")?.to_string(),
                    body: None,
                    date: item.date.to_owned(),
                })
            })
            .collect();
        let mut listing_items = listing_items?;
        listing_items.sort_unstable_by(|ref a, ref b| match &b.date {
            &Some(ref b) => match &a.date {
                &Some(ref a) => b.cmp(a),
                _ => Ordering::Greater,
            },
            _ => Ordering::Less,
        });
        let context = ListingContext {
            items: listing_items,
        };
        let contents = layout.render(file_path.as_str(), &context)?;
        let file_path = PathBuf::from(file_path);
        let path = file_path.join("index");
        convert_path(&root_path, &path).map(|path| {
            let context = Context {
                title: None,
                body: Some(contents),
                date: None,
                path: path.to_owned().to_str().ok_or("path")?.to_string(),
            };
            let contents = root_layout.render("layout", &context)?;
            Ok(ListingData {
                file_path,
                path,
                contents,
            })
        })?
    }

    pub fn write(&self, output_path: &PathBuf) -> Result<(), Error> {
        let file_path = output_path.join(&self.path);
        create_dir(&file_path
            .parent()
            .ok_or("problem with file parent dir")?
            .to_path_buf())?;
        let mut file = File::create(file_path)?;
        file.write_all(self.contents.as_bytes())
            .expect("Error writing file");
        Ok(())
    }
}

pub struct Items {
    pub items: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Defaults {
    layout: String,
}

impl Items {
    pub fn new(root_path: &PathBuf) -> Result<Items, Error> {
        let mut defaults = HashMap::new();
        for entry in WalkDir::new(&root_path) {
            let entry = entry?;
            let file_path = entry.path();
            if let Some(file_name) = file_path.file_name() {
                if file_name == "defaults.yml" {
                    let mut contents = String::new();
                    File::open(file_path.to_owned())?.read_to_string(&mut contents)?;
                    let key = file_path
                        .parent()
                        .ok_or("Error getting parent")?
                        .to_str()
                        .ok_or("Error getting parent")?
                        .to_string();
                    let Defaults { layout } = serde_yaml::from_str(&contents)?;
                    defaults.insert(key, layout);
                }
            }
        }
        let paths = MarkdownPaths::new(&root_path);
        let Layouts { listings, layouts } = Layouts::new(&root_path)?;
        let mut items: Vec<Item> = Vec::new();
        for path in paths {
            let path = path?;
            let item = DocumentData::new(path, &root_path, &layouts, &defaults)
                .map(|d| Item::Document(d))?;
            items.push(item);
        }
        for (path, layout) in listings {
            let item = Item::Listing(ListingData::new(
                path,
                &root_path,
                layout,
                layouts.get("layout").ok_or("Error getting root layout")?,
                &items,
            )?);
            items.push(item);
        }
        Ok(Items { items })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ItemMetaData {
    title: String,
    layout: Option<String>,
}

struct ParsedPath {
    date: Option<Date<Utc>>,
    path: PathBuf,
}

fn convert_path(root_path: &PathBuf, path: &PathBuf) -> Result<PathBuf, Error> {
    let mut path = path.strip_prefix(root_path
        .to_str()
        .ok_or("Error casting current dir to str")?)?
        .to_path_buf();
    path.set_extension("html");
    Ok(path)
}

fn create_dir(path: &PathBuf) -> Result<PathBuf, Error> {
    let mut current = PathBuf::new();
    for path in path.components() {
        current.push(path.as_os_str());
        if !current.exists() {
            fs::create_dir(&current)?;
        }
    }
    Ok(current)
}

fn parse_path(path: &PathBuf, root_path: &PathBuf) -> Result<ParsedPath, Error> {
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
        path.push(year.to_string());
        path.push(month.to_string());
        path.push(day.to_string());
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

    convert_path(&root_path, &path).map(|path| ParsedPath { path, date })
}
