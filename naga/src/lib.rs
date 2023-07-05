mod utils;

use wasm_bindgen::prelude::*;

use lazy_static::lazy_static;
use naga::Module;
use std::collections::HashMap;
use std::sync::Mutex;

// Reference: https://github.com/pjoe/wasm-naga

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

struct ModuleStore {
    store: HashMap<usize, Module>,
    next_idx: usize,
}

impl ModuleStore {
    fn append(&mut self, module: Module) -> usize {
        let idx = self.next_idx;
        self.next_idx += 1;
        self.store.insert(idx, module);
        idx
    }
    fn remove(&mut self, idx: usize) -> Option<Module> {
        self.store.remove(&idx)
    }
}

lazy_static! {
    static ref MODULES: Mutex<ModuleStore> = Mutex::new(ModuleStore {
        store: HashMap::new(),
        next_idx: 0,
    });
}

#[cfg(feature = "wgsl-out")]
#[wasm_bindgen]
pub fn wgsl_out(module: usize) -> Result<String, JsValue> {
    wgsl_out_inner(module).map_err(|e| e.into())
}

#[cfg(feature = "wgsl-out")]
pub fn wgsl_out_inner(module: usize) -> Result<String, String> {
    utils::set_panic_hook();
    match MODULES.lock().unwrap().remove(module) {
        None => Err("module not found".into()),
        Some(module) => {
            use naga::back::wgsl;
            let analysis = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                Default::default(),
            )
            .validate(&module)
            .map_err(|e| format!("{}", e))?;
            let result = wgsl::write_string(&module, &analysis, wgsl::WriterFlags::empty())
                .map_err(|e| format!("{}", e))?;
            Ok(result)
        }
    }
}

#[cfg(feature = "spv-in")]
#[wasm_bindgen]
pub fn spv_in(input: &[u8]) -> Result<usize, JsValue> {
    spv_in_inner(input).map_err(|e| e.into())
}
#[cfg(feature = "spv-in")]
pub fn spv_in_inner(input: &[u8]) -> Result<usize, String> {
    utils::set_panic_hook();
    let module = naga::front::spv::parse_u8_slice(&input, &Default::default())
        .map_err(|e| format!("{}", e))?;
    Ok(MODULES.lock().unwrap().append(module))
}
