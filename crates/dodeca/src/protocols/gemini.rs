use super::ProtocolHandler;
use crate::db::OutputFile;
use crate::types::Route;
use html_parser::{Dom, Node};

pub struct GeminiHandler;

impl ProtocolHandler for GeminiHandler {
    fn protocol_name(&self) -> &'static str {
        "gemini"
    }

    fn generate(
        &self,
        route: &Route,
        dom: &Dom,
        _original_html: &str,
        _head_injections: Vec<String>,
        _hrefs: Vec<String>,
        _element_ids: Vec<String>,
    ) -> Option<OutputFile> {
        let mut gemtext = String::new();

        fn parse_node(node: &Node, gemtext: &mut String, queued_links: &mut Vec<(String, String)>) {
            match node {
                Node::Text(text) => {
                    let cleaned = text.replace('\n', " ");
                    let cleaned = cleaned.trim();
                    if !cleaned.is_empty() {
                        gemtext.push_str(cleaned);
                        gemtext.push(' ');
                    }
                }
                Node::Element(element) => match element.name.as_str() {
                    "h1" => {
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        gemtext.push_str("# ");
                        for child in &element.children {
                            parse_node(child, gemtext, queued_links);
                        }
                        gemtext.push('\n');
                    }
                    "h2" => {
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        gemtext.push_str("## ");
                        for child in &element.children {
                            parse_node(child, gemtext, queued_links);
                        }
                        gemtext.push('\n');
                    }
                    "h3" | "h4" | "h5" | "h6" => {
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        gemtext.push_str("### ");
                        for child in &element.children {
                            parse_node(child, gemtext, queued_links);
                        }
                        gemtext.push('\n');
                    }
                    "a" => {
                        let href = element
                            .attributes
                            .get("href")
                            .and_then(|h: &Option<String>| h.clone())
                            .unwrap_or_default();
                        let mut text = String::new();
                        for child in &element.children {
                            if let Node::Text(t) = child {
                                text.push_str(t);
                            }
                        }
                        if !href.is_empty() {
                            queued_links.push((href, text.trim().to_string()));
                            gemtext.push_str(text.trim());
                            gemtext.push(' ');
                        }
                    }
                    "img" => {
                        let src = element
                            .attributes
                            .get("src")
                            .and_then(|h: &Option<String>| h.clone())
                            .unwrap_or_default();
                        let alt = element
                            .attributes
                            .get("alt")
                            .and_then(|h: &Option<String>| h.clone())
                            .unwrap_or_default();
                        if !src.is_empty() {
                            queued_links.push((src, format!("Image: {}", alt)));
                        }
                    }
                    "pre" => {
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        gemtext.push_str("```\n");
                        for child in &element.children {
                            if let Node::Element(code_el) = child {
                                if code_el.name == "code" {
                                    for code_child in &code_el.children {
                                        if let Node::Text(t) = code_child {
                                            gemtext.push_str(t);
                                        }
                                    }
                                }
                            } else if let Node::Text(t) = child {
                                gemtext.push_str(t);
                            }
                        }
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        gemtext.push_str("```\n");
                    }
                    "p" | "div" | "br" => {
                        for child in &element.children {
                            parse_node(child, gemtext, queued_links);
                        }
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        if !queued_links.is_empty() {
                            for (href, text) in queued_links.drain(..) {
                                gemtext.push_str(&format!("=> {} {}\n", href, text));
                            }
                        }
                        gemtext.push('\n');
                    }
                    "li" => {
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        gemtext.push_str("* ");
                        for child in &element.children {
                            parse_node(child, gemtext, queued_links);
                        }
                        gemtext.push('\n');
                    }
                    "blockquote" => {
                        if !gemtext.ends_with('\n') {
                            gemtext.push('\n');
                        }
                        gemtext.push_str("> ");
                        for child in &element.children {
                            parse_node(child, gemtext, queued_links);
                        }
                        gemtext.push('\n');
                    }
                    _ => {
                        for child in &element.children {
                            parse_node(child, gemtext, queued_links);
                        }
                    }
                },
                Node::Comment(_) => {}
            }
        }

        let mut queued_links = Vec::new();
        for child in &dom.children {
            parse_node(child, &mut gemtext, &mut queued_links);
        }
        if !queued_links.is_empty() {
            if !gemtext.ends_with('\n') {
                gemtext.push('\n');
            }
            for (href, text) in queued_links.drain(..) {
                gemtext.push_str(&format!("=> {} {}\n", href, text));
            }
        }

        Some(OutputFile::Gemini {
            route: route.clone(),
            content: gemtext,
        })
    }
}
