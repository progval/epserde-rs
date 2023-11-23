/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showcasing the cheaty serialization of a slice.
use epserde::prelude::*;

fn main() {
    // Create a vector to serialize

    let a = vec![0, 1, 2, 3];

    let a: &[i32] = a.as_ref();

    let mut buf = epserde::new_aligned_cursor();
    // Serialize the slice using the cheaty implementation
    let _bytes_written = a.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization as a vector
    buf.set_position(0);
    let full = <Vec<i32>>::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Vec<i32>>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do an ε-copy deserialization as, again, a slice
    let buf = buf.into_inner();
    let eps = <Vec<i32>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Vec<i32> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}