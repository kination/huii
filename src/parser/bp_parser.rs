use winnow::ascii::{alphanumeric1, space1, multispace0, line_ending, space0};
use winnow::combinator::{opt, repeat, separated, alt};
use winnow::token::{take_till, take_while};
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

pub fn parse_program(input: &mut &str) -> PResult<Program> {
    (multispace0, repeat(0.., parse_declaration), multispace0)
        .map(|(_, declarations, _)| Program { declarations })
        .parse_next(input)
}

fn parse_declaration(input: &mut &str) -> PResult<Declaration> {
    (multispace0, 
        winnow::combinator::alt((
            parse_import.map(Declaration::Import),
            parse_schema.map(Declaration::Schema),
            parse_flow.map(Declaration::Flow),
        ))
    )
    .map(|(_, dec)| dec)
    .parse_next(input)
}

fn parse_import(input: &mut &str) -> PResult<String> {
    ("import", space1, "\"", take_till(0.., '\"'), "\"")
        .map(|res: (&str, &str, &str, &str, &str)| res.3.to_string())
        .parse_next(input)
}

fn parse_schema(input: &mut &str) -> PResult<Schema> {
    ("schema", space1, alphanumeric1, multispace0, "{", multispace0, 
        separated(0.., parse_field, (multispace0, opt(","), multispace0)),
    multispace0, "}")
    .map(|res: (&str, &str, &str, &str, &str, &str, Vec<Field>, &str, &str)| Schema {
        name: res.2.to_string(),
        fields: res.6,
    })
    .parse_next(input)
}

fn parse_field(input: &mut &str) -> PResult<Field> {
    (alphanumeric1, multispace0, ":", multispace0, parse_type)
        .map(|res: (&str, &str, &str, &str, Type)| Field {
            name: res.0.to_string(),
            ty: res.4,
        })
        .parse_next(input)
}

fn parse_flow(input: &mut &str) -> PResult<Flow> {
    ("flow", space1, alphanumeric1, multispace0, "(", 
        separated(0.., parse_param, (multispace0, opt(","), multispace0)),
    ")", multispace0, opt(("->", multispace0, parse_type)), multispace0, "{", multispace0, "}")
    .map(|res: (&str, &str, &str, &str, &str, Vec<Param>, &str, &str, Option<(&str, &str, Type)>, &str, &str, &str, &str)| Flow {
        name: res.2.to_string(),
        params: res.5,
        return_ty: res.8.map(|r| r.2),
        implementation: Implementation::Empty,
    })
    .parse_next(input)
}

fn parse_param(input: &mut &str) -> PResult<Param> {
    (alphanumeric1, multispace0, opt((":", multispace0, parse_type)))
        .map(|(name, _, ty)| Param {
            name: name.to_string(),
            ty: ty.map(|t| t.2),
        })
        .parse_next(input)
}

fn parse_type(input: &mut &str) -> PResult<Type> {
    winnow::combinator::alt((
        "i32".value(Type::I32),
        "f32".value(Type::F32),
        "bool".value(Type::Bool),
        "str".value(Type::Str),
        "void".value(Type::Void),
        alphanumeric1.map(|s: &str| Type::Custom(s.to_string())),
    ))
    .parse_next(input)
}
