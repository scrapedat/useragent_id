use anyhow::{anyhow, Result};
use async_trait::async_trait;
use wasmtime::{Config, Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

#[async_trait]
pub trait WasmAgent: Send + Sync {
    async fn execute(&mut self, input: &str) -> Result<String>;
}

pub struct WasmAgentDispatcher {
    engine: Engine,
}

impl WasmAgentDispatcher {
    pub fn new() -> Result<Self> {
        let mut cfg = Config::new();
        cfg.wasm_multi_memory(true).wasm_multi_value(true);
        let engine = Engine::new(&cfg)?;
        Ok(Self { engine })
    }

    pub async fn load_agent(&self, wasm_path: &str) -> Result<DynamicWasmAgent> {
        let module = Module::from_file(&self.engine, wasm_path)?;
    let mut linker = Linker::new(&self.engine);
    let mut store = Store::new(&self.engine, ());
    let instance = linker.instantiate(&mut store, &module)?;
        // Get memory export
        let memory = instance
            .get_export(&mut store, "memory")
            .and_then(|e| e.into_memory())
            .ok_or_else(|| anyhow!("wasm module has no exported memory"))?;

        // Get functions
    let alloc: TypedFunc<u32, u32> = instance.get_typed_func(&mut store, "alloc")?;
    let dealloc: TypedFunc<(u32, u32), ()> = instance.get_typed_func(&mut store, "dealloc")?;
    let execute: TypedFunc<(u32, u32), u64> = instance.get_typed_func(&mut store, "execute")?;

        Ok(DynamicWasmAgent { instance, store, memory, alloc, dealloc, execute })
    }
}

pub struct DynamicWasmAgent {
    instance: Instance,
    store: Store<()>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,
    dealloc: TypedFunc<(u32, u32), ()>,
    execute: TypedFunc<(u32, u32), u64>,
}

#[async_trait]
impl WasmAgent for DynamicWasmAgent {
    async fn execute(&mut self, input: &str) -> Result<String> {
        // Allocate guest buffer
        let in_bytes = input.as_bytes();
        let in_len = in_bytes.len() as u32;
        let ptr = self.alloc.call(&mut self.store, in_len)?;

        // Write into guest memory
        let data = self
            .memory
            .data_mut(&mut self.store);
        let start = ptr as usize;
        let end = start + in_len as usize;
        if end > data.len() { return Err(anyhow!("guest memory overflow")); }
        data[start..end].copy_from_slice(in_bytes);

        // Call execute
    let packed = self.execute.call(&mut self.store, (ptr, in_len))?;
        // We can free input now
    let _ = self.dealloc.call(&mut self.store, (ptr, in_len));

        if packed == 0 { return Err(anyhow!("execute returned 0 (error)")); }
        let out_ptr = (packed >> 32) as u32;
        let out_len = (packed & 0xFFFF_FFFF) as u32;

        // Read output
        let data = self.memory.data(&self.store);
        let start = out_ptr as usize;
        let end = start + out_len as usize;
        if end > data.len() { return Err(anyhow!("guest memory out of bounds")); }
        let out = std::str::from_utf8(&data[start..end])?.to_string();

        // Free output
    let _ = self.dealloc.call(&mut self.store, (out_ptr, out_len));
        Ok(out)
    }
}
