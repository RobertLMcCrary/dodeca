use super::ProtocolHandler;
use crate::db::OutputFile;
use crate::types::Route;
use html_parser::{Dom, Node};

pub struct GopherHandler {
    pub header: Option<String>,
}

impl ProtocolHandler for GopherHandler {
    fn protocol_name(&self) -> &'static str {
        "gopher"
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
        let mut gopher = String::new();
        if let Some(header) = &self.header {
            for line in header.lines() {
                gopher.push_str(&format!("i{}\tfake\tnull\t0\r\n", line));
            }
            gopher.push_str("i\tfake\tnull\t0\r\n"); // empty line
        }

        fn parse_node(node: &Node, gopher: &mut String) {
            match node {
                Node::Text(text) => {
                    let cleaned = text.trim();
                    if !cleaned.is_empty() {
                        for line in cleaned.lines() {
                            gopher.push_str(&format!("i{}\tfake\tnull\t0\r\n", line));
                        }
                    }
                }
                Node::Element(element) => {
                    match element.name.as_str() {
                        "h1" => {
                            gopher.push_str("i\tfake\tnull\t0\r\n"); // spacing
                            if let Some(Node::Text(text)) = element.children.first() {
                                gopher.push_str(&format!("i# {}\tfake\tnull\t0\r\n", text.trim()));
                            }
                        }
                        "h2" => {
                            gopher.push_str("i\tfake\tnull\t0\r\n");
                            if let Some(Node::Text(text)) = element.children.first() {
                                gopher.push_str(&format!("i## {}\tfake\tnull\t0\r\n", text.trim()));
                            }
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
                                if href.starts_with("http") {
                                    gopher.push_str(&format!(
                                        "h{}\tURL:{}\t\t443\r\n",
                                        text.trim(),
                                        href
                                    ));
                                } else {
                                    gopher.push_str(&format!(
                                        "1{}\t{}\t\t70\r\n",
                                        text.trim(),
                                        href
                                    ));
                                }
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
                                gopher.push_str(&format!("I{}\t{}\t\t70\r\n", alt, src));
                            }
                        }
                        "p" | "div" | "br" => {
                            for child in &element.children {
                                parse_node(child, gopher);
                            }
                            gopher.push_str("i\tfake\tnull\t0\r\n");
                        }
                        "li" => {
                            let mut li_content = String::new();
                            for child in &element.children {
                                if let Node::Text(t) = child {
                                    li_content.push_str(t);
                                }
                            }
                            if !li_content.is_empty() {
                                gopher.push_str(&format!(
                                    "i* {}\tfake\tnull\t0\r\n",
                                    li_content.trim()
                                ));
                            }
                        }
                        _ => {
                            for child in &element.children {
                                parse_node(child, gopher);
                            }
                        }
                    }
                }
                Node::Comment(_) => {}
            }
        }

        for child in &dom.children {
            parse_node(child, &mut gopher);
        }

        gopher.push_str(".\r\n");

        Some(OutputFile::Gopher {
            route: route.clone(),
            content: gopher,
        })
    }
}
