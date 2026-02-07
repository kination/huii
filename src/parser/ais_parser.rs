use winnow::ascii::{space0, space1, multispace0};
use winnow::combinator::{repeat, alt, opt};
use winnow::token::take_till;
use winnow::{ModalResult as PResult, Parser};

use super::ast::*;

// Internal helper enum for parsing mixed content
#[derive(Debug)]
enum ParsedItem {
    Import(String),
    Flow(Flow),
    Func(Flow), // func keyword (explicit main flow)
    Intent(String), // Top-level intent
}

fn skip_comments_and_space(input: &mut &str) -> PResult<()> {
    let _: Vec<()> = repeat(0.., alt((
        " ".value(()),
        "\t".value(()),
        "\n".value(()),
        "\r".value(()),
        ("//", take_till(0.., ('\r', '\n'))).value(()),
    ))).parse_next(input)?;
    Ok(())
}

pub fn parse_ais(input: &mut &str) -> PResult<Program> {
    skip_comments_and_space.parse_next(input)?;
    
    let items: Vec<ParsedItem> = repeat(0.., parse_parsed_item).parse_next(input)?;
    
    let mut declarations = Vec::new();
    let mut main_func: Option<Flow> = None;
    let mut implicit_intents = Vec::new();
    
    for item in items {
        match item {
            ParsedItem::Import(path) => declarations.push(Declaration::Import(path)),
            ParsedItem::Flow(flow) => declarations.push(Declaration::Flow(flow)),
            ParsedItem::Func(flow) => {
                // If multiple funcs are defined, we take the last one or error? 
                // For now, let's just override or treat as main.
                main_func = Some(flow);
            },
            ParsedItem::Intent(line) => implicit_intents.push(line),
        }
    }
    
    if let Some(mut func_flow) = main_func {
        // Explicit Entry
        // func acts as the main flow.
        // We name it "implicit_flow" so the caller maps it to the filename.
        func_flow.name = "implicit_flow".to_string();
        declarations.push(Declaration::Flow(func_flow));
        
        // Note: implicit_intents (top-level) are ignored or invalid if func is present.
        // For robustness, we ignore them here, or could warn.
    } else {
        // Implicit Entry
        // If no func, check if we have top-level intents.
        // Even if empty, if it's a file without func, it's effectively an implicit main flow (empty).
        // But we only add it if there are intents OR it's a script.
        // Standard rule: If func missing, entire file is logic.
        
        if !implicit_intents.is_empty() || declarations.is_empty() {
             declarations.push(Declaration::Flow(Flow {
                name: "implicit_flow".to_string(),
                params: vec![],
                return_ty: None,
                implementation: Implementation::Inline(implicit_intents),
            }));
        } 
        // If declarations exist (e.g. imports only) and no intents, it's just a module?
        // User rule: "If func is missing, the entire file content is the main flow."
        // So yes, we should create implicit_flow containing collected intents.
    }
    
    Ok(Program { declarations })
}

fn parse_parsed_item(input: &mut &str) -> PResult<ParsedItem> {
    (multispace0, 
        alt((
            parse_ais_import.map(ParsedItem::Import),
            parse_func_block.map(ParsedItem::Func),
            parse_ais_flow_block.map(ParsedItem::Flow),
            parse_intent_line.map(ParsedItem::Intent),
        ))
    )
    .map(|res: (&str, ParsedItem)| res.1)
    .parse_next(input)
}

fn parse_ais_import(input: &mut &str) -> PResult<String> {
    ("import", space1, "\"", take_till(0.., '\"'), "\"")
        .map(|res: (&str, &str, &str, &str, &str)| res.3.to_string())
        .parse_next(input)
}

// func(params)
fn parse_func_block(input: &mut &str) -> PResult<Flow> {
    (
        "func",
        opt( (space0, "(", take_till(0.., ')'), ")") ), // Optional params
        opt((space0, ":")), // Optional colon
        multispace0,
        parse_intent_lines
    )
    .map(|res| Flow {
        name: "implicit_flow".to_string(), // Placeholder, updated in aggregation
        params: vec![], // For MVP
        return_ty: None,
        implementation: Implementation::Inline(res.4),
    })
    .parse_next(input)
}

// flow name (params)
// or @impl name
// or @test flow name
fn parse_ais_flow_block(input: &mut &str) -> PResult<Flow> {
    (
        opt((space0, "@test", multispace0)), // Optional test attribute
        alt(("flow", "@impl")),
        space1,
        take_till(1.., ('(', '\n', '\r', ' ', ':')), // Name
        opt( (space0, "(", take_till(0.., ')'), ")") ), // Optional params
        opt((space0, ":")), // Optional colon
        multispace0,
        parse_intent_lines
    )
    .map(|res| Flow {
        name: res.3.trim().to_string(),
        params: vec![], // For MVP
        return_ty: None,
        implementation: Implementation::Inline(res.7),
    })
    .parse_next(input)
}

fn parse_intent_lines(input: &mut &str) -> PResult<Vec<String>> {
    repeat(1.., parse_intent_line)
        .parse_next(input)
}

fn parse_intent_line(input: &mut &str) -> PResult<String> {
    (skip_comments_and_space, "-", space0, take_till(1.., ('\r', '\n')))
        .map(|res: ((), &str, &str, &str)| res.3.trim().to_string())
        .parse_next(input)
}
