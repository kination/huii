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
                    // Rename implicit flow if default_name is provided
                    if flow.name == "implicit_flow" {
                        if let Some(name) = default_name {
                            flow.name = name.to_string();
                        }
                    }
                    // Normalize name (e.g., "create user" -> "create_user")
                    flow.name = Self::normalize_identifier(&flow.name);
                    self.flows.insert(flow.name.clone(), flow);
                }
                _ => {}
            }
        }
    }

    pub fn generate_ai_prompt(&self, target_flow: &str) -> Option<String> {
        let flow = self.flows.get(target_flow)?;
        let mut prompt = String::new();

        prompt.push_str("You are an AILang code generator. Translate the following intent into optimized Rust code.\n\n");
        
        prompt.push_str("### Context: Schemas\n");
        for schema in self.schemas.values() {
            prompt.push_str(&format!("schema {} {{\n", schema.name));
            for field in &schema.fields {
                prompt.push_str(&format!("    {}: {:?}\n", field.name, field.ty));
            }
            prompt.push_str("}\n");
        }

        prompt.push_str("\n### Target Flow Signature\n");
        let params_str = if flow.params.is_empty() { "" } else { "params" };
        prompt.push_str(&format!("flow {}({}) -> {:?}\n", flow.name, params_str, flow.return_ty));

        prompt.push_str("\n### Intent\n");
        if let Implementation::Inline(lines) = &flow.implementation {
            for line in lines {
                prompt.push_str(&format!("- {}\n", line));
            }
        }

        prompt.push_str("\n### Requirements\n");
        prompt.push_str("- Output ONLY the raw Rust code for the function.\n");
        prompt.push_str("- NO markdown fencing (```rust).\n");
        prompt.push_str("- NO conversational text or explanations.\n");
        prompt.push_str("- Use the provided schema definitions.\n");
        prompt.push_str(&format!("- The function name MUST be `{}`.\n", flow.name));
        if flow.params.is_empty() {
             prompt.push_str("- The function should take NO arguments (e.g., `fn hello()`).\n");
        }
        prompt.push_str("- Assume standard imports like `std::collections::HashMap` are available.\n");
        prompt.push_str("- DO NOT add standard imports (like `use std::...`) inside the function or block. They are already provided globally.\n");
        prompt.push_str("- If the intent is simple (like print), use `println!`.\n");

        Some(prompt)
    }
}
