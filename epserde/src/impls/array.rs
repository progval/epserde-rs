/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::mem::MaybeUninit;

use crate::{CopyType, DeserializeInner, Eps, EpsCopy, SliceWithPos, Zero, ZeroCopy};

use crate::des::*;

impl<T: CopyType, const N: usize> CopyType for [T; N] {
    type Copy = T::Copy;
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + DeserializeInner + 'static, const N: usize> DeserializeInner for [T; N]
where
    [T; N]: DeserializeHelper<<T as CopyType>::Copy, FullType = [T; N]>,
{
    type DeserType<'a> = <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> Result<([T; N], R)> {
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_full_copy_inner_impl(
            backend,
        )
    }

    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> Result<(
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'_>,
        SliceWithPos,
    )> {
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_eps_copy_inner_impl(
            backend,
        )
    }
}

impl<T: ZeroCopy + DeserializeInner + 'static, const N: usize> DeserializeHelper<Zero> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = &'a [T; N];
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(mut backend: R) -> Result<(Self, R)> {
        backend = backend.align::<T>()?;
        let mut res = MaybeUninit::<[T; N]>::uninit();
        // SAFETY: read_exact guarantees that the array will be filled with data.
        unsafe {
            backend.read_exact(res.assume_init_mut().align_to_mut::<u8>().1)?;
            Ok((res.assume_init(), backend))
        }
    }
    #[inline(always)]

    fn _deserialize_eps_copy_inner_impl(
        mut backend: SliceWithPos,
    ) -> Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        let bytes = std::mem::size_of::<[T; N]>();
        backend = backend.align::<T>()?;
        let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<[T; N]>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        Ok((&data[0], backend.skip(bytes)))
    }
}

impl<T: EpsCopy + DeserializeInner + 'static, const N: usize> DeserializeHelper<Eps> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = [<T as DeserializeInner>::DeserType<'a>; N];
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(mut backend: R) -> Result<(Self, R)> {
        backend = backend.align::<T>()?;
        let mut res = MaybeUninit::<[T; N]>::uninit();
        unsafe {
            for item in &mut res.assume_init_mut().iter_mut() {
                let (elem, new_backend) = T::_deserialize_full_copy_inner(backend)?;
                std::ptr::write(item, elem);
                backend = new_backend;
            }
            Ok((res.assume_init(), backend))
        }
    }
    #[inline(always)]
    fn _deserialize_eps_copy_inner_impl(
        mut backend: SliceWithPos,
    ) -> Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        backend = backend.align::<T>()?;
        let mut res = MaybeUninit::<<Self as DeserializeInner>::DeserType<'_>>::uninit();
        unsafe {
            for item in &mut res.assume_init_mut().iter_mut() {
                let (elem, new_backend) = T::_deserialize_eps_copy_inner(backend)?;
                std::ptr::write(item, elem);
                backend = new_backend;
            }
            Ok((res.assume_init(), backend))
        }
    }
}