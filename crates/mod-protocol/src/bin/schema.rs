use me3_mod_protocol::profile::ModProfile;
use schemars::schema_for;

pub fn main() {
    let schema = schema_for!(ModProfile);
    println!(
        "{}",
        serde_json::to_string_pretty(&schema).expect("failed to generate schema JSON")
    );
}
