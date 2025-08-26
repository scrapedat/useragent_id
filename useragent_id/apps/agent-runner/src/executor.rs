use anyhow::Result;
use eval::{Evaluator, ExecutionStatus};
use memory::{MemoryValue, SharedContext};
use shared::types::{Agent, AgentType};
use agents_dispatcher::{WasmAgent, WasmAgentDispatcher};
use planner::{planner::decompose_task, task::{AgentType as PlanAgentType}};
use std::process::Command;


/// Executes a given agent.
pub fn run_agent(agent: &Agent) -> Result<()> {
    println!("Executing agent: {}", agent.name);
    println!("Description: {}", agent.description);
    println!("Executable Path: {}", agent.executable_path.display());
    println!("---");

    if !agent.executable_path.exists() {
        anyhow::bail!("Agent executable not found at path: {}", agent.executable_path.display());
    }

    // Dispatch by agent type
    let (stdout_s, status) = match agent.agent_type {
        AgentType::Wasm => {
            // Run via Wasm dispatcher (sync host, async trait; we block on a oneshot runtime)
            let dispatcher = WasmAgentDispatcher::new()?;
            let wasm_path = agent.executable_path.to_string_lossy().to_string();
            let mut rt = tokio::runtime::Runtime::new()?;
            let mut wasm_agent = rt.block_on(dispatcher.load_agent(&wasm_path))?;
            let result = rt.block_on(wasm_agent.execute("{\"msg\":\"from-agent-runner\"}"))?;
            (result, ExecutionStatus::Success)
        }
        _ => {
            // For now, assume compiled native binary
            let output = Command::new(&agent.executable_path).output()?;
            println!("Agent STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                println!("Agent STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
            }
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let status = if output.status.success() {
                println!("Agent {} finished execution successfully.", agent.id);
                ExecutionStatus::Success
            } else {
                println!("Agent {} finished with an error.", agent.id);
                ExecutionStatus::Failure(format!("exit: {:?}", output.status.code()))
            };
            (stdout, status)
        }
    };

    let ctx = SharedContext::new();
    let evaluator = Evaluator::new(ctx.clone());
    ctx.set("last_stdout", MemoryValue::string(&stdout_s));
    let trace = match status {
        ExecutionStatus::Success => evaluator.record_success(
            &agent.id.to_string(),
            &agent.name,
            &agent.description,
            &stdout_s,
        ),
        ExecutionStatus::Failure(ref msg) => evaluator.record_failure(
            &agent.id.to_string(),
            &agent.name,
            &agent.description,
            &stdout_s,
            msg,
        ),
        ExecutionStatus::PartialSuccess(p) => evaluator.record_failure(
            &agent.id.to_string(),
            &agent.name,
            &agent.description,
            &stdout_s,
            &format!("partial: {:.2}", p),
        ),
    };
    let _ = evaluator.save_trace(&trace);

    Ok(())
}

/// Given a high-level objective, plan and execute subtasks.
pub fn run_objective(objective: &str) -> Result<()> {
    let ctx = SharedContext::new();
    // Put some seed inputs; in a real flow, these come from UI or memory
    ctx.set("target_url", MemoryValue::string("https://example.com"));

    // Plan
    let task = tokio::runtime::Runtime::new()?.block_on(decompose_task(objective, &ctx))?;
    println!("Planned task {} with {} subtask(s)", task.id, task.subtasks.len());

    // Simple dispatcher selection; map Scrape -> WASM echo for now
    for st in task.subtasks {
        println!("Executing subtask {}: {}", st.id, st.objective);
        match st.required_agent {
            PlanAgentType::Scrape => {
                // Run the native chromiumoxide scraper agent via stdin/stdout JSON
                use std::io::Write;
                use std::process::{Command, Stdio};
                let bin = "target/debug/scraper_chromiumoxide"; // dev builds
                // Derive input URL from shared context or fallback
                let url = match ctx.get("target_url") {
                    Some(MemoryValue::String(s)) => s,
                    _ => "https://example.com".to_string(),
                };
                let input = serde_json::json!({
                    "url": url,
                    "headless": true
                }).to_string();
                let mut child = Command::new(bin)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(input.as_bytes())?;
                }
                let out = child.wait_with_output()?;
                let stdout_s = String::from_utf8_lossy(&out.stdout).to_string();
                let evaluator = Evaluator::new(ctx.clone());
                if out.status.success() {
                    let trace = evaluator.record_success(&st.id, "scraper_chromiumoxide", &input, &stdout_s);
                    let _ = evaluator.save_trace(&trace);
                    ctx.set(&st.output_key, MemoryValue::string(&stdout_s));
                } else {
                    let err_s = String::from_utf8_lossy(&out.stderr).to_string();
                    let trace = evaluator.record_failure(&st.id, "scraper_chromiumoxide", &input, &stdout_s, &err_s);
                    let _ = evaluator.save_trace(&trace);
                }
            }
            PlanAgentType::Custom(_) => {
                let dispatcher = WasmAgentDispatcher::new()?;
                let wasm_path = "target/wasm32-unknown-unknown/release/echo_wasm.wasm";
                let mut wasm_agent = tokio::runtime::Runtime::new()?.block_on(dispatcher.load_agent(wasm_path))?;
                let input = serde_json::json!({
                    "objective": st.objective,
                    "inputs": st.input_keys,
                }).to_string();
                let out = tokio::runtime::Runtime::new()?.block_on(wasm_agent.execute(&input))?;
                let evaluator = Evaluator::new(ctx.clone());
                let trace = evaluator.record_success(&st.id, "wasm-echo", &input, &out);
                let _ = evaluator.save_trace(&trace);
                ctx.set(&st.output_key, MemoryValue::string(&out));
            }
            _ => {
                println!("No runner for agent type; skipping");
            }
        }
    }
    Ok(())
}
