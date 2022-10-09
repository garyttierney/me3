use faithe::RuntimeOffset;
pub enum FunctionAddress {
    Offset(RuntimeOffset),
    Pointer(*const ()),
}

pub trait FunctionRef {
    type Target;

    fn set_target(&self, new_target: *const ());
    fn get_target(&self) -> Self::Target;
    fn get_ptr(&self) -> *const ();
}

#[macro_export]
macro_rules! function {
    (
        $(
            $vs:vis $name:ident: $(extern $($cc:literal)?)? fn($($arg_id:ident: $arg_ty:ty),*) $(-> $ret_ty:ty)? = $lib_name:tt$sep:tt$var:tt$([$add:tt])?;
        )*
    ) => {
        $(
            #[allow(non_upper_case_globals)]
            $vs static mut $name: $name = $name {
                offset: std::cell::RefCell::new($crate::framework::hooks::FunctionAddress::Offset($crate::faithe::__define_offset!($sep $var)))
            };
            #[allow(non_camel_case_types)]
            $vs struct $name {
                offset: std::cell::RefCell<$crate::framework::hooks::FunctionAddress>,
            }
            unsafe impl ::core::marker::Sync for $name { }

            impl $crate::framework::hooks::FunctionRef for $name {
                type Target = $(extern $($cc)?)? fn($($arg_ty),*) $(-> $ret_ty)?;

                fn get_ptr(&self) -> *const () {
                    match &*self.offset.borrow() {
                        $crate::framework::hooks::FunctionAddress::Offset(offset) => {
                            if !offset.is_resolved() {
                                $crate::faithe::__expect!(offset.try_resolve($lib_name, $crate::faithe::__define_offset2!($($add)?)), "Failed to resolve function's address");
                            }

                            offset.address() as *const ()
                        },
                        $crate::framework::hooks::FunctionAddress::Pointer(ptr) => *ptr
                    }
                }

                fn get_target(&self) -> Self::Target {
                    let address = self.get_ptr();

                    unsafe { ::core::mem::transmute::<_, $(extern $($cc)?)? fn($($arg_ty),*) $(-> $ret_ty)?>(address) }
                }

                fn set_target(&self, new_fn: *const ()) {
                    let mut current = self.offset.borrow_mut();
                    *current = $crate::framework::hooks::FunctionAddress::Pointer(new_fn);
                }
            }

            impl $name {
                #[inline]
                $vs fn call(&self, $($arg_id:$arg_ty),*) $(-> $ret_ty)? {
                    use $crate::framework::hooks::FunctionRef;
                    (self.get_target())($($arg_id),*)
                }
            }
        )*
    };
}
