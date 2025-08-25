use anyhow::Result;
use shared::types::Agent;
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

    // For now, we assume the agent is a compiled binary that can be run directly.
    // We will need to handle different agent types (Wasm, Python, etc.) later.
    let output = Command::new(&agent.executable_path)
        .output()?;

    println!("Agent STDOUT:");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    
    if !output.stderr.is_empty() {
        println!("Agent STDERR:");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if output.status.success() {
        println!("Agent {} finished execution successfully.", agent.id);
    } else {
        println!("Agent {} finished with an error.", agent.id);
    }

    Ok(())
}
