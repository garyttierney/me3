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
                offset: std::cell::RefCell::new($crate::FunctionAddress::Offset($crate::faithe::__define_offset!($sep $var)))
            };
            #[allow(non_camel_case_types)]
            $vs struct $name {
                offset: std::cell::RefCell<$crate::FunctionAddress>,
            }
            unsafe impl ::core::marker::Sync for $name { }

            impl crate::FunctionRef for $name {
                type Target = $(extern $($cc)?)? fn($($arg_ty),*) $(-> $ret_ty)?;

                fn get_ptr(&self) -> *const () {
                    match &*self.offset.borrow() {
                        $crate::FunctionAddress::Offset(offset) => {
                            if !offset.is_resolved() {
                                $crate::faithe::__expect!(offset.try_resolve($lib_name, $crate::faithe::__define_offset2!($($add)?)), "Failed to resolve function's address");
                            }

                            offset.address() as *const ()
                        },
                        $crate::FunctionAddress::Pointer(ptr) => *ptr
                    }
                }

                fn get_target(&self) -> Self::Target {
                    let address = self.get_ptr();

                    unsafe { ::core::mem::transmute::<_, $(extern $($cc)?)? fn($($arg_ty),*) $(-> $ret_ty)?>(address) }
                }

                fn set_target(&self, new_fn: *const ()) {
                    let mut current = self.offset.borrow_mut();
                    *current = $crate::FunctionAddress::Pointer(new_fn);
                }
            }

            impl $name {
                #[inline]
                $vs fn call(&self, $($arg_id:$arg_ty),*) $(-> $ret_ty)? {
                    use $crate::FunctionRef;
                    (self.get_target())($($arg_id),*)
                }
            }
        )*
    };
}
