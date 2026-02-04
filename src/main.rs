mod parser;
mod context;
mod ai;
mod lock;
mod verifier;
mod ir;
mod codegen;
mod cli;

use std::env;
use std::fs;
use crate::context::builder::Context;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: huii <file.bp|file.ais> [-g|--generate] [-m|--model <model>] [--run] [--out <file>] [-v|--verbose]");
        return;
    }

    let file_path = &args[1];
    let generate_mode = args.contains(&"-g".to_string()) || args.contains(&"--generate".to_string());
    let run_mode = args.contains(&"--run".to_string());
    let verbose_mode = args.contains(&"-v".to_string()) || args.contains(&"--verbose".to_string());
    
    // Parse arguments
    let mut model_name = std::env::var("HUII_MODEL").unwrap_or_else(|_| "qwen2.5:0.5b".to_string());
    let mut out_file = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if (arg == "-m" || arg == "--model") && i + 1 < args.len() {
            model_name = args[i + 1].clone();
        } else if arg == "--out" && i + 1 < args.len() {
            out_file = Some(args[i + 1].clone());
        }
        i += 1;
    }

    let file_stem = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("implicit_flow");

    let content = fs::read_to_string(file_path).expect("Failed to read file");
    let mut ctx = Context::new();
    let llm_client = ai::llm_client::LLMClient::new("http://localhost:11434/api/generate", &model_name);

    if file_path.ends_with(".bp") {
        let mut input = content.as_str();
        match parser::bp_parser::parse_program(&mut input) {
            Ok(program) => {
                ctx.register_program(&program, None);
                if verbose_mode {
                    println!("BP Registered: {:#?}", program);
                }
            }
            Err(e) => println!("BP Parse Error: {:?}", e),
        }
    } else if file_path.ends_with(".ais") {
        let mut input = content.as_str();
        match parser::ais_parser::parse_ais(&mut input) {
            Ok(program) => {
                ctx.register_program(&program, Some(file_stem));
                if verbose_mode {
                    println!("AIS Registered: {:#?}", program);
                }
                
                // Process flows
                for decl in &program.declarations {
                    if let parser::ast::Declaration::Flow(f) = decl {
                        let normalized_name = Context::normalize_identifier(&f.name);
                        let target_name = if f.name == "implicit_flow" { file_stem } else { &normalized_name };
                        
                        if let Some(prompt) = ctx.generate_ai_prompt(target_name) {
                            if verbose_mode {
                                println!("\n--- GENERATED PROMPT ---");
                                println!("{}", prompt);
                                println!("------------------------");
                            }

                            if generate_mode || run_mode {
                                if verbose_mode {
                                    println!(">> Sending request to Ollama ({}) for flow '{}'...", model_name, target_name);
                                }
                                match llm_client.generate_code(&prompt).await {
                                    Ok(code) => {
                                        // Robust extraction: Look for ```rust ... ``` or ``` ... ```
                                        let clean_code = if let Some(start) = code.find("```") {
                                            let after_start = &code[start + 3..];
                                            let content_start = after_start.find('\n').map(|n| n + 1).unwrap_or(0);
                                            let real_content = &after_start[content_start..];
                                            if let Some(end) = real_content.find("```") {
                                                real_content[..end].trim()
                                            } else {
                                                real_content.trim()
                                            }
                                        } else {
                                            code.trim()
                                        };

                                        let clean_code = clean_code.to_string();
                                        
                                        if generate_mode || verbose_mode {
                                            println!("\n[AI GENERATED CODE]\n{}\n[END OF CODE]\n", clean_code);
                                        }

                                        if let Some(path) = &out_file {
                                            fs::write(path, &clean_code).expect("Failed to write output file");
                                            if verbose_mode {
                                                println!(">> Code saved to {}", path);
                                            }
                                        }

                                        if run_mode {
                                            if verbose_mode {
                                                println!(">> Compiling and Running...");
                                            }
                                            
                                            // Create a temporary runnable file
                                            let runner_code = format!(
                                                "use std::collections::HashMap;\n\
                                                 type Schema = String;\n\
                                                 #[allow(dead_code)]\n\
                                                 struct Params {{}}\n\
                                                 \n\
                                                 {}\n\
                                                 \n\
                                                 fn main() {{ {}(); }}", 
                                                &clean_code, 
                                                target_name
                                            );
                                            
                                            let temp_file = format!("target/runner_{}.rs", target_name);
                                            fs::create_dir_all("target").ok();
                                            fs::write(&temp_file, runner_code).expect("Failed to write temp runner");
                                            
                                            // Compile using rustc
                                            let compile_output = std::process::Command::new("rustc")
                                                .arg(&temp_file)
                                                .arg("-o")
                                                .arg(format!("target/runner_{}", target_name))
                                                .output()
                                                .expect("Failed to run rustc");
                                                
                                            if compile_output.status.success() {
                                                if verbose_mode {
                                                    println!(">> Execution Output:\n");
                                                }
                                                let run_output = std::process::Command::new(format!("target/runner_{}", target_name))
                                                    .output()
                                                    .expect("Failed to execute runner");
                                                
                                                print!("{}", String::from_utf8_lossy(&run_output.stdout));
                                                eprint!("{}", String::from_utf8_lossy(&run_output.stderr));
                                            } else {
                                                eprintln!(">> Compilation failed.");
                                                eprintln!("{}", String::from_utf8_lossy(&compile_output.stderr));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!(">> Failed to generate code: {}", e);
                                        eprintln!(">> Ensure Ollama is running (e.g., `ollama serve`) and '{}' model is pulled.", model_name);
                                    }
                                }
                            } else {
                                if verbose_mode {
                                    println!(">> Use '-g' to generate, '--run' to execute.");
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => println!("AIS Parse Error: {:?}", e),
        }
    } else {
        println!("Unknown file type: {}", file_path);
    }
}
