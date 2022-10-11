use std::process::exit;

use dll_syringe::{process::OwnedProcess, Syringe};

fn main() {
    // TODO: parameterize
    let target_name = std::env::args().nth(1).expect("expected process name");
    let target = OwnedProcess::find_first_by_name(&target_name).expect("process not found");
    let injector = Syringe::for_process(target);
    let dll_name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "me3_host.dll".to_owned());

    injector
        .inject(dll_name)
        .expect("failed to inject me3_host");

    exit(1);
}
