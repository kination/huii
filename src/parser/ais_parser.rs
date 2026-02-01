use winnow::ascii::{alphanumeric1, space0, space1, multispace0};
use winnow::combinator::{repeat, alt, opt};
use winnow::token::take_till;
use winnow::{ModalResult as PResult, Parser};

use super::ast::*;

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
    
    // Check if it's a standard AIS with @impl or flow
    let declarations: Vec<Declaration> = repeat(0.., parse_ais_declaration).parse_next(input)?;
    
    if declarations.is_empty() && !input.trim().is_empty() {
        // Micro-Script Mode: treat entire content as an implicit flow
        let intents = parse_intent_lines.parse_next(input)?;
        Ok(Program {
            declarations: vec![Declaration::Flow(Flow {
                name: "implicit_flow".to_string(),
                params: vec![],
                return_ty: None,
                implementation: Implementation::Inline(intents),
            })],
        })
    } else {
        Ok(Program { declarations })
    }
}

fn parse_ais_declaration(input: &mut &str) -> PResult<Declaration> {
    (multispace0, 
        winnow::combinator::alt((
            parse_ais_import.map(Declaration::Import),
            parse_ais_impl_block.map(Declaration::Flow),
            parse_ais_pure_flow.map(Declaration::Flow),
        ))
    )
    .map(|res: (&str, Declaration)| res.1)
    .parse_next(input)
}

fn parse_ais_import(input: &mut &str) -> PResult<String> {
    ("import", space1, "\"", take_till(0.., '\"'), "\"")
        .map(|res: (&str, &str, &str, &str, &str)| res.3.to_string())
        .parse_next(input)
}

// @impl name
//   - intent line
fn parse_ais_impl_block(input: &mut &str) -> PResult<Flow> {
    ("@impl", space1, alphanumeric1, multispace0, parse_intent_lines)
        .map(|res: (&str, &str, &str, &str, Vec<String>)| Flow {
            name: res.2.to_string(),
            params: vec![],
            return_ty: None,
            implementation: Implementation::Inline(res.4),
        })
        .parse_next(input)
}

// flow name(params)
//   - intent line
fn parse_ais_pure_flow(input: &mut &str) -> PResult<Flow> {
    ("flow", space1, take_till(0.., '('), "(", take_till(0.., ')'), ")", multispace0, parse_intent_lines)
        .map(|res: (&str, &str, &str, &str, &str, &str, &str, Vec<String>)| Flow {
            name: res.2.trim().to_string(),
            params: vec![], // For MVP, keeping params simple
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
