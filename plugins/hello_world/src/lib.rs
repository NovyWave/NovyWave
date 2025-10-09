mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        world: "runtime",
    });
}

use bindings::{
    Guest, __export_world_runtime_cabi, _export_greet_cabi, _export_init_cabi,
    _export_shutdown_cabi, export,
};

const GREETING: &str = "NovyWave";

struct HelloWorld;

impl Guest for HelloWorld {
    fn init() {
        let _ = GREETING;
    }

    fn greet() {
        let _ = GREETING;
    }

    fn shutdown() {}
}

export!(HelloWorld);
