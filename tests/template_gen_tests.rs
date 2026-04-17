use huii::codegen::template_gen::try_template_generate;
use huii::parser::ast::*;

fn make_flow(name: &str, params: Vec<Param>, lines: Vec<&str>) -> Flow {
    Flow {
        name: name.to_string(),
        params,
        return_ty: None,
        implementation: Implementation::Inline(lines.into_iter().map(String::from).collect()),
    }
}

#[test]
fn test_print_only() {
    let flow = make_flow("hello", vec![], vec!["print \"hello newlang\""]);
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("println!(\"hello newlang\")"));
    assert!(code.contains("fn main()"));
    assert!(code.contains("fn hello()"));
}

#[test]
fn test_print_with_param() {
    let flow = make_flow(
        "greet",
        vec![Param { name: "name".to_string(), ty: Some(Type::Str) }],
        vec!["Print \"Hello \" followed by the name."],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("println!(\"Hello {}\""));
    assert!(code.contains("fn greet(name: String)"));
}

#[test]
fn test_arithmetic_add() {
    let flow = make_flow(
        "add",
        vec![
            Param { name: "a".to_string(), ty: Some(Type::I32) },
            Param { name: "b".to_string(), ty: Some(Type::I32) },
        ],
        vec!["add a and b"],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("fn add(a: i32, b: i32) -> i32"));
    assert!(code.contains("a + b"));
    assert!(code.contains("fn main()"));
}

#[test]
fn test_arithmetic_multiply() {
    let flow = make_flow(
        "mul",
        vec![
            Param { name: "x".to_string(), ty: Some(Type::F32) },
            Param { name: "y".to_string(), ty: Some(Type::F32) },
        ],
        vec!["multiply x by y"],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("fn mul(x: f32, y: f32) -> f32"));
    assert!(code.contains("x * y"));
}

#[test]
fn test_arithmetic_subtract() {
    let flow = make_flow(
        "sub",
        vec![
            Param { name: "a".to_string(), ty: Some(Type::I32) },
            Param { name: "b".to_string(), ty: Some(Type::I32) },
        ],
        vec!["subtract b from a"],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("a - b"));
}

#[test]
fn test_return_integer() {
    let flow = make_flow("get_answer", vec![], vec!["return 42"]);
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("fn get_answer() -> i32"));
    assert!(code.contains("42"));
}

#[test]
fn test_return_bool() {
    let flow = make_flow("is_ready", vec![], vec!["return true"]);
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("fn is_ready() -> bool"));
    assert!(code.contains("true"));
}

#[test]
fn test_max_pattern() {
    let flow = make_flow(
        "find_max",
        vec![
            Param { name: "a".to_string(), ty: Some(Type::I32) },
            Param { name: "b".to_string(), ty: Some(Type::I32) },
        ],
        vec!["return the larger of a and b"],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("fn find_max(a: i32, b: i32) -> i32"));
    assert!(code.contains("if a > b { a } else { b }"));
}

#[test]
fn test_min_pattern() {
    let flow = make_flow(
        "find_min",
        vec![
            Param { name: "a".to_string(), ty: Some(Type::I32) },
            Param { name: "b".to_string(), ty: Some(Type::I32) },
        ],
        vec!["return the minimum of a and b"],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_some());
    let code = result.unwrap();
    assert!(code.contains("if a < b { a } else { b }"));
}

#[test]
fn test_complex_flow_returns_none() {
    let flow = make_flow(
        "compute",
        vec![
            Param { name: "a".to_string(), ty: Some(Type::I32) },
            Param { name: "b".to_string(), ty: Some(Type::I32) },
        ],
        vec!["calculate a + b", "if result > 100 return 100"],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_none());
}

#[test]
fn test_mismatched_types_returns_none() {
    let flow = make_flow(
        "bad",
        vec![
            Param { name: "a".to_string(), ty: Some(Type::I32) },
            Param { name: "b".to_string(), ty: Some(Type::Str) },
        ],
        vec!["add a and b"],
    );
    let result = try_template_generate(&flow);
    assert!(result.is_none());
}
