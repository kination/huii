use winnow::ascii::multispace0;
use winnow::token::{take_until, rest};
use winnow::{ModalResult as PResult, Parser};
use serde::Deserialize;
use super::ast::*;

#[derive(Deserialize, Debug)]
struct FrontMatter {
    #[serde(rename = "in", default)]
    inputs: Vec<IOField>,
    #[serde(rename = "out", default)]
    outputs: Vec<IOField>,
}

#[derive(Deserialize, Debug)]
struct IOField {
    key: String,
    #[serde(rename = "type")]
    ty: String,
}

pub fn parse_has(input: &mut &str) -> PResult<Program> {
    // 1. Parse Frontmatter Block
    let _ = multispace0(input)?;
    let _ = "---".parse_next(input)?;
    let frontmatter_str = take_until(0.., "---").parse_next(input)?;
    let _ = "---".parse_next(input)?;
    
    // 2. Parse YAML
    let frontmatter: FrontMatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|_| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?; // Simplified error mapping

    // 3. Parse Body (Rest of the file)
    let body_str: &str = rest.parse_next(input)?;
    let body_lines: Vec<String> = body_str.lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // 4. Construct AST
    let params = frontmatter.inputs.into_iter().map(|f| Param {
        name: f.key,
        ty: Some(parse_type_from_str(&f.ty)),
    }).collect();

    let return_ty = if frontmatter.outputs.is_empty() {
        None
    } else if frontmatter.outputs.len() == 1 {
        Some(parse_type_from_str(&frontmatter.outputs[0].ty))
    } else {
        // Ensure complex output is handled. For now assuming Custom type.
        Some(Type::Custom("ComplexOutput".to_string()))
    };

    let flow = Flow {
        name: "implicit_flow".to_string(),
        params,
        return_ty,
        implementation: Implementation::Inline(body_lines),
    };

    Ok(Program {
        declarations: vec![Declaration::Flow(flow)],
    })
}

fn parse_type_from_str(s: &str) -> Type {
    match s.to_lowercase().as_str() {
        "string" | "str" => Type::Str,
        "bool" | "boolean" => Type::Bool,
        "integer" | "int" | "i32" => Type::I32,
        "float" | "f32" => Type::F32,
        "void" => Type::Void,
        _ => Type::Custom(s.to_string()),
    }
}
