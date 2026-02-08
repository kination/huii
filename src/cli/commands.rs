use std::fs;
use std::path::Path;
use std::process::Command;
use crate::parser;
use crate::context::builder::Context;
use crate::ai::grammar;
use crate::ai::llm_client::LLMClient;
use crate::codegen::template_gen;
use crate::lock::manager::{LockManager, LockData};
use anyhow::{Result, anyhow};

pub async fn execute_run(file_path: &str, model: Option<String>, force: bool) -> Result<()> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow!("File not found: {}", file_path));
    }

    let model_name = model.unwrap_or_else(|| std::env::var("HUII_MODEL").unwrap_or_else(|_| "codellama:7b".to_string()));

    match path.extension().and_then(|s| s.to_str()) {
        Some("hbp") => parse_hbp_code(path).await,
        Some("has") => parse_has_code(path, &model_name, force).await,
        _ => Err(anyhow!("Unsupported file extension for huii. Use '.hbp' or '.has'")),
    }
}

async fn parse_hbp_code(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let mut input = content.as_str();

    let _program = parser::hbp_parser::parse_program(&mut input)
        .map_err(|e| anyhow!("HBP file parse error: {:?}", e))?;

    println!("HBP file structure is valid.");
    println!("Execution of pure .hbp files requires implementation bodies (not fully supported in parser yet).");

    Ok(())
}

async fn parse_has_code(path: &Path, model_name: &str, force: bool) -> Result<()> {
    let content = fs::read_to_string(path)?;
    // Read from current lock first
    let current_hash = LockManager::compute_hash(&content);
    let lock_path = path.with_extension("lock");

    let mut code_to_run = String::new();
    let mut should_generate = true;

    // 1. Check lock file cache
    if !force {
        if let Ok(Some(lock_data)) = LockManager::load(&lock_path) {
            if lock_data.source_hash == current_hash {
                code_to_run = lock_data.generated_code;
                should_generate = false;
            }
        }
    }

    if should_generate {
        // 2. Parse .has
        let mut input = content.as_str();
        let program = parser::has_parser::parse_ais(&mut input)
             .map_err(|e| anyhow!("HAS Parse Error: {:?}", e))?;

        // 3. Build Context
        let mut ctx = Context::new();
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("implicit_flow");
        ctx.register_program(&program, Some(file_stem));

        // 4. Try template-based generation first (deterministic, no LLM needed)
        let template_result = ctx.flows.get(file_stem)
            .and_then(|flow| template_gen::try_template_generate(flow));

        if let Some(template_code) = template_result {
            code_to_run = template_code;
        } else {
            // 5. LLM-based generation with self-correction
            code_to_run = generate_with_llm(&ctx, file_stem, model_name).await?;
        }

        // 6. Final verification & locking
        let temp_runner_path = Path::new("target/runners").join(format!("{}.rs", file_stem));
        if let Some(parent) = temp_runner_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&temp_runner_path, &code_to_run)?;

        let verifier = crate::verifier::semantic::Verifier::new();
        match verifier.verify(&temp_runner_path) {
            Ok(crate::verifier::semantic::VerificationResult::Success) => {
                let lock_data = LockData {
                    source_hash: current_hash,
                    generated_code: code_to_run.clone(),
                    verified_at: chrono::Utc::now().to_rfc3339(),
                    passed_tests: vec![],
                };
                LockManager::save(&lock_path, &lock_data)?;
            }
            Ok(crate::verifier::semantic::VerificationResult::Failure(_err)) => {
                if lock_path.exists() {
                    let _ = fs::remove_file(&lock_path);
                }
            }
            Err(_e) => {}
        }
    }

    // 7. Execute
    execute_rust_code(&code_to_run, path).await
}

