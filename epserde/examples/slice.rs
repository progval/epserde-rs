/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showcasing the cheaty serialization of a slice.
use epserde::prelude::*;

fn main() {
    let a = vec![0, 1, 2, 3];
    // Turn it into a slice
    let a: &[i32] = a.as_ref();
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    // Serialize the slice using the cheaty implementation
    let _bytes_written = a.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization as a vector
    cursor.set_position(0);
    let full = <Vec<i32>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Vec<i32>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization as, again, a slice
    let buf = cursor.into_inner();
    let eps = <Vec<i32>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Vec<i32> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
