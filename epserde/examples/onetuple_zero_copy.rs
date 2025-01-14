/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*
 * This example shows how the standard behavior of ε-serde on primitive
 * types (returning a value rather than a reference) is somewhat custom:
 * the deserialization type associated to a one-element tuple containing
 * just a `usize` is a reference.
 */
use epserde::prelude::*;

fn main() {
    // Create a new value to serialize
    let x = (0_usize,);
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let _bytes_written = x.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <(usize,)>::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<(usize,)>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(x, full);

    println!();

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <(usize,)>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<(usize,) as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(x, *eps);
}
