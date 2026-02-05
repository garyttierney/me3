use me3_launcher_attach_protocol::AttachConfig;

pub fn current_attach_config() -> Option<AttachConfig> {
    let json_var = std::env::var("ME3_ATTACH_CONFIG").ok();
    json_var.map(|v| serde_json::from_str(&v).expect("mod host serialized invalid data"))
}
