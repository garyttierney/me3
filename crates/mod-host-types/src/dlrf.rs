//! DLRF runtime reflection system.
//!
//! Credit for the bulk of this reverse engineering work goes to [Axi](https://github.com/axd1x8a).

use std::{fmt, mem::MaybeUninit, ops::Deref};

use crate::{
    alloc::DlAllocator,
    ffi::{ThinCStr, ThinWCStr},
    vector::DlVector,
};

/// Unique type identifier for types registered into DLRF.
///
/// These are generated using template metaprogramming, and actually represent the address
/// of unique static `u8` with a value of 0, instantiated per type. Hence a valid [`DLTypeId`]
/// is inherently non-zero.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DLTypeId(pub usize);

impl DLTypeId {
    /// A [`DLTypeId`] that does not represent an actual type.
    pub const NONE: Self = Self(0);

    /// Whether this [`DLTypeId`] represents an actual type.
    pub const fn is_none(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug)]
pub struct DLRuntimeClassVtable<T> {
    pub dtor: unsafe extern "C" fn(*mut T),
    pub name: extern "C" fn(&T) -> ThinCStr,
    pub name_wide: extern "C" fn(&T) -> ThinWCStr,
    pub type_id: extern "C" fn(&T) -> DLTypeId,
    pub const_type_id: extern "C" fn(&T) -> DLTypeId,
    pub ptr_type_id: extern "C" fn(&T) -> DLTypeId,
    pub const_ptr_type_id: extern "C" fn(&T) -> DLTypeId,
    pub is_primitive_type: extern "C" fn(&T) -> bool,
    pub destruct: unsafe extern "C" fn(&T, instance: *mut (), allocator: *mut DlAllocator),
    pub flags: extern "C" fn(&T) -> u32,
    pub register_runtime_ctor:
        unsafe extern "C" fn(&mut T, invoker: *const (), name: ThinCStr, name_wide: ThinWCStr),
    pub register_method_invoker: unsafe extern "C" fn(
        &mut T,
        invoker: *const DLConcreteMethodInvoker,
        name: ThinCStr,
        name_wide: ThinWCStr,
    ),
}

#[repr(C)]
pub struct DLRuntimeClassImpl<'a> {
    vtable: &'a DLRuntimeClassVtable<Self>,
    pub parent: Option<&'a Self>,
    pub runtime_ctor: Option<&'a DLRuntimeMethod<'a>>,
    pub methods: DlVector<DLRuntimeClassMethodInvoker<'a>>,
    pub name: ThinCStr<'a>,
    pub name_wide: ThinWCStr<'a>,
}

impl<'a> DLRuntimeClassImpl<'a> {
    pub fn vtable(&self) -> &'a DLRuntimeClassVtable<Self> {
        self.vtable
    }

    pub fn type_id(&self) -> DLTypeId {
        (self.vtable.type_id)(self)
    }

    pub fn const_type_id(&self) -> DLTypeId {
        (self.vtable.const_type_id)(self)
    }

    pub fn ptr_type_id(&self) -> DLTypeId {
        (self.vtable.ptr_type_id)(self)
    }

    pub fn const_ptr_type_id(&self) -> DLTypeId {
        (self.vtable.const_ptr_type_id)(self)
    }

    pub fn is_primitive_type(&self) -> bool {
        (self.vtable.is_primitive_type)(self)
    }
}

impl fmt::Debug for DLRuntimeClassImpl<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DLRuntimeClassImpl")
            .field("vmt", &(self.vtable as *const _))
            .field("parent", &self.parent)
            .field("runtime_ctor", &self.runtime_ctor)
            .field("methods", &self.methods)
            .field("name", &self.name)
            .field("name_wide", &self.name_wide)
            .finish()
    }
}

#[repr(C)]
pub struct DLRuntimeMethod<'a> {
    pub class: &'a DLRuntimeClassImpl<'a>,
    pub name: Option<ThinCStr<'a>>,
    pub name_wide: Option<ThinWCStr<'a>>,
    dl_mutex: [u8; 0x30],
    pub addr: *const (),
}

impl fmt::Debug for DLRuntimeMethod<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DLRuntimeMethod")
            .field("class", &(self.class as *const _))
            .field("name", &self.name)
            .field("name_wide", &self.name_wide)
            .field("addr", &self.addr)
            .finish()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct DLRuntimeClassMethodInvoker<'a> {
    pub resolver: &'a DLMethodResolver<'a>,
    pub name: ThinCStr<'a>,
    pub name_wide: ThinWCStr<'a>,
    pub name_len: usize,
}

