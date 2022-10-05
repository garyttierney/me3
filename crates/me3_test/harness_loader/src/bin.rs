use std::error::Error;

use pollster::FutureExt as _;

fn main() -> Result<(), Box<dyn Error>> {
    // let handler = OffsetHandler::new("test/cache.json")?;
    // let nt_ctx = NtContext::resolve_local(&handler).block_on()?;

    // println!("{:#?}", nt_ctx);

    Ok(())
}
