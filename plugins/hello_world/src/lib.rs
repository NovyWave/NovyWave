mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        world: "runtime",
    });
}

use bindings::{__export_world_runtime_cabi, host, Guest};

const GREETING: &str = "NovyWave";

struct HelloWorld;

impl Guest for HelloWorld {
    fn init() {
        let message = format!("{} plugin initialized", GREETING);
        host::log_info(&message);
    }

    fn greet() {
        host::log_info("hello_world plugin greeting from inside Wasm");
    }

    fn shutdown() {
        host::log_info("hello_world plugin shutting down");
    }
}

__export_world_runtime_cabi!(HelloWorld with_types_in bindings);
