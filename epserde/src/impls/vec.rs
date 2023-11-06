/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for vectors.

*/
use crate::deser;
use crate::deser::helpers::*;
use crate::deser::*;
use crate::ser;
use crate::ser::helpers::*;
use crate::ser::*;
use crate::traits::*;
use core::hash::Hash;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

impl<T> CopyType for Vec<T> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for Vec<T> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Vec".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T: ReprHash> ReprHash for Vec<T> {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        *offset_of = 0;
        T::repr_hash(hasher, offset_of);
    }
}

impl<T: CopyType + SerializeInner + TypeHash> SerializeInner for Vec<T>
where
    Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Vec<T> {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        serialize_slice_zero(backend, self.as_slice())
    }
}

impl<T: DeepCopy + SerializeInner> SerializeHelper<Deep> for Vec<T> {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        serialize_slice_deep(backend, self.as_slice())
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + DeserializeInner + 'static> DeserializeInner for Vec<T>
where
    Vec<T>: DeserializeHelper<<T as CopyType>::Copy, FullType = Vec<T>>,
{
    type DeserType<'a> = <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_full_inner_impl(backend)
    }

    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>> {
        <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_eps_inner_impl(backend)
    }
}

impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero> for Vec<T> {
    type FullType = Self;
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        deserialize_full_vec_zero(backend)
    }
    #[inline(always)]
    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        deserialize_eps_slice_zero(backend)
    }
}

impl<T: DeepCopy + DeserializeInner + 'static> DeserializeHelper<Deep> for Vec<T> {
    type FullType = Self;
    type DeserType<'a> = Vec<<T as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        deserialize_full_vec_deep::<T>(backend)
    }
    #[inline(always)]
    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        deserialize_eps_vec_deep::<T>(backend)
    }
}
