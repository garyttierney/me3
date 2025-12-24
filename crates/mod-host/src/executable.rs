use std::{borrow::Cow, cell::UnsafeCell, fmt, slice};

use pelite::{
    pe::{Pe, PeObject, PeView},
    resources::FindError,
};
use windows::{core::PCWSTR, Win32::System::LibraryLoader::GetModuleHandleW};

#[derive(Clone, Copy)]
pub struct Executable {
    inner: &'static UnsafeCell<[u8]>,
}

#[derive(Clone, Debug)]
pub struct Version {
    pub product: Cow<'static, str>,
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub build: u16,
    pub region: Region,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Region {
    Worldwide,
    Japan,
}

impl Executable {
    /// # Safety
    ///
    /// Module must be accessed exclusively for the duration of the call
    /// (typically only possible in a suspended process).
    ///
    /// # Panics
    ///
    /// if `GetModuleHandleW(NULL)` fails.
    pub unsafe fn new() -> Self {
        unsafe {
            let view = PeView::module(GetModuleHandleW(PCWSTR::null()).unwrap().0 as _);
            let image = view.image();

            Self {
                inner: UnsafeCell::from_mut(slice::from_raw_parts_mut(
                    image.as_ptr() as _,
                    image.len(),
                )),
            }
        }
    }

    pub fn version(&self) -> Result<Version, FindError> {
        let version_info = self.resources()?.version_info()?;
        let product_version = version_info
            .fixed()
            .ok_or(FindError::NotFound)?
            .dwProductVersion;

        let language = version_info.translation().first();

        let product = language.and_then(|language| {
            let mut product = None;
            version_info.strings(*language, |key, value| {
                (key == "ProductName").then(|| product = Some(Cow::Owned(value.to_owned())));
            });
            product
        });

        let product = product.unwrap_or(Cow::Borrowed("Unknown Product"));

        // Use FROMSOFTWARE version mapping scheme:
        let major = product_version.Major.min(1);
        let minor = product_version.Minor + (product_version.Major - major) * 10;

        let region = match language {
            Some(language) if language.lang_id == 0x411 => Region::Japan,
            _ => Region::Worldwide,
        };

        Ok(Version {
            product,
            major,
            minor,
            patch: product_version.Patch,
            build: product_version.Build,
            region,
        })
    }
}

unsafe impl PeObject<'static> for Executable {
    fn image(&self) -> &'static [u8] {
        unsafe { &*self.inner.get() }
    }

    fn align(&self) -> pelite::Align {
        pelite::Align::Section
    }
}

unsafe impl Pe<'static> for Executable {}

impl fmt::Debug for Executable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let image = self.image();
        f.debug_struct("Executable")
            .field("image_base", &image.as_ptr())
            .field("size", &image.len())
            .finish()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}.{:02}.{}.{} {}",
            self.product, self.major, self.minor, self.patch, self.build, self.region
        )
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

unsafe impl Send for Executable {}

unsafe impl Sync for Executable {}
