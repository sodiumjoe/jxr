use handlebars;
use serde_yaml;
use std::error;
use std::fmt;
use std::io;
use std::num;
use std::path;
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Serde(serde_yaml::Error),
    Item(ItemError),
    Handlebars(handlebars::RenderError),
    ParseInt(num::ParseIntError),
    StripPrefix(path::StripPrefixError),
    HandlebarsTemplate(handlebars::TemplateError),
}

#[derive(Debug)]
pub struct ItemError {
    string: String,
}

impl error::Error for ItemError {
    fn description(&self) -> &str {
        &self.string
    }
}

impl fmt::Display for ItemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {}", &self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::Serde(ref err) => write!(f, "Serde error: {}", err),
            Error::Item(ref err) => write!(f, "Error: {}", err),
            Error::Handlebars(ref err) => write!(f, "Handlebars error: {}", err),
            Error::ParseInt(ref err) => write!(f, "ParseInt error: {}", err),
            Error::StripPrefix(ref err) => write!(f, "StripPrefix error: {}", err),
            Error::HandlebarsTemplate(ref err) => write!(f, "HandlebarsTemplate error: {}", err),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Serde(ref err) => Some(err),
            Error::Item(ref err) => Some(err),
            Error::Handlebars(ref err) => Some(err),
            Error::ParseInt(ref err) => Some(err),
            Error::StripPrefix(ref err) => Some(err),
            Error::HandlebarsTemplate(ref err) => Some(err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Error {
        Error::Serde(err)
    }
}

impl From<handlebars::RenderError> for Error {
    fn from(err: handlebars::RenderError) -> Error {
        Error::Handlebars(err)
    }
}

impl From<handlebars::TemplateError> for Error {
    fn from(err: handlebars::TemplateError) -> Error {
        Error::HandlebarsTemplate(err)
    }
}

impl From<num::ParseIntError> for Error {
    fn from(err: num::ParseIntError) -> Error {
        Error::ParseInt(err)
    }
}

impl From<path::StripPrefixError> for Error {
    fn from(err: path::StripPrefixError) -> Error {
        Error::StripPrefix(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Item(ItemError { string: err })
    }
}

impl<'a> From<&'a str> for Error {
    fn from(err: &str) -> Error {
        Error::Item(ItemError {
            string: String::from(err),
        })
    }
}
