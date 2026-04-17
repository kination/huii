use crate::parser::ast::*;

/// Attempt to generate Rust code deterministically for simple, recognizable patterns.
/// Returns `None` if the flow is too complex for template-based generation,
/// signaling the caller to fall back to LLM generation.
pub fn try_template_generate(flow: &Flow) -> Option<String> {
    let lines = match &flow.implementation {
        Implementation::Inline(lines) => lines,
        _ => return None,
    };

    if lines.is_empty() {
        return None;
    }

    // Try each pattern detector in order
    if let Some(code) = try_print_only(flow, lines) {
        return Some(code);
    }

    if let Some(code) = try_print_with_param(flow, lines) {
        return Some(code);
    }

    if let Some(code) = try_arithmetic(flow, lines) {
        return Some(code);
    }

    if let Some(code) = try_return_literal(flow, lines) {
        return Some(code);
    }

    if let Some(code) = try_min_max(flow, lines) {
        return Some(code);
    }

    None
}

/// Pattern: no params, body is a single "print" statement with a string literal.
/// e.g., `print "hello newlang"` → `println!("hello newlang");`
fn try_print_only(flow: &Flow, lines: &[String]) -> Option<String> {
    if !flow.params.is_empty() || lines.len() != 1 {
        return None;
    }

    let line = lines[0].trim().to_lowercase();
    let message = extract_print_message(&line)?;

    let fn_name = &flow.name;
    Some(format!(
        "fn {fn_name}() {{\n    println!(\"{message}\");\n}}\n\nfn main() {{\n    {fn_name}();\n}}\n"
    ))
}

/// Pattern: one String param, body is "print ... {param}" or "print ... followed by {param}".
/// e.g., `Print "Hello " followed by the name.` with param `name: String`
fn try_print_with_param(flow: &Flow, lines: &[String]) -> Option<String> {
    if flow.params.len() != 1 || lines.len() != 1 {
        return None;
    }

    let param = &flow.params[0];
    let param_type = param.ty.as_ref()?;
    if !matches!(param_type, Type::Str) {
        return None;
    }

    let original_line = lines[0].trim();
    let lower_line = original_line.to_lowercase();
    if !lower_line.contains("print") {
        return None;
    }

    let param_name = &param.name;
    let fn_name = &flow.name;

    // Extract any quoted prefix string (use original casing)
    let prefix = extract_quoted_string(original_line).unwrap_or_default();

    if prefix.is_empty() {
        Some(format!(
            "fn {fn_name}({param_name}: String) {{\n    println!(\"{{}}\", {param_name});\n}}\n\nfn main() {{\n    let {param_name} = String::from(\"test\");\n    {fn_name}({param_name});\n}}\n"
        ))
    } else {
        Some(format!(
            "fn {fn_name}({param_name}: String) {{\n    println!(\"{prefix}{{}}\", {param_name});\n}}\n\nfn main() {{\n    let {param_name} = String::from(\"test\");\n    {fn_name}({param_name});\n}}\n"
        ))
    }
}

/// Extract the message from a print-like intent line.
/// Handles: `print "hello world"`, `print 'hello world'`, `print hello world`
fn extract_print_message(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    if !lower.contains("print") {
        return None;
    }

    // Remove "print" prefix
    let after_print = if let Some(pos) = lower.find("print") {
        let rest = &line[pos + 5..].trim_start();
        // Skip optional quotes around the entire thing
        if rest.starts_with('"') || rest.starts_with('\'') {
            let quote = rest.chars().next().unwrap();
            let inner = &rest[1..];
            if let Some(end) = inner.rfind(quote) {
                inner[..end].to_string()
            } else {
                inner.to_string()
            }
        } else {
            rest.to_string()
        }
    } else {
        return None;
    };

    if after_print.is_empty() {
        return None;
    }

    Some(after_print)
}

/// Extract a quoted string from text (first occurrence).
fn extract_quoted_string(text: &str) -> Option<String> {
    for quote in ['"', '\''] {
        if let Some(start) = text.find(quote) {
            let rest = &text[start + 1..];
            if let Some(end) = rest.find(quote) {
                let extracted = rest[..end].to_string();
                if !extracted.trim().is_empty() {
                    return Some(extracted);
                }
            }
        }
    }
    None
}

/// Map AST Type to Rust type string.
fn rust_type(ty: &Type) -> &'static str {
    match ty {
        Type::I32 => "i32",
        Type::F32 => "f32",
        Type::Bool => "bool",
        Type::Str => "String",
        Type::Void => "()",
        _ => "i32", // fallback for templates
    }
}

/// Map AST Type to a default literal for use in main().
fn default_literal(ty: &Type) -> &'static str {
    match ty {
        Type::I32 => "1",
        Type::F32 => "1.0",
        Type::Bool => "true",
        Type::Str => "\"test\"",
        _ => "0",
    }
}

