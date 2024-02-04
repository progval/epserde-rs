/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use anyhow::Result;
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data<A: PartialEq = usize, const Q: usize = 3> {
    a: A,
    b: [i32; Q],
}

#[test]
fn test_cheaty_serialize() -> Result<()> {
    let a = vec![1, 2, 3, 4];
    let s = a.as_slice();
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    s.serialize(&mut cursor)?;
    cursor.set_position(0);
    let b = <Vec<i32>>::deserialize_full(&mut cursor)?;
    assert_eq!(a, b.as_slice());
    let backend = cursor.into_inner();
    let b = <Vec<i32>>::deserialize_eps(&backend)?;
    assert_eq!(a, b);
    Ok(())
}
