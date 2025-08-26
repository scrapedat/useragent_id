use anyhow::Result;
use agents_dispatcher::{WasmAgent, WasmAgentDispatcher};

#[tokio::main]
async fn main() -> Result<()> {
    let dispatcher = WasmAgentDispatcher::new()?;
    let wasm_path = std::env::args().nth(1).expect("pass path to echo-wasm.wasm");
    let mut agent = dispatcher.load_agent(&wasm_path).await?;
    let input = r#"{"msg":"hello"}"#;
    let out = agent.execute(input).await?;
    println!("{}", out);
    Ok(())
}