#[repr(C)]
pub struct DLMethodResolver<'a> {
    pub class: &'a DLRuntimeClassImpl<'a>,
    pub name: ThinCStr<'a>,
    pub name_wide: ThinWCStr<'a>,
    pub invokers: DlVector<&'a DLConcreteMethodInvoker<'a>>,
    // fields below are no longer populated.
    _param_infos: DlVector<DLParameterInfo<'a>>,
    _loose_param_info_start: Option<&'a DLParameterInfo<'a>>,
}

impl fmt::Debug for DLMethodResolver<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DLMethodResolver")
            .field("class", &(self.class as *const _))
            .field("name", &self.name)
            .field("name_wide", &self.name_wide)
            .field("invokers", &self.invokers)
            // .field("param_infos", &self._param_infos)
            // .field("loose_param_info_start", &self._loose_param_info_start)
            .finish()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct DLMethodInvokerVtable<T> {
    pub invoke: *const (), // signature not reversed yet.
    pub dtor: unsafe extern "C" fn(*mut T),
    pub param_count: extern "C" fn(&T) -> u32,
    pub strict_param_info: for<'a> extern "C" fn(&'a T, out: &mut MaybeUninit<DLParameterInfo<'a>>),
    pub loose_param_info: for<'a> extern "C" fn(&'a T, out: &mut MaybeUninit<DLParameterInfo<'a>>),
    pub return_type: extern "C" fn(&T) -> DLTypeId,
}

#[repr(C)]
pub struct DLConcreteMethodInvoker<'a> {
    vtable: &'a DLMethodInvokerVtable<Self>,
    pub addr: *const (),
}

impl<'a> DLConcreteMethodInvoker<'a> {
    pub fn vtable(&self) -> &'a DLMethodInvokerVtable<Self> {
        self.vtable
    }

    pub fn param_count(&self) -> u32 {
        (self.vtable.param_count)(self)
    }

    /// Get all mandatory parameters to this function.
    pub fn strict_param_info(&self) -> DLParameterInfo<'_> {
        let mut param_info = MaybeUninit::uninit();
        (self.vtable.strict_param_info)(self, &mut param_info);
        unsafe { param_info.assume_init() }
    }

    /// Get all parameters to this function, including optional ones.
    pub fn loose_param_info(&self) -> DLParameterInfo<'_> {
        let mut param_info = MaybeUninit::uninit();
        (self.vtable.loose_param_info)(self, &mut param_info);
        unsafe { param_info.assume_init() }
    }

    pub fn return_type(&self) -> DLTypeId {
        (self.vtable.return_type)(self)
    }
}

impl fmt::Debug for DLConcreteMethodInvoker<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DLConcreteMethodInvoker")
            .field(&self.addr)
            .finish()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct DLParameterInfo<'a> {
    params: [DLTypeId; 15],
    pub method_invoker: &'a DLConcreteMethodInvoker<'a>,
}

impl DLParameterInfo<'_> {
    pub fn params(&self) -> &[DLTypeId] {
        &self.params[..self.method_invoker.param_count() as usize]
    }
}

impl Deref for DLParameterInfo<'_> {
    type Target = [DLTypeId];

    fn deref(&self) -> &Self::Target {
        self.params()
    }
}

/// Entry in the runtime class registry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RuntimeClassEntry<'a> {
    /// The type ID of this class. May also be the type-ID for the const class, or a pointer to the
    /// class.
    pub type_id: DLTypeId,
    /// The associated runtime class information.
    pub class: &'a DLRuntimeClassImpl<'a>,
}

impl<'a> RuntimeClassEntry<'a> {
    /// Construct a slice of [`RuntimeClassEntry`] from analysis output.
    ///
    /// # Safety
    ///
    /// - `registry` must be sourced from [`me3_binary_analysis::dlrf::runtime_classes`].
    /// - The program passed to the above function must be a view over a real game executable.
    pub unsafe fn from_analysis(
        registry: &'a [me3_binary_analysis::dlrf::RuntimeClassEntry],
    ) -> &'a [Self] {
        unsafe { std::mem::transmute(registry) }
    }
}

impl PartialEq for RuntimeClassEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for RuntimeClassEntry<'_> {}

impl PartialOrd for RuntimeClassEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RuntimeClassEntry<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.type_id.cmp(&other.type_id)
    }
}
