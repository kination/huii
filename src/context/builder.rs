use std::collections::HashMap;
use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub struct Context {
    pub schemas: HashMap<String, Schema>,
    pub flows: HashMap<String, Flow>,
    pub metadata: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            flows: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn normalize_identifier(id: &str) -> String {
        id.trim().replace(' ', "_")
    }

    pub fn register_program(&mut self, program: &Program, default_name: Option<&str>) {
        for decl in &program.declarations {
            match decl {
                Declaration::Schema(s) => {
                    self.schemas.insert(Self::normalize_identifier(&s.name), s.clone());
                }
                Declaration::Flow(f) => {
                    let mut flow = f.clone();
                    if flow.name == "implicit_flow" {
                        if let Some(name) = default_name {
                            flow.name = name.to_string();
                        }
                    }
                    flow.name = Self::normalize_identifier(&flow.name);
                    self.flows.insert(flow.name.clone(), flow);
                }
                _ => {}
            }
        }
    }

    fn format_type(ty: &Type) -> String {
        match ty {
            Type::I32 => "i32".to_string(),
            Type::F32 => "f32".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Str => "String".to_string(),
            Type::Void => "()".to_string(),
            Type::Custom(s) => s.clone(),
            Type::Stream(t) => format!("impl Stream<Item={}>", Self::format_type(t)),
            Type::List(t) => format!("Vec<{}>", Self::format_type(t)),
            Type::Option(t) => format!("Option<{}>", Self::format_type(t)),
        }
    }

    fn build_few_shot_section(is_standalone: bool) -> String {
        let mut s = String::new();
        s.push_str("### Example\n\n");

        if is_standalone {
            s.push_str("Specification:\n");
            s.push_str("- Function: `greet`\n");
            s.push_str("- Parameters: name: String\n");
            s.push_str("- Return: ()\n");
            s.push_str("- Intent: Print \"Hello \" followed by name\n\n");
            s.push_str("Generated Code:\n");
            s.push_str("```rust\n");
            s.push_str("fn greet(name: String) {\n");
            s.push_str("    println!(\"Hello {}\", name);\n");
            s.push_str("}\n\n");
            s.push_str("fn main() {\n");
            s.push_str("    let name = String::from(\"world\");\n");
            s.push_str("    greet(name);\n");
            s.push_str("}\n");
            s.push_str("```\n\n");
        } else {
            s.push_str("Specification:\n");
            s.push_str("- Function: `add`\n");
            s.push_str("- Parameters: a: i32, b: i32\n");
            s.push_str("- Return: i32\n");
            s.push_str("- Intent: Return sum of a and b\n\n");
            s.push_str("Generated Code:\n");
            s.push_str("```rust\n");
            s.push_str("fn add(a: i32, b: i32) -> i32 {\n");
            s.push_str("    a + b\n");
            s.push_str("}\n");
            s.push_str("```\n\n");
        }

        s.push_str("Now generate code for the following specification:\n\n");
        s
    }

    fn build_params_signature(flow: &Flow) -> String {
        flow.params.iter().map(|p| {
            let ty = p.ty.as_ref().map(|t| Self::format_type(t)).unwrap_or("()".to_string());
            format!("{}: {}", p.name, ty)
        }).collect::<Vec<_>>().join(", ")
    }

    fn build_return_type(flow: &Flow) -> String {
        flow.return_ty.as_ref().map(|t| Self::format_type(t)).unwrap_or("()".to_string())
    }

    fn build_default_value(ty: &Type) -> String {
        match ty {
            Type::I32 => "0".to_string(),
            Type::F32 => "0.0".to_string(),
            Type::Bool => "false".to_string(),
            Type::Str => "String::from(\"test\")".to_string(),
            Type::Void => "()".to_string(),
            Type::List(inner) => format!("Vec::<{}>::new()", Self::format_type(inner)),
            Type::Option(_) => "None".to_string(),
            _ => "Default::default()".to_string(),
        }
    }

    fn build_main_call(flow: &Flow) -> String {
        let mut main_body = String::new();
        main_body.push_str("    let args: Vec<String> = std::env::args().collect();\n");

        for (i, param) in flow.params.iter().enumerate() {
            let ty = param.ty.as_ref().unwrap_or(&Type::Void);
            let default_val = Self::build_default_value(ty);
            let type_str = Self::format_type(ty);
            
            main_body.push_str(&format!(
                "    let {} = args.get({} + 1).and_then(|s| s.parse::<{}>().ok()).unwrap_or({});\n",
                param.name, i, type_str, default_val
            ));
        }

        let args = flow.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
        let return_type = Self::build_return_type(flow);

        if return_type == "()" {
            main_body.push_str(&format!("    {}({});\n", flow.name, args));
        } else {
            main_body.push_str(&format!("    let result = {}({});\n", flow.name, args));
            main_body.push_str("    println!(\"{:?}\", result);\n");
        }

        format!("fn main() {{\n{}}}", main_body)
    }

    /// Generate a prompt that asks the LLM to produce ONLY the function body.
    /// Returns (prompt, wrapper_template) where wrapper_template contains "{BODY}" placeholder.
    pub fn generate_body_only_prompt(&self, target_flow: &str) -> Option<(String, String)> {
        let flow = self.flows.get(target_flow)?;
        let mut prompt = String::new();

        prompt.push_str("You are a Rust code generator. Generate ONLY the function body (the code inside the braces). Do NOT include the function signature, imports, or main function.\n\n");

        // Few-shot example for body-only
        prompt.push_str("### Example\n");
        prompt.push_str("Function: `greet(name: String) -> ()`\n");
        prompt.push_str("Intent: Print \"Hello \" followed by name\n");
        prompt.push_str("Body:\nprintln!(\"Hello {}\", name);\n\n");

        // Schema context
        if !self.schemas.is_empty() {
            prompt.push_str("### Available Structs\n");
            for schema in self.schemas.values() {
                prompt.push_str(&format!("struct {} {{ ", schema.name));
                let fields: Vec<String> = schema.fields.iter()
                    .map(|f| format!("{}: {}", f.name, Self::format_type(&f.ty)))
                    .collect();
                prompt.push_str(&fields.join(", "));
                prompt.push_str(" }\n");
            }
            prompt.push_str("\n");
        }

        // Target function
        let params_sig = Self::build_params_signature(flow);
        let return_type = Self::build_return_type(flow);
        prompt.push_str(&format!("### Your Task\nFunction: `{}({}) -> {}`\n", flow.name, params_sig, return_type));

        prompt.push_str("Intent:\n");
        if let Implementation::Inline(lines) = &flow.implementation {
            for line in lines {
                prompt.push_str(&format!("- {}\n", line));
            }
        }

        prompt.push_str("\nGenerate ONLY the body code (no `fn`, no `main`, no `use`). Output raw Rust statements only.\n");

        // Build the wrapper template
        let mut wrapper = String::new();
        if return_type == "()" {
            wrapper.push_str(&format!("fn {}({}) {{\n", flow.name, params_sig));
        } else {
            wrapper.push_str(&format!("fn {}({}) -> {} {{\n", flow.name, params_sig, return_type));
        }
        wrapper.push_str("{BODY}\n");
        wrapper.push_str("}\n\n");
        wrapper.push_str(&Self::build_main_call(flow));
        wrapper.push_str("\n");

        Some((prompt, wrapper))
    }

    pub fn generate_ai_prompt(&self, target_flow: &str, is_standalone: bool) -> Option<String> {
        let flow = self.flows.get(target_flow)?;
        let mut prompt = String::new();

        prompt.push_str("You are an AI Coding Agent. Your task is to generate a COMPLETE, RUNNABLE Rust source file based on the following logic specification.\n\n");

        // Few-shot example
        prompt.push_str(&Self::build_few_shot_section(is_standalone));

        // Schemas
        prompt.push_str("### Schemas\n");
        if self.schemas.is_empty() {
            prompt.push_str("None\n");
        }
        for schema in self.schemas.values() {
            prompt.push_str(&format!("struct {} {{\n", schema.name));
            for field in &schema.fields {
                prompt.push_str(&format!("    {}: {},\n", field.name, Self::format_type(&field.ty)));
            }
            prompt.push_str("}\n");
        }

        prompt.push_str("\n### Logic Specification (Flow)\n");
        prompt.push_str(&format!("Function Name: `{}`\n", flow.name));
        prompt.push_str("Parameters:\n");
        if flow.params.is_empty() {
            prompt.push_str("- None\n");
        } else {
            for param in &flow.params {
                let type_str = param.ty.as_ref().map(|t| Self::format_type(t)).unwrap_or("()".to_string());
                prompt.push_str(&format!("- {}: {}\n", param.name, type_str));
            }
        }
        let return_type_str = flow.return_ty.as_ref().map(|t| Self::format_type(t)).unwrap_or("()".to_string());
        prompt.push_str(&format!("Return Type: {}\n", return_type_str));

        prompt.push_str("\n### Implementation Details (Natural Language)\n");
        if let Implementation::Inline(lines) = &flow.implementation {
            for line in lines {
                prompt.push_str(&format!("- {}\n", line));
            }
        }

        prompt.push_str("\n### Structure Instructions\n");
        prompt.push_str("1. Imports (e.g. `use ...`)\n");
        prompt.push_str("2. Helper Structs (if any from Schemas)\n");
        prompt.push_str("3. The Logic Function (`fn `...)\n");

        if is_standalone {
            prompt.push_str("4. The `main` Function: DEFINE `fn main() { ... }` at the END of the file.\n");
            prompt.push_str("   - Inside `main`, parse CLI arguments using `std::env::args()`.\n");
            prompt.push_str("   - Pass the parsed arguments to the logic function.\n");
            prompt.push_str("   - IMPORTANT: `fn main()` must be a function DEFINITION, not a function call.\n");
        } else {
            prompt.push_str("4. (No `main` function)\n");
        }

        prompt.push_str("\n### Coding Guidelines\n");
        if is_standalone {
            prompt.push_str("- `fn main()` must be at the root level, NOT inside any other function.\n");
            prompt.push_str("- Use `String::from` for string literals. Match types exactly.\n");
            prompt.push_str("- In `if/else` expressions returning values, do NOT put a semicolon after the final expression.\n");
            prompt.push_str("- Always use `let` to define new variables.\n");
            prompt.push_str("- Do NOT use unstable features. Use standard library methods only.\n");
            prompt.push_str("- Return `()` from `main`.\n");
        }

        prompt.push_str("- Output ONLY the Rust code. No explanations, no markdown.\n");

        Some(prompt)
    }

    /// Build a fresh correction prompt (does NOT append to original — prevents prompt bloat).
    pub fn build_correction_prompt(failed_code: &str, error_msg: &str) -> String {
        let mut prompt = String::new();
        prompt.push_str("You are an AI Coding Agent. The following Rust code failed to compile. Fix the error and output the COMPLETE corrected Rust source file.\n\n");
        prompt.push_str("### Compiler Error\n");
        let truncated_error = if error_msg.len() > 500 { &error_msg[..500] } else { error_msg };
        prompt.push_str(truncated_error);
        prompt.push_str("\n\n### Failed Code\n```rust\n");
        prompt.push_str(failed_code);
        prompt.push_str("\n```\n\n");
        prompt.push_str("Output ONLY the corrected Rust code. No explanations, no markdown.\n");
        prompt
    }
}
