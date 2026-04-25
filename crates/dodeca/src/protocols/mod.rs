use crate::db::OutputFile;
use crate::types::Route;
use html_parser::{Dom, Element, Node};

pub mod html;
pub mod gemini;
pub mod gopher;

pub use html::HtmlHandler;
pub use gemini::GeminiHandler;
pub use gopher::GopherHandler;

pub trait ProtocolHandler: Send + Sync {
    fn protocol_name(&self) -> &'static str;
    fn generate(
        &self,
        route: &Route,
        dom: &Dom,
        original_html: &str,
        head_injections: Vec<String>,
        hrefs: Vec<String>,
        element_ids: Vec<String>,
    ) -> Option<OutputFile>;
}

pub fn should_render_for_protocol(element: &Element, current_protocol: &str) -> bool {
    if let Some(Some(protocols)) = element.attributes.get("data-protocol") {
        let allowed: Vec<&str> = protocols
            .split(|c: char| c == ',' || c == ' ')
            .filter(|s: &&str| !s.is_empty())
            .collect();
        if !allowed.contains(&current_protocol) {
            return false;
        }
    }

    if let Some(Some(excludes)) = element.attributes.get("data-protocol-exclude") {
        let excluded: Vec<&str> = excludes
            .split(|c: char| c == ',' || c == ' ')
            .filter(|s: &&str| !s.is_empty())
            .collect();
        if excluded.contains(&current_protocol) {
            return false;
        }
    }

    true
}

pub fn filter_nodes(nodes: &mut Vec<Node>, protocol: &str) {
    let mut i = 0;
    while i < nodes.len() {
        let mut remove = false;
        let mut unwrap = false;
        if let Node::Element(el) = &nodes[i] {
            if !should_render_for_protocol(el, protocol) {
                remove = true;
            } else if el.name == "wrapper" {
                unwrap = true;
            }
        }
        
        if remove {
            nodes.remove(i);
        } else if unwrap {
            if let Node::Element(mut el) = nodes.remove(i) {
                filter_nodes(&mut el.children, protocol);
                for child in el.children.into_iter().rev() {
                    nodes.insert(i, child);
                }
            }
        } else {
            if let Node::Element(el) = &mut nodes[i] {
                filter_nodes(&mut el.children, protocol);
            }
            i += 1;
        }
    }
}

pub fn filter_dom(mut dom: Dom, protocol: &str) -> Dom {
    filter_nodes(&mut dom.children, protocol);
    dom
}

pub fn serialize_node(node: &Node, out: &mut String) {
    match node {
        Node::Text(text) => out.push_str(text),
        Node::Element(el) => {
            out.push('<');
            out.push_str(&el.name);
            for (k, v) in &el.attributes {
                if let Some(val) = v {
                    out.push_str(&format!(" {}=\"{}\"", k, val));
                } else {
                    out.push_str(&format!(" {}", k));
                }
            }
            if el.children.is_empty() {
                if ["br", "img", "hr", "meta", "link", "input", "source"].contains(&el.name.as_str()) {
                    out.push_str(" />");
                } else {
                    out.push_str(&format!("></{}>", el.name));
                }
            } else {
                out.push('>');
                for child in &el.children {
                    serialize_node(child, out);
                }
                out.push_str(&format!("</{}>", el.name));
            }
        }
        Node::Comment(c) => {
            out.push_str("<!--");
            out.push_str(c);
            out.push_str("-->");
        }
    }
}

pub fn serialize_dom(dom: &Dom, original_html: &str) -> String {
    let mut out = String::new();
    if original_html.trim_start().to_lowercase().starts_with("<!doctype html>") {
        out.push_str("<!DOCTYPE html>\n");
    }
    for child in &dom.children {
        serialize_node(child, &mut out);
    }
    out
}
