use huii::parser::hbp_parser::parse_program;
use huii::parser::ast::{Declaration, Variant, Type};

#[test]
fn test_enum_parsing() {
    let content = r#"
enum Status {
    Active,
    Inactive,
    Suspended(String),
    Complex { reason: String, code: i32 }
}
"#;
    let mut input = content.trim();
    let program = parse_program(&mut input).expect("Failed to parse enum");

    assert_eq!(program.declarations.len(), 1);
    
    if let Declaration::Enum(e) = &program.declarations[0] {
        assert_eq!(e.name, "Status");
        assert_eq!(e.variants.len(), 4);

        // Active
        if let Variant::Unit(name) = &e.variants[0] {
            assert_eq!(name, "Active");
        } else {
            panic!("Expected Unit variant Active");
        }

        // Inactive
        if let Variant::Unit(name) = &e.variants[1] {
            assert_eq!(name, "Inactive");
        } else {
            panic!("Expected Unit variant Inactive");
        }

        // Suspended(String)
        if let Variant::Tuple(name, types) = &e.variants[2] {
            assert_eq!(name, "Suspended");
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], Type::Custom("String".to_string()));
        } else {
            panic!("Expected Tuple variant Suspended");
        }

        // Complex { reason: String, code: i32 }
        if let Variant::Struct(name, fields) = &e.variants[3] {
            assert_eq!(name, "Complex");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "reason");
            assert_eq!(fields[0].ty, Type::Custom("String".to_string()));
            assert_eq!(fields[1].name, "code");
            assert_eq!(fields[1].ty, Type::I32);
        } else {
            panic!("Expected Struct variant Complex");
        }

    } else {
        panic!("Expected Enum declaration");
    }
}
