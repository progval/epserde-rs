/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone, Copy)]
#[repr(C)]
#[zero_copy]
struct Data {
    a: usize,
}

fn main() {
    // Create a vector to serialize
    let a = vec![Data { a: 5 }];
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let _bytes_written = a.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <Vec<Data>>::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Vec<Data>>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <Vec<Data>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Vec<Data> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
