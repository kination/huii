use huii::parser::ais_parser::parse_ais;
use huii::parser::ast::{Declaration, Implementation};

#[test]
fn test_implicit_entry_mixed_content() {
    // Implicit entry with imports and helper flows
    // Similar to particle.has
    let content = r#"
import "particle.hbp"

- gravity: 9.8 m/s^2 down
- Euler integration: pos += vel * dt

@test
flow check_gravity
    - given: Particle(vel=(0,0,0))
    - when: update_particles(dt=1.0)
    - then: vel.y < 0
"#;
    let mut input = content.trim();
    let program = parse_ais(&mut input).expect("Failed to parse");

    // Expect 3 declarations: Import, Implicit Flow (main), Test Flow (helper)
    assert_eq!(program.declarations.len(), 3);

    let import_decl = program.declarations.iter().find(|d| matches!(d, Declaration::Import(_)));
    assert!(import_decl.is_some());

    let main_flow = program.declarations.iter().find(|d| {
        if let Declaration::Flow(f) = d {
            f.name == "implicit_flow"
        } else {
            false
        }
    });
    assert!(main_flow.is_some());
    if let Declaration::Flow(f) = main_flow.unwrap() {
        if let Implementation::Inline(lines) = &f.implementation {
            assert!(lines.iter().any(|l| l.contains("gravity: 9.8")));
            assert!(lines.iter().any(|l| l.contains("Euler integration")));
        } else {
            panic!("Main flow implementation is not Inline");
        }
    }

    let test_flow = program.declarations.iter().find(|d| {
        if let Declaration::Flow(f) = d {
            f.name == "check_gravity"
        } else {
            false
        }
    });
    assert!(test_flow.is_some());
}

#[test]
fn test_explicit_entry_func() {
    // Explicit entry with func
    // Similar to convert_string.has
    let content = r#"
import "main.hbp"

flow convert_to_lower
    - lower logic

func(input, type):
    - if type is upper
    - if type is lower
"#;
    let mut input = content.trim();
    let program = parse_ais(&mut input).expect("Failed to parse");

    // Expect 3 declarations: Import, Helper Flow, Main Flow (func)
    assert_eq!(program.declarations.len(), 3);

    let main_flow = program.declarations.iter().find(|d| {
        if let Declaration::Flow(f) = d {
            f.name == "implicit_flow" // Maps to filename
        } else {
            false
        }
    });
    assert!(main_flow.is_some());
    
    // Ensure helper flow is present
    let helper_flow = program.declarations.iter().find(|d| {
        if let Declaration::Flow(f) = d {
            f.name == "convert_to_lower"
        } else {
            false
        }
    });
    assert!(helper_flow.is_some());

    // Ensure NO implicit flow from top-level intents (there are none, but logic handles it)
}

// Helper function
fn get_flow_name(decl: &Declaration) -> Option<&str> {
    if let Declaration::Flow(f) = decl {
        Some(&f.name)
    } else {
        None
    }
}

#[test]
fn test_identifiers_no_normalization() {
    let content = r#"
flow create_user(name)
    - logic
"#;
    let mut input = content.trim();
    let program = parse_ais(&mut input).expect("Failed to parse");
    
    let flow = program.declarations.iter().find(|d| {
        if let Declaration::Flow(f) = d {
            f.name == "create_user"
        } else {
            false
        }
    });
    assert!(flow.is_some());
    assert_eq!(get_flow_name(flow.unwrap()).unwrap(), "create_user");
}

#[test]
fn test_simple_implicit() {
    let content = r#"
- print "hello"
"#;
    let mut input = content.trim();
    let program = parse_ais(&mut input).expect("Failed to parse");
    
    let main_flow = program.declarations.first().unwrap();
    if let Declaration::Flow(f) = main_flow {
        assert_eq!(f.name, "implicit_flow");
        if let Implementation::Inline(lines) = &f.implementation {
             assert!(lines[0].contains("print \"hello\""));
        }
    } else {
        panic!("Expected Flow declaration");
    }
}
