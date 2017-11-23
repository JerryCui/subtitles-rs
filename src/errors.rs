//! Error-handling for this library.

// `error-chain` makes it hard to document all the definitions it
// generates.
#![allow(missing_docs, unused_doc_comment)]

use csv;
use handlebars;
use serde_json;
use std::io;
use std::str;
use uchardet;

use grammar;

error_chain! {
    links {
        Uchardet(uchardet::Error, uchardet::ErrorKind);
    }

    foreign_links {
        Csv(csv::Error);
        Io(io::Error);
        Parse(grammar::ParseError);
        Render(handlebars::RenderError);
        SerdeJson(serde_json::Error);
        Template(handlebars::TemplateError);
        Utf8(str::Utf8Error);
    }
}

/// Create a new error from something that can be turned into a string.
pub fn err_str<T: Into<String>>(message: T) -> Error {
    From::from(message.into())
}
