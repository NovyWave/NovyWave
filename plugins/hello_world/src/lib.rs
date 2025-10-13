mod bindings {
    wit_bindgen::generate!({
        path: "./wit",
    });
}

use bindings::{__export_world_plugin_cabi, novywave::hello_world::host, Guest};

struct HelloWorld;

impl Guest for HelloWorld {
    fn init() {
        host::log_info("Hello World!");
    }

    fn shutdown() {
        host::log_info("hello_world plugin shutting down");
    }
}

__export_world_plugin_cabi!(HelloWorld with_types_in bindings);
