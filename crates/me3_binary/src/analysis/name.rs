use std::ffi::CString;

#[link(name = "dbghelp")]
extern "system" {
    fn UnDecorateSymbolName(input: *const i8, output: *mut u8, output_size: u32, flags: u32)
        -> u32;
}

pub fn demangle<S: AsRef<str>>(mangled: S) -> Option<String> {
    let fixed_up = mangled.as_ref().trim_start_matches(|c| c == '.');
    let cstr = CString::new(fixed_up).expect("invalid string");
    let mut output = Vec::with_capacity(2048);

    unsafe {
        let len = UnDecorateSymbolName(
            cstr.as_ptr(),
            output.as_mut_ptr(),
            output.capacity() as u32,
            0x0800 | 0x1000 | 0x2000 | 0x0002 | 0x0001,
        );

        output.set_len(len as usize);
    };

    if output.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&output).to_string())
    }
}
