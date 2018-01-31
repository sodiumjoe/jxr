use error::{Error, Result};
use handlebars::Handlebars;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::PathBuf;
use Item;
use util::read_file;

#[derive(Debug, Serialize, Deserialize)]
struct RenderContext {
    title: Option<String>,
    body: Option<String>,
    date: Option<String>,
    description: Option<String>,
    path: PathBuf,
    items: Vec<RenderContext>,
}

impl RenderContext {
    pub fn new(file_context: &Item) -> RenderContext {
        let &Item {
            ref title,
            ref date,
            ref description,
            layout: ref _layout,
            ref body,
            ref path,
            input_file_path: ref _input_file_path,
            ref items,
            output_path: ref _output_path,
        } = file_context;

        let date = date.map(|date| date.format("%Y-%m-%d").to_string());
        let mut items: Vec<_> = items.iter().map(|f| RenderContext::new(f)).collect();
        items.sort_unstable_by(|a, b| match b.date {
            Some(ref b) => match a.date {
                Some(ref a) => b.cmp(a),
                _ => Ordering::Greater,
            },
            _ => Ordering::Less,
        });

        RenderContext {
            title: title.to_owned(),
            date,
            description: description.to_owned(),
            body: body.to_owned(),
            items,
            path: path.to_owned(),
        }
    }
}

pub struct Renderer {
    pub layouts: HashMap<String, Handlebars>,
}

impl Renderer {
    pub fn new(path: &PathBuf) -> Result<Renderer> {
        let mut layouts: HashMap<String, Handlebars> = HashMap::new();
        let layout_file_paths = read_dir(&path)?.filter_map(|entry| -> Option<PathBuf> {
            let entry = entry.ok();
            let path = entry.map(|e| e.path());
            if let Some("hbs") = path.as_ref()
                .and_then(|path| path.extension())
                .and_then(|ext| ext.to_str())
            {
                path
            } else {
                None
            }
        });
        for path in layout_file_paths {
            let contents = read_file(&path)?;
            let mut handlebars = Handlebars::new();
            if let Some(key) = path.file_stem().and_then(|f| f.to_str()) {
                handlebars.register_template_string(key, contents)?;
                layouts.insert(key.to_string(), handlebars);
            }
        }
        Ok(Renderer { layouts })
    }

    pub fn render(&self, file_context: &Item) -> Result<String> {
        let handlebars = self.layouts
            .get(&file_context.layout.to_owned())
            .ok_or(Error::from(format!(
                "missing layout: {:?} for path {:?}",
                file_context.layout, file_context.input_file_path
            )))?;
        let mut context = RenderContext::new(file_context);
        let body = Some(handlebars
            .render(&file_context.layout, &context)
            .map_err(|e| Error::from(e))?);
        context.body = body;
        self.layouts
            .get("layout")
            .ok_or("missing root layout")?
            .render("layout", &context)
            .map_err(|e| Error::from(e))
    }
}
