/// A type that represents a reference to a PARAM file within a FROMSOFTWARE game.
///
/// ```
/// #[derive(Debug)]
/// #[repr(C)]
/// pub struct NetworkAreaParam {
///     /// セルサイズX
///     cell_size_x: f32,
///     /// セルサイズY
///     cell_size_y: f32,
///     /// セルサイズZ
///     cell_size_z: f32,
///     /// セルオフセットX
///     cell_offset_x: f32,
///     /// セルオフセットY
///     cell_offset_y: f32,
///    /// セルオフセットZ
///     cell_offset_z: f32,
/// }
///
/// impl me3_game_support_fromsoft::sprj::ParamFileDescriptor for NetworkAreaParam {
///     const ID: usize = 49;
///     type Row = Self;
/// }
/// ```
pub trait ParamFileDescriptor {
    const ID: usize;
    type Row: Sized;
}

/// A helper macro to simplify defining PARAM files for structures that are only referenced within a single PARAM file.
/// ```
/// use me3_game_support_fromsoft::impl_param_file_descriptor;
///
/// pub struct NetworkAreaParam {
///     /// セルサイズX
///     cell_size_x: f32,
///     /// セルサイズY
///     cell_size_y: f32,
///     /// セルサイズZ
///     cell_size_z: f32,
///     /// セルオフセットX
///     cell_offset_x: f32,
///     /// セルオフセットY
///     cell_offset_y: f32,
///     /// セルオフセットZ
///     cell_offset_z: f32,
/// }
///
/// impl_param_file_descriptor!(NetworkAreaParam, 49);
/// ```
#[macro_export]
macro_rules! impl_param_file_descriptor {
    ($ty:ident, $id:expr) => {
        impl me3_game_support_fromsoft::sprj::ParamFileDescriptor for $ty {
            const ID: usize = $id;
            type Row = $ty;
        }
    };
}
