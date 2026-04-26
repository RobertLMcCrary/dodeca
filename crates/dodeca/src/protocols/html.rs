use super::{ProtocolHandler, serialize_dom};
use crate::db::OutputFile;
use crate::types::Route;
use html_parser::Dom;

pub struct HtmlHandler;

impl ProtocolHandler for HtmlHandler {
    fn protocol_name(&self) -> &'static str {
        "http"
    }

    fn generate(
        &self,
        route: &Route,
        dom: &Dom,
        original_html: &str,
        head_injections: Vec<String>,
        hrefs: Vec<String>,
        element_ids: Vec<String>,
    ) -> Option<OutputFile> {
        let content =
            if original_html.contains("data-protocol") || original_html.contains("<wrapper>") {
                serialize_dom(dom, original_html)
            } else {
                original_html.to_string()
            };
        Some(OutputFile::Html {
            route: route.clone(),
            content,
            head_injections,
            hrefs,
            element_ids,
        })
    }
}
