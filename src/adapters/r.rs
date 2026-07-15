use super::base::{collapse_ws, count_parse_errors, LanguageAdapter};
use crate::core::{CallKind, CallSite, Declaration, DeclarationKind, ParseResult};
use ast_grep_core::{Doc, Node};
use std::path::Path;

pub struct RAdapter;

impl LanguageAdapter for RAdapter {
    fn language_name(&self) -> &'static str {
        "r"
    }

    fn parse<'a, D: Doc>(&self, path: &Path, source: &[u8], root: Node<'a, D>) -> ParseResult {
        let mut declarations = Vec::new();
        _walk_top(&root, source, &mut declarations);
        ParseResult {
            path: path.to_path_buf(),
            language: self.language_name(),
            source: source.to_vec(),
            line_count: source.iter().filter(|&&b| b == b'\n').count() + 1,
            error_count: count_parse_errors(root.clone()),
            declarations,
            imports: Vec::new(),
        }
    }
}

fn _walk_top<'a, D: Doc>(node: &Node<'a, D>, src: &[u8], out: &mut Vec<Declaration>) {
    for child in node.children().filter(|child| child.is_named()) {
        if child.kind() != "binary_operator" {
            continue;
        }
        if let Some(declaration) = _assignment_to_decl(&child, src) {
            out.push(declaration);
        }
    }
}

fn _assignment_to_decl<'a, D: Doc>(node: &Node<'a, D>, src: &[u8]) -> Option<Declaration> {
    let operator = node.field("operator")?.text().into_owned();
    let (name_node, value, left_assignment) = match operator.as_str() {
        "<-" | "<<-" | "=" => (node.field("lhs")?, node.field("rhs")?, true),
        "->" | "->>" => (node.field("rhs")?, node.field("lhs")?, false),
        _ => return None,
    };
    if name_node.kind() != "identifier" {
        return None;
    }
    let name = _identifier_text(&name_node);
    if value.kind() == "function_definition" {
        return Some(_function_to_decl(
            node,
            &value,
            src,
            name,
            &operator,
            left_assignment,
        ));
    }

    let signature = if left_assignment {
        format!("{name} {operator} {}", collapse_ws(&value.text()))
    } else {
        format!("{} {operator} {name}", collapse_ws(&value.text()))
    };
    let range = node.range();
    Some(Declaration {
        kind: DeclarationKind::Field,
        name,
        signature,
        bases: Vec::new(),
        attrs: Vec::new(),
        docs: Vec::new(),
        docs_inside: false,
        visibility: String::new(),
        start_line: node.start_pos().line() + 1,
        end_line: node.end_pos().line() + 1,
        start_byte: range.start,
        end_byte: range.end,
        doc_start_byte: range.start,
        native_kind: None,
        modifiers: Vec::new(),
        deprecated: false,
        children: Vec::new(),
        calls: Vec::new(),
    })
}

fn _function_to_decl<'a, D: Doc>(
    assignment: &Node<'a, D>,
    function: &Node<'a, D>,
    src: &[u8],
    name: String,
    operator: &str,
    left_assignment: bool,
) -> Declaration {
    let function_keyword = function
        .field("name")
        .map(|node| node.text().into_owned())
        .unwrap_or_else(|| "function".to_string());
    let parameters = function
        .field("parameters")
        .map(|node| collapse_ws(&node.text()))
        .unwrap_or_else(|| "()".to_string());
    let signature = if left_assignment {
        format!("{name} {operator} {function_keyword}{parameters}")
    } else {
        format!("{function_keyword}{parameters} {operator} {name}")
    };

    let mut calls = Vec::new();
    if let Some(body) = function.field("body") {
        _walk_calls_in_body(&body, src, &mut calls);
    }
    let range = assignment.range();
    Declaration {
        kind: DeclarationKind::Function,
        name,
        signature,
        bases: Vec::new(),
        attrs: Vec::new(),
        docs: Vec::new(),
        docs_inside: false,
        visibility: String::new(),
        start_line: assignment.start_pos().line() + 1,
        end_line: assignment.end_pos().line() + 1,
        start_byte: range.start,
        end_byte: range.end,
        doc_start_byte: range.start,
        native_kind: None,
        modifiers: Vec::new(),
        deprecated: false,
        children: Vec::new(),
        calls,
    }
}

fn _walk_calls_in_body<'a, D: Doc>(node: &Node<'a, D>, src: &[u8], out: &mut Vec<CallSite>) {
    if node.kind() == "function_definition" {
        return;
    }
    if node.kind() == "call" {
        if let Some(call) = _call_site(node, src) {
            out.push(call);
        }
    }
    for child in node.children() {
        _walk_calls_in_body(&child, src, out);
    }
}

fn _call_site<'a, D: Doc>(node: &Node<'a, D>, src: &[u8]) -> Option<CallSite> {
    let function = node.field("function")?;
    let kind = function.kind();
    let (name, receiver) = match kind.as_ref() {
        "identifier" => (_identifier_text(&function), None),
        "namespace_operator" | "extract_operator" => {
            let rhs = function.field("rhs")?;
            let lhs = function.field("lhs")?;
            (
                _identifier_text(&rhs),
                Some(String::from_utf8_lossy(&src[lhs.range()]).to_string()),
            )
        }
        _ => return None,
    };
    Some(CallSite {
        name,
        receiver,
        line: node.start_pos().line() as u32 + 1,
        kind: CallKind::Call,
    })
}

fn _identifier_text<D: Doc>(node: &Node<D>) -> String {
    node.text().trim_matches('`').to_string()
}
