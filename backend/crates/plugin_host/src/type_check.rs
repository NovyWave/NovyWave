#![allow(dead_code)]
fn _type_check() {
    let engine = wasmtime::Engine::default();
    let component = wasmtime::component::Component::new(&engine, []).unwrap();
}
