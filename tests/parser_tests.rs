use huii::parser::ais_parser::parse_ais;
use huii::parser::ast::{Declaration, Type, Implementation};

#[test]
fn test_basic_structure() {
    let content = r#"
---
in:
  - key: email
    type: string
out:
  - key: is_valid
    type: boolean
---
Check if email is valid.
"#;
    let mut input = content.trim();
    let program = parse_ais(&mut input).expect("Failed to parse");

    assert_eq!(program.declarations.len(), 1);
    if let Declaration::Flow(flow) = &program.declarations[0] {
        assert_eq!(flow.name, "implicit_flow");
        assert_eq!(flow.params.len(), 1);
        assert_eq!(flow.params[0].name, "email");
        assert_eq!(flow.params[0].ty, Some(Type::Str));
        assert_eq!(flow.return_ty, Some(Type::Bool));
        
        if let Implementation::Inline(lines) = &flow.implementation {
            assert_eq!(lines[0], "Check if email is valid.");
        }
    } else {
        panic!("Expected flow");
    }
}

#[test]
fn test_multiple_params() {
    let content = r#"
---
in:
  - key: a
    type: integer
  - key: b
    type: float
out:
  - key: c
    type: void
---
Body
"#;
    let mut input = content.trim();
    let program = parse_ais(&mut input).expect("Failed to parse");
    
    if let Declaration::Flow(flow) = &program.declarations[0] {
        assert_eq!(flow.params.len(), 2);
        assert_eq!(flow.params[0].ty, Some(Type::I32));
        assert_eq!(flow.params[1].ty, Some(Type::F32));
        assert_eq!(flow.return_ty, Some(Type::Void));
    }
}

#[test]
fn test_custom_type() {
    let content = r#"
---
in:
  - key: user
    type: UserProfile
out: []
---
Process user
"#;
    let mut input = content.trim();
    let program = parse_ais(&mut input).expect("Failed to parse");
    
    if let Declaration::Flow(flow) = &program.declarations[0] {
        assert_eq!(flow.params[0].ty, Some(Type::Custom("UserProfile".to_string())));
        assert_eq!(flow.return_ty, None);
    }
}
