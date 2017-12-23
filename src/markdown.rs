use std::path::PathBuf;
use walkdir::{IntoIter, WalkDir, Error};

pub struct MarkdownPaths {
    walkdir: IntoIter,
}

impl MarkdownPaths {
    pub fn new(path: &PathBuf) -> MarkdownPaths {
        MarkdownPaths { walkdir: WalkDir::new(path).into_iter() }
    }
}

impl Iterator for MarkdownPaths {
    type Item = Result<PathBuf, Error>;

    fn next(&mut self) -> Option<Result<PathBuf, Error>> {
        loop {
            if let Some(entry) = self.walkdir.next() {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if (ext) == "md" {
                                break Some(Ok(path.to_path_buf()));
                            }
                        }
                    }
                    Err(err) => break Some(Err(err)),
                }
            } else {
                break None;
            }
        }
    }
}
