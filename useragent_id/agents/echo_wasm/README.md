# echo-wasm

A minimal WASM agent exposing a tiny ABI for host interaction:
- memory (default linear memory)
- alloc(size: u32) -> ptr
- dealloc(ptr: u32, size: u32)
- execute(in_ptr: u32, in_len: u32) -> u64 where hi32=out_ptr, lo32=out_len

Input and output are UTF-8 JSON strings.
