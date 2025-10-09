use plugin_host::PluginHost;
use shared::AppConfig;

fn main() {
    let config_toml = std::fs::read_to_string(".novywave").expect("read .novywave");
    let config: AppConfig = toml::from_str(&config_toml).expect("parse app config");

    let host = PluginHost::new().expect("init plugin host");

    if config.plugins.entries.is_empty() {
        println!("no plugins configured");
    }

    for entry in &config.plugins.entries {
        if !entry.enabled {
            println!("{} disabled", entry.id);
            continue;
        }

        let mut entry_clone = entry.clone();
        // Resolve relative artifact paths against repo root
        let path = std::path::Path::new(&entry.artifact_path);
        if path.is_relative() {
            let absolute = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(path);
            entry_clone.artifact_path = absolute
                .to_str()
                .unwrap_or(&entry.artifact_path)
                .to_string();
        }

        match host.load(&entry_clone) {
            Ok(handle) => {
                println!("{} ok | init={}", entry.id, handle.init_message());
            }
            Err(err) => {
                println!("{} error: {}", entry.id, err);
            }
        }
    }
}
