/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use super::*;

/// The type of result returned by the serialization functions
pub type Result<T> = core::result::Result<T, core::fmt::Error>;

/// [`std::io::Write`]-like trait for serialization that does not
/// depend on [`std`].
///
/// In an [`std`] context, the user does not need to use directly
/// this trait as we provide a blanket
/// implementation that implements [`WriteNoStd`] for all types that implement
/// [`std::io::Write`]. In particular, in such a context you can use [`std::io::Cursor`]
/// for in-memory serialization.
pub trait WriteNoStd {
    /// Write some bytes and return the number of bytes written (trivial buf.len())
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flush all changes to the underlying storage if applicable
    fn flush(&mut self) -> Result<()>;
}

#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
impl<W: Write> WriteNoStd for W {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Write::write(self, buf).map_err(|_| core::fmt::Error)
    }
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        Write::flush(self).map_err(|_| core::fmt::Error)
    }
}

/// A little wrapper around a writer that keeps track of the current position
/// so we can align the data.
///
/// This is needed because the [`Write`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
pub struct WriteWithPos<F: WriteNoStd> {
    /// What we actually write on
    backend: F,
    /// How many bytes we have written from the start
    pos: usize,
}

impl<F: WriteNoStd> WriteWithPos<F> {
    #[inline(always)]
    /// Create a new [`WriteWithPos`] on top of a generic writer `F`
    pub fn new(backend: F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<F: WriteNoStd> FieldWrite for WriteWithPos<F> {
    #[inline(always)]
    fn get_pos(&self) -> usize {
        self.pos
    }
}

impl<F: WriteNoStd> WriteNoStd for WriteWithPos<F> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let res = self.backend.write(buf)?;
        self.pos += res;
        Ok(res)
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }
}

/// Trait providing methods to write fields and bytes; moreover,
/// implementors need to keep track of the current position
/// in the [`WriteNoStd`] stream. This is needed to guarante the correct alignment of the data to
/// allow zero-copy deserialization.
///
/// This is not meant to be used by the user and is only used internally.
/// Moreover, [`FieldWrite::add_padding_to_align`] and [`FieldWrite::add_field`]
/// could be implemented with [`FieldWrite::add_field_bytes`], but having this
/// specialization allows us to automatically generate the schema.
pub trait FieldWrite: WriteNoStd + Sized {
    #[inline(always)]
    /// Add some zero padding so that `self.get_pos() % align == 0`
    fn add_padding_to_align(&mut self, align: usize) -> Result<()> {
        let padding = pad_align_to(self.get_pos(), align);
        for _ in 0..padding {
            self.write(&[0])?;
        }
        Ok(())
    }

    #[inline(always)]
    /// Add a complex field to the serialization, this is mostly used by the
    /// full-copy implementations
    fn add_field<V: SerializeInner>(self, _field_name: &str, value: &V) -> Result<Self> {
        value._serialize_inner(self)
    }

    #[inline(always)]
    /// Add raw bytes to the serialization, this is mostly used by the zero-copy
    /// implementations
    fn add_field_bytes(
        mut self,
        _field_name: &str,
        _type_name: String,
        value: &[u8],
        _align: usize,
    ) -> Result<Self> {
        self.write(value)?;
        Ok(self)
    }

    /// Get how many bytes we wrote from the start of the serialization
    fn get_pos(&self) -> usize;
}

#[derive(Debug, Clone)]
/// A row in the schema csv
pub struct SchemaRow {
    /// Name of the field
    pub field: String,
    /// Type of the field
    pub ty: String,
    /// Offset of the field from the start of the file
    pub offset: usize,
    /// The length in bytes of the field
    pub size: usize,
    /// The alignment needed by the field, this is mostly to check if the
    /// serialization is correct
    pub align: usize,
}

#[derive(Default, Debug, Clone)]
/// All the informations needed to decode back the data from another language.
///
/// The schma is not guaranteed to be sorted.
pub struct Schema(pub Vec<SchemaRow>);