/// LLM-based code generation with improved self-correction loop.
///
/// Strategy:
/// 1. Try body-only generation (simpler prompt, wrapped in template)
/// 2. On failure, try full-file generation with few-shot examples
/// 3. On second failure, use a fresh correction prompt (not appended)
/// 4. Abort early if same error repeats
async fn generate_with_llm(ctx: &Context, file_stem: &str, model_name: &str) -> Result<String> {
    let llm_client = LLMClient::new("http://localhost:11434/api/generate", model_name);
    let verifier = crate::verifier::semantic::Verifier::new();
    let temp_runner_path = Path::new("target/runners").join(format!("{}.rs", file_stem));

    if let Some(parent) = temp_runner_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let max_attempts = 3;
    let mut previous_errors: Vec<String> = Vec::new();
    let mut best_code = String::new();

    for attempt in 0..max_attempts {
        let raw_code = if attempt == 0 {
            // Attempt 1: body-only generation (simplest prompt, most reliable)
            if let Some((body_prompt, wrapper)) = ctx.generate_body_only_prompt(file_stem) {
                let body = llm_client.generate_code(&body_prompt, None).await
                    .map_err(|e| anyhow!("LLM Generation failed: {}", e))?;
                let clean_body = extract_rust_code(&body);
                wrapper.replace("{BODY}", clean_body)
            } else {
                // Fallback to full prompt if body-only not available
                let prompt = ctx.generate_ai_prompt(file_stem, true)
                    .ok_or_else(|| anyhow!("Could not generate prompt for flow '{}'", file_stem))?;
                let raw = llm_client.generate_code(&prompt, Some(grammar::RUST_GBNF)).await
                    .map_err(|e| anyhow!("LLM Generation failed: {}", e))?;
                extract_rust_code(&raw).to_string()
            }
        } else if attempt == 1 {
            // Attempt 2: full-file generation with few-shot + grammar
            let prompt = ctx.generate_ai_prompt(file_stem, true)
                .ok_or_else(|| anyhow!("Could not generate prompt for flow '{}'", file_stem))?;
            let raw = llm_client.generate_code(&prompt, Some(grammar::RUST_GBNF)).await
                .map_err(|e| anyhow!("LLM Generation failed: {}", e))?;
            extract_rust_code(&raw).to_string()
        } else {
            // Attempt 3: fresh correction prompt (NOT appended to original)
            let last_error = previous_errors.last().map(|s| s.as_str()).unwrap_or("Unknown error");
            let correction_prompt = Context::build_correction_prompt(&best_code, last_error);
            let raw = llm_client.generate_code(&correction_prompt, Some(grammar::RUST_GBNF)).await
                .map_err(|e| anyhow!("LLM Correction failed: {}", e))?;
            extract_rust_code(&raw).to_string()
        };

        best_code = raw_code;

        // Verify
        fs::write(&temp_runner_path, &best_code)?;
        match verifier.verify(&temp_runner_path) {
            Ok(crate::verifier::semantic::VerificationResult::Success) => {
                return Ok(best_code);
            }
            Ok(crate::verifier::semantic::VerificationResult::Failure(error)) => {
                // Abort early if same error repeats
                if previous_errors.iter().any(|prev| prev == &error) {
                    break;
                }
                previous_errors.push(error);
            }
            Err(_) => {
                break;
            }
        }
    }

    Ok(best_code)
}

fn extract_rust_code(text: &str) -> &str {
    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        let content_start = after_start.find('\n').map(|n| n + 1).unwrap_or(0);
        let real_content = &after_start[content_start..];
        if let Some(end) = real_content.find("```") {
            real_content[..end].trim()
        } else {
            real_content.trim()
        }
    } else {
        text.trim()
    }
}

async fn execute_rust_code(code: &str, original_path: &Path) -> Result<()> {
    let file_stem = original_path.file_stem().and_then(|s| s.to_str()).unwrap_or("runner");
    let target_dir = Path::new("target/runners");
    fs::create_dir_all(target_dir)?;

    let runner_path = target_dir.join(format!("{}.rs", file_stem));
    let binary_path = target_dir.join(file_stem);

    fs::write(&runner_path, code)?;

    let compile_output = Command::new("rustc")
        .arg(&runner_path)
        .arg("-o")
        .arg(&binary_path)
        .output()?;

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        return Err(anyhow!("Compilation failed:\n{}", stderr));
    }

    let run_output = Command::new(&binary_path).output()?;

    print!("{}", String::from_utf8_lossy(&run_output.stdout));
    eprint!("{}", String::from_utf8_lossy(&run_output.stderr));

    if !run_output.status.success() {
        return Err(anyhow!("Execution failed with status: {}", run_output.status));
    }

    Ok(())
}
