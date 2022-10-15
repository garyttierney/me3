use std::{
    io::Seek,
    io::{SeekFrom, Write},
    marker::PhantomData,
    mem::replace,
    num::TryFromIntError,
};

use byteorder::{ByteOrder, WriteBytesExt};

pub enum UnresolvedStatus {
    Offset(u64),
    Resolved,
}

pub struct Unresolved<T: Resolvable, E: ByteOrder> {
    inner: UnresolvedStatus,
    _value_ty: PhantomData<T>,
    _byte_order_ty: PhantomData<E>,
}

pub trait Resolvable: Sized + Copy {
    fn write_to<W: Write, O: ByteOrder>(self, output: W) -> std::io::Result<()>;
}

impl Resolvable for u32 {
    fn write_to<W: Write, O: ByteOrder>(self, mut output: W) -> std::io::Result<()> {
        output.write_u32::<O>(self)
    }
}

impl<T: Resolvable, E: ByteOrder> Unresolved<T, E> {
    pub fn resolve<W: Write + Seek>(
        &mut self,
        mut writer: W,
        value: T,
    ) -> Result<T, std::io::Error> {
        let saved_pos = match replace(&mut self.inner, UnresolvedStatus::Resolved) {
            UnresolvedStatus::Offset(pos) => pos,
            UnresolvedStatus::Resolved => panic!("already resolved"),
        };

        let pos = writer.stream_position()?;
        writer.seek(SeekFrom::Start(saved_pos))?;
        value.write_to::<_, E>(&mut writer)?;
        writer.seek(SeekFrom::Start(pos))?;

        Ok(value)
    }
}

impl<T, E> Unresolved<T, E>
where
    T: Resolvable + TryFrom<u64, Error = TryFromIntError>,
    E: ByteOrder,
{
    pub fn resolve_with_position<W: Write + Seek>(
        &mut self,
        mut writer: W,
    ) -> Result<T, std::io::Error> {
        let value = T::try_from(writer.stream_position()?).unwrap();

        self.resolve(writer, value)
    }

    pub fn resolve_with_relative_offset<W: Write + Seek>(
        &mut self,
        mut writer: W,
        pos: u64,
    ) -> Result<T, std::io::Error> {
        let offset = writer.stream_position()? - pos;
        let value = T::try_from(offset).unwrap();

        self.resolve(writer, value)
    }
}

impl<T, E> Drop for Unresolved<T, E>
where
    T: Resolvable,
    E: ByteOrder,
{
    fn drop(&mut self) {
        if let UnresolvedStatus::Offset(pos) = self.inner {
            panic!(
                "unresolved {} at 0x{:x} dropped before resolving",
                std::any::type_name::<T>(),
                pos
            );
        }
    }
}

pub trait WriteFormatsExt {
    fn write_unresolved<T: Resolvable, E: ByteOrder>(
        &mut self,
    ) -> Result<Unresolved<T, E>, std::io::Error>;
    fn write_unresolved_u32<E: ByteOrder>(&mut self) -> Result<Unresolved<u32, E>, std::io::Error>;
}

impl<W: Write + Seek> WriteFormatsExt for W {
    fn write_unresolved<T: Resolvable, E: ByteOrder>(
        &mut self,
    ) -> Result<Unresolved<T, E>, std::io::Error> {
        let offset = self.stream_position()?;
        let unresolved_value_size = std::mem::size_of::<T>();

        self.seek(SeekFrom::Current(unresolved_value_size as i64))?;

        Ok(Unresolved {
            _value_ty: PhantomData::default(),
            _byte_order_ty: PhantomData::default(),
            inner: UnresolvedStatus::Offset(offset),
        })
    }

    fn write_unresolved_u32<E: ByteOrder>(&mut self) -> Result<Unresolved<u32, E>, std::io::Error> {
        self.write_unresolved::<u32, E>()
    }
}
