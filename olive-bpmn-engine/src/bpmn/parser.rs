use crate::bpmn::schema::Definitions;
use strong_xml::XmlRead;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("xml parsing error: {error:?}")]
    ParsingError { error: strong_xml::XmlError },
    #[error("xml normalization error: {error:?}")]
    NormalizationError {
        #[from]
        error: NormalizationError,
    },
}

/// Parse BPMN XML document.
pub fn parse(string: &str) -> Result<Definitions, ParseError> {
    let normalized = normalize(string)?;
    Definitions::from_str(&normalized).map_err(|err| ParseError::ParsingError { error: err })
}

use sxd_document as sxd;

/// Normalization error
#[derive(Error, Debug)]
pub enum NormalizationError {
    #[error("xml parsing error: {error:?}")]
    ParsingError { error: sxd::parser::Error },
    #[error("xml writing error: {error:?}")]
    WritingError { error: std::io::Error },
}

const BPMN_NS: &str = "http://www.omg.org/spec/BPMN/20100524/MODEL";

// This function uses a different XML package (sxd-document) for processing XML
// documents. Hopefully there's no need for this package elsewhere.
// It mostly comes to the fact that xmlparser/strong-xml don't support namespaces so the prefix is
// hard-coded in element definitions. So in [`normalize`] we have to ensure the document looks
// exactly the way we need it to be.
//
// It will do the following:
//
// * Resolve BPMN's namespace (http://www.omg.org/spec/BPMN/20100524/MODEL) and
//   ensure that `bpmn` is used as a declared prefix for it.
fn normalize(string: &str) -> Result<String, NormalizationError> {
    let package = sxd::parser::parse(string)
        .map_err(|err| NormalizationError::ParsingError { error: err })?;
    let doc = package.as_document();
    let root = doc.root();
    let children = root.children();
    let top = children.iter().find_map(|x| match x {
        sxd::dom::ChildOfRoot::Element(e) => Some(e),
        _ => None,
    });
    match top {
        None => Ok(string.into()),
        Some(e) => {
            if e.name().local_part() == "definitions" {
                let ns = e
                    .preferred_prefix()
                    .and_then(|p| e.namespace_uri_for_prefix(p));
                match ns {
                    None => {}
                    Some(BPMN_NS) => {
                        update_prefix(e);
                        let mut output = Vec::new();
                        sxd::writer::format_document(&doc, &mut output)
                            .map_err(|err| NormalizationError::WritingError { error: err })?;
                        return Ok(String::from_utf8_lossy(&output).into_owned());
                    }
                    Some(_) => {}
                }
            }
            Ok(string.into())
        }
    }
}

fn update_prefix(element: &sxd::dom::Element) {
    element.set_preferred_prefix(Some("bpmn"));
    let children = element.children();
    let element_sub = children.iter().filter_map(|x| match x {
        sxd::dom::ChildOfElement::Element(e) => Some(e),
        _ => None,
    });

    element_sub.for_each(update_prefix);
}