impl Schema {
    /// Return in a String the csv representation of the schema
    /// also printing the bytes of the data used to decode each leaf field.
    ///
    /// The schema is not guaranteed to be sorted, so if you need it sorted use:
    ///  `schema.0.sort_by_key(|row| row.offset);`
    ///
    /// WARNING: the size of the csv will be bigger than the size of the
    /// serialized file, so it's a bad idea calling this on big data structures.
    pub fn debug(&self, data: &[u8]) -> String {
        let mut result = "field,offset,align,size,ty,bytes\n".to_string();
        for i in 0..self.0.len().saturating_sub(1) {
            let row = &self.0[i];
            // if it's a composed type, don't print the bytes
            if row.offset == self.0[i + 1].offset {
                result.push_str(&format!(
                    "{},{},{},{},{},\n",
                    row.field, row.offset, row.align, row.size, row.ty,
                ));
            } else {
                result.push_str(&format!(
                    "{},{},{},{},{},{:02x?}\n",
                    row.field,
                    row.offset,
                    row.align,
                    row.size,
                    row.ty,
                    &data[row.offset..row.offset + row.size],
                ));
            }
        }

        // the last field can't be a composed type by definition
        if let Some(row) = self.0.last() {
            result.push_str(&format!(
                "{},{},{},{},{},{:02x?}\n",
                row.field,
                row.offset,
                row.align,
                row.size,
                row.ty,
                &data[row.offset..row.offset + row.size],
            ));
        }

        result
    }

    /// Return in a String the csv representation of the schema.
    ///
    /// The schema is not guaranteed to be sorted, so if you need it sorted use:
    ///  `schema.0.sort_by_key(|row| row.offset);`
    pub fn to_csv(&self) -> String {
        let mut result = "field,offset,align,size,ty\n".to_string();
        for row in &self.0 {
            result.push_str(&format!(
                "{},{},{},{},{}\n",
                row.field, row.offset, row.align, row.size, row.ty
            ));
        }
        result
    }
}

/// Internal writer that keeps track of the schema and the path of the field
/// we are serializing
pub struct SchemaWriter<W: FieldWrite> {
    /// The schema so far
    pub schema: Schema,
    /// The "path" of the previous fields names
    path: Vec<String>,
    /// What we actually write on
    writer: W,
}

impl<W: FieldWrite> SchemaWriter<W> {
    #[inline(always)]
    /// Create a new empty [`SchemaWriter`] on top of a generic writer `W`
    pub fn new(backend: W) -> Self {
        Self {
            schema: Default::default(),
            path: vec![],
            writer: backend,
        }
    }
}

impl<W: FieldWrite> FieldWrite for SchemaWriter<W> {
    #[inline(always)]
    fn add_padding_to_align(&mut self, align: usize) -> Result<()> {
        let padding = pad_align_to(self.get_pos(), align);
        if padding == 0 {
            return Ok(());
        }

        let off = self.schema.0.last_mut().unwrap().offset;

        for row in self.schema.0.iter_mut().rev() {
            if row.offset < off {
                break;
            }
            row.offset += padding;
        }

        self.schema.0.push(SchemaRow {
            field: "PADDING".into(),
            ty: format!("[u8; {}]", padding),
            offset: self.get_pos(),
            size: padding,
            align: 1,
        });
        for _ in 0..padding {
            self.write(&[0])?;
        }
        Ok(())
    }

    #[inline(always)]
    fn add_field<V: SerializeInner>(mut self, field_name: &str, value: &V) -> Result<Self> {
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        let struct_idx = self.schema.0.len();
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: core::any::type_name::<V>().to_string(),
            offset: self.get_pos(),
            align: core::mem::align_of::<V>(),
            size: 0,
        });
        // serialize the value
        self = value._serialize_inner(self)?;
        // compute the serialized size and update the schema
        let size = self.get_pos() - self.schema.0[struct_idx].offset;
        self.schema.0[struct_idx].size = size;
        self.path.pop();
        Ok(self)
    }

    #[inline(always)]
    fn add_field_bytes(
        mut self,
        field_name: &str,
        type_name: String,
        value: &[u8],
        align: usize,
    ) -> Result<Self> {
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: type_name,
            offset: self.get_pos(),
            size: value.len(),
            align,
        });
        self.writer.write(value)?;
        self.path.pop();
        Ok(self)
    }

    #[inline(always)]
    fn get_pos(&self) -> usize {
        self.writer.get_pos()
    }
}

impl<W: FieldWrite> WriteNoStd for SchemaWriter<W> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}