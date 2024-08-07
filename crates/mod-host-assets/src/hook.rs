use crate::ffi::DLWString;

#[repr(C)]
pub struct RSResourceFileRequest {
    pub vfptr: usize,
    _unk8: [u8; 0x48],
    pub resource_path: DLWString,
}
