use me3_framework::deref;

pub use self::file::ParamFileDescriptor;

pub mod file;

#[repr(transparent)]
pub struct ParamRepository(*mut usize);

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct ParamFileHeader {
    #[doc(hidden)]
    _pad1: [u8; 0xA],
    pub size: i16,
    #[doc(hidden)]
    _pad2: [u8; 0x34],
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct ParamFileEntry {
    pub id: i32,

    #[doc(hidden)]
    _pad1: [u8; 4],

    pub offset: i32,

    #[doc(hidden)]
    _pad2: [u8; 0xc],
}

#[derive(Copy, Clone, Debug)]
pub struct ParamFileData<'p> {
    header: &'p ParamFileHeader,
    entries: &'p [ParamFileEntry],
}

impl ParamRepository {
    fn get_file_data(&self, id: i32) -> Option<ParamFileData<'_>> {
        let param_repo_instance = self.0;

        unsafe {
            let header =
                deref!([[[param_repo_instance + 0x48 * id as usize + 0x70] + 0x68] + 0x68])?
                    .cast::<ParamFileHeader>()
                    .as_ptr();

            let entry_pointer = header.add(1) as *const ParamFileEntry;
            let entries = std::slice::from_raw_parts(entry_pointer, (*header).size as usize);

            Some(ParamFileData {
                header: header.as_ref()?,
                entries,
            })
        }
    }

    fn get_row_pointer(&self, file: i32, id: i32) -> Option<*const ()> {
        let file = self.get_file_data(file)?;
        let entry_offset = file
            .entries
            .binary_search_by_key(&id, |entry| entry.id)
            .ok()?;

        let entry = &file.entries[entry_offset];
        let data = file.header as *const ParamFileHeader;

        unsafe { Some(data.byte_offset(entry.offset as isize) as *const _) }
    }

    #[allow(dead_code)]
    pub fn get_row<T>(&self, id: i32) -> Option<&T::Row>
    where
        T: ParamFileDescriptor,
    {
        let row_ptr = self.get_row_pointer(T::ID as i32, id)?;

        unsafe { row_ptr.cast::<T::Row>().as_ref() }
    }

    // TODO: this could cooperate with the games synchronization primitives.
    #[allow(dead_code)]
    pub fn get_row_mut<T>(&self, id: i32) -> Option<&mut T::Row>
    where
        T: ParamFileDescriptor,
    {
        let row_ptr = self.get_row_pointer(T::ID as i32, id)?;

        unsafe { row_ptr.cast::<T::Row>().cast_mut().as_mut() }
    }
}
