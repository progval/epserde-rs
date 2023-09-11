/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::marker::PhantomData;

use crate::des;
use crate::ser;
use crate::ser::FieldWrite;
use crate::CopyType;
use crate::DeserializeError;
use crate::DeserializeInner;
use crate::Eps;
use crate::ReadWithPos;
use crate::SerializeInner;
use crate::SliceWithPos;
use crate::Zero;

macro_rules! impl_prim{
    ($($ty:ty),*) => {$(
        impl CopyType for $ty {
            type Copy = Zero;
        }

		impl SerializeInner for $ty {
            // here we are lying :)
            // primitive types are zero copy, but we won't return
            // a reference to them
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> ser::Result<F> {
                backend.write(&self.to_ne_bytes())?;
                Ok(backend)
            }
        }

		impl DeserializeInner for $ty {
            #[inline(always)]
            fn _deserialize_full_copy_inner<R: ReadWithPos>(mut backend: R) -> des::Result<(Self, R)> {
                backend = backend.pad_align_and_check::<$ty>()?;
                let mut buf = [0; core::mem::size_of::<$ty>()];
                backend.read_exact(&mut buf)?;
                Ok((
                    <$ty>::from_ne_bytes(buf),
                    backend
                ))
            }
            type DeserType<'a> = $ty;
            #[inline(always)]
            fn _deserialize_eps_copy_inner(
                mut backend: SliceWithPos,
            ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
                backend = backend.pad_align_and_check::<$ty>()?;
                Ok((
                    <$ty>::from_ne_bytes(
                        backend.data[..core::mem::size_of::<$ty>()]
                            .try_into()
                            .unwrap(),
                    ),
                    backend.skip(core::mem::size_of::<$ty>()),
                ))
            }
        }
    )*};
}

impl_prim!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

// Booleans are zero-copy serialized as u8.

impl CopyType for bool {
    type Copy = Zero;
}

impl SerializeInner for bool {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> ser::Result<F> {
        let val = if *self { 1 } else { 0 };
        backend.write(&[val])?;
        Ok(backend)
    }
}

impl DeserializeInner for bool {
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        u8::_deserialize_full_copy_inner(backend).map(|(x, b)| (x != 0, b))
    }
    type DeserType<'a> = Self;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
        Ok((backend.data[0] != 0, backend.skip(1)))
    }
}

// Chars are zero-copy serialized as u32.

impl CopyType for char {
    type Copy = Zero;
}

impl SerializeInner for char {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        (*self as u32)._serialize_inner(backend)
    }
}

impl DeserializeInner for char {
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        u32::_deserialize_full_copy_inner(backend).map(|(x, c)| (char::from_u32(x).unwrap(), c))
    }
    type DeserType<'a> = Self;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
        u32::_deserialize_eps_copy_inner(backend).map(|(x, c)| (char::from_u32(x).unwrap(), c))
    }
}

// PhantomData is zero-copy. No reading or writing is performed when (de)serializing it.

impl<T> CopyType for PhantomData<T> {
    type Copy = Zero;
}

impl<T: SerializeInner> SerializeInner for PhantomData<T> {
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        Ok(backend)
    }
}

impl<T: DeserializeInner> DeserializeInner for PhantomData<T> {
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        Ok((PhantomData::<T>, backend))
    }
    type DeserType<'a> = PhantomData<<T as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
        Ok((
            PhantomData::<<T as DeserializeInner>::DeserType<'_>>,
            backend,
        ))
    }
}

// () is zero-copy. No reading or writing is performed when (de)serializing it.

impl CopyType for () {
    type Copy = Zero;
}

impl SerializeInner for () {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        Ok(backend)
    }
}

impl DeserializeInner for () {
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        Ok(((), backend))
    }
    type DeserType<'a> = Self;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
        Ok(((), backend))
    }
}

// Options are ε-copy types serialized as a one-byte tag (0 for None, 1 for Some) followed, in case, by the value.

impl<T> CopyType for Option<T> {
    type Copy = Eps;
}

impl<T: SerializeInner> SerializeInner for Option<T> {
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> ser::Result<F> {
        match self {
            None => {
                backend = backend.add_field_align("Tag", &0_u8)?;
            }
            Some(val) => {
                backend = backend.add_field_align("Tag", &1_u8)?;
                backend = backend.add_field_align("Some", val)?;
            }
        };
        Ok(backend)
    }
}

impl<T: DeserializeInner> DeserializeInner for Option<T> {
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        let (tag, backend) = u8::_deserialize_full_copy_inner(backend)?;
        match tag {
            0 => Ok((None, backend)),
            1 => {
                let (elem, backend) = T::_deserialize_full_copy_inner(backend)?;
                Ok((Some(elem), backend))
            }
            _ => Err(DeserializeError::InvalidTag(tag)),
        }
    }
    type DeserType<'a> = Option<<T as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
        match backend.data[0] {
            0 => Ok((None, backend.skip(1))),
            1 => {
                let (value, backend) = T::_deserialize_eps_copy_inner(backend.skip(1))?;
                Ok((Some(value), backend))
            }
            _ => Err(DeserializeError::InvalidTag(backend.data[0])),
        }
    }
}