use abi_stable::StableAbi;

pub use self::file::ParamFileDescriptor;

pub mod file;

#[repr(transparent)]
#[derive(StableAbi)]
pub struct ParamRepository(*mut usize);

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct ParamFileHeader {
    #[doc(hidden)]
    _pad1: [u8; 0xA],
    size: i16,
    #[doc(hidden)]
    _pad2: [u8; 0x34],
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct ParamFileEntry {
    id: i32,

    #[doc(hidden)]
    _pad1: [u8; 4],

    offset: i32,

    #[doc(hidden)]
    _pad2: [u8; 0xc],
}

impl ParamFileEntry {
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    #[inline]
    pub fn offset(&self) -> i32 {
        self.offset
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ParamFileData<'p> {
    header: &'p ParamFileHeader,
    entries: &'p [ParamFileEntry],
}

impl ParamRepository {
    fn get_file_data<'p>(&'p self, id: i32) -> Option<ParamFileData<'p>> {
        let param_repo_instance = unsafe { *self.0 as *const usize };

        unsafe {
            let file = *param_repo_instance.byte_offset(0x48 * id as isize + 0x70) as *const usize;
            let file_data = *file.byte_offset(0x68) as *const usize;

            let header = *file_data.byte_offset(0x68) as *const ParamFileHeader;
            let entry_pointer = header.add(1) as *const ParamFileEntry;
            let entries = std::slice::from_raw_parts(entry_pointer, (*header).size as usize);

            Some(ParamFileData {
                header: header.as_ref()?,
                entries,
            })
        }
    }

    fn get_row_pointer<T: ParamFileDescriptor>(&self, id: i32) -> Option<*const T::Row> {
        let file = self.get_file_data(T::ID as i32)?;
        let entry_offset = file
            .entries
            .binary_search_by_key(&id, |entry| entry.id)
            .ok()?;

        let entry = &file.entries[entry_offset];
        let data = file.header as *const ParamFileHeader;

        unsafe { Some(data.byte_offset(entry.offset as isize).cast::<T::Row>()) }
    }

    #[allow(dead_code)]
    fn get_row<T>(&self, id: i32) -> Option<&T::Row>
    where
        T: ParamFileDescriptor,
    {
        let row_ptr = self.get_row_pointer::<T>(id)?;

        unsafe { row_ptr.as_ref() }
    }

    #[allow(dead_code)]
    fn get_row_mut<T>(&mut self, id: i32) -> Option<&mut T::Row>
    where
        T: ParamFileDescriptor,
    {
        let row_ptr = self.get_row_pointer::<T>(id)?;

        unsafe { row_ptr.cast_mut().as_mut() }
    }
}
