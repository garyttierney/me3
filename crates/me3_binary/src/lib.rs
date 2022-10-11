use pelite::{
    pe::PeObject,
    pe64::{Pe, PeFile, PeView},
};

pub mod analysis;

#[derive(Copy, Clone)]
pub enum Program<'a> {
    File(PeFile<'a>),
    Mapping(PeView<'a>),
}

unsafe impl<'a> Pe<'a> for Program<'a> {}
unsafe impl<'a> PeObject<'a> for Program<'a> {
    fn image(&self) -> &'a [u8] {
        match self {
            Self::File(file) => file.image(),
            Self::Mapping(mapping) => mapping.image(),
        }
    }

    fn align(&self) -> pelite::Align {
        match self {
            Self::File(file) => file.align(),
            Self::Mapping(mapping) => mapping.align(),
        }
    }
}

impl<'a> Program<'a> {
    /// # Safety
    ///
    /// This must only be called from the context of a valid PE program.
    /// Attempting to call this on a program with malformed or incorrect PE headers
    /// is undefined behaviour.
    pub unsafe fn current() -> Self {
        Self::Mapping(PeView::new())
    }
}