/// Detect arithmetic operator from natural language intent.
/// Returns the Rust operator (+, -, *, /) if detected.
fn detect_arithmetic_op(line: &str) -> Option<&'static str> {
    let l = line.to_lowercase();

    // Addition
    if l.contains("add") || l.contains("sum") || l.contains("plus") || l.contains("a + b") {
        return Some("+");
    }
    // Subtraction
    if l.contains("subtract") || l.contains("minus") || l.contains("a - b") || l.contains("difference") {
        return Some("-");
    }
    // Multiplication
    if l.contains("multiply") || l.contains("product") || l.contains("a * b") || l.contains("times") {
        return Some("*");
    }
    // Division
    if l.contains("divide") || l.contains("a / b") || l.contains("quotient") {
        return Some("/");
    }
    None
}

/// Pattern: two numeric params of the same type, body is a single arithmetic operation.
/// e.g., `add a and b` with params (a: i32, b: i32) → `fn add(a: i32, b: i32) -> i32 { a + b }`
fn try_arithmetic(flow: &Flow, lines: &[String]) -> Option<String> {
    if flow.params.len() != 2 || lines.len() != 1 {
        return None;
    }

    let p1 = &flow.params[0];
    let p2 = &flow.params[1];
    let ty1 = p1.ty.as_ref()?;
    let ty2 = p2.ty.as_ref()?;

    // Both params must be the same numeric type
    if ty1 != ty2 || !matches!(ty1, Type::I32 | Type::F32) {
        return None;
    }

    let op = detect_arithmetic_op(&lines[0])?;
    let ret_ty = rust_type(ty1);
    let fn_name = &flow.name;
    let a = &p1.name;
    let b = &p2.name;
    let default = default_literal(ty1);

    Some(format!(
        "fn {fn_name}({a}: {ret_ty}, {b}: {ret_ty}) -> {ret_ty} {{\n    {a} {op} {b}\n}}\n\nfn main() {{\n    let result = {fn_name}({default}, {default});\n    println!(\"{{}}\", result);\n}}\n"
    ))
}

/// Pattern: no params, body is "return <literal>".
/// e.g., `return 42` → `fn foo() -> i32 { 42 }`
/// e.g., `return true` → `fn foo() -> bool { true }`
fn try_return_literal(flow: &Flow, lines: &[String]) -> Option<String> {
    if !flow.params.is_empty() || lines.len() != 1 {
        return None;
    }

    let line = lines[0].trim().to_lowercase();
    let value_str = line.strip_prefix("return ")
        .or_else(|| line.strip_prefix("returns "))?
        .trim()
        .trim_end_matches('.');

    if value_str.is_empty() {
        return None;
    }

    // Detect the literal type and Rust expression
    let (ret_ty, rust_expr) = if let Ok(_) = value_str.parse::<i32>() {
        ("i32", value_str.to_string())
    } else if let Ok(_) = value_str.parse::<f32>() {
        // Only match if it looks like a float (contains '.')
        if value_str.contains('.') {
            ("f32", format!("{}_f32", value_str))
        } else {
            return None;
        }
    } else if value_str == "true" || value_str == "false" {
        ("bool", value_str.to_string())
    } else if value_str.starts_with('"') && value_str.ends_with('"') {
        ("String", format!("String::from({})", value_str))
    } else {
        return None;
    };

    let fn_name = &flow.name;
    Some(format!(
        "fn {fn_name}() -> {ret_ty} {{\n    {rust_expr}\n}}\n\nfn main() {{\n    let result = {fn_name}();\n    println!(\"{{}}\", result);\n}}\n"
    ))
}

/// Pattern: two numeric params, body describes max/min comparison.
/// e.g., `return the larger of a and b` → `if a > b { a } else { b }`
/// e.g., `return the minimum of a and b` → `if a < b { a } else { b }`
fn try_min_max(flow: &Flow, lines: &[String]) -> Option<String> {
    if flow.params.len() != 2 || lines.len() != 1 {
        return None;
    }

    let p1 = &flow.params[0];
    let p2 = &flow.params[1];
    let ty1 = p1.ty.as_ref()?;
    let ty2 = p2.ty.as_ref()?;

    if ty1 != ty2 || !matches!(ty1, Type::I32 | Type::F32) {
        return None;
    }

    let line = lines[0].trim().to_lowercase();

    // Detect max or min intent
    let is_max = line.contains("max") || line.contains("larger") || line.contains("greater")
        || line.contains("bigger") || line.contains("highest");
    let is_min = line.contains("min") || line.contains("smaller") || line.contains("lesser")
        || line.contains("lowest");

    if !is_max && !is_min {
        return None;
    }

    let cmp_op = if is_max { ">" } else { "<" };
    let ret_ty = rust_type(ty1);
    let fn_name = &flow.name;
    let a = &p1.name;
    let b = &p2.name;
    let default = default_literal(ty1);

    Some(format!(
        "fn {fn_name}({a}: {ret_ty}, {b}: {ret_ty}) -> {ret_ty} {{\n    if {a} {cmp_op} {b} {{ {a} }} else {{ {b} }}\n}}\n\nfn main() {{\n    let result = {fn_name}({default}, {default});\n    println!(\"{{}}\", result);\n}}\n"
    ))
}
