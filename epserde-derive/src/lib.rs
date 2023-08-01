/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput};

struct CommonDeriveInput {
    name: syn::Ident,
    generics: proc_macro2::TokenStream,
    generics_names: proc_macro2::TokenStream,
    generics_names_raw: Vec<String>,
    consts_names_raw: Vec<String>,
    where_clause: proc_macro2::TokenStream,
}

impl CommonDeriveInput {
    fn new(
        input: DeriveInput,
        traits_to_add: Vec<syn::Path>,
        lifetimes_to_add: Vec<syn::Lifetime>,
    ) -> Self {
        let name = input.ident;
        let mut generics = quote!();
        let mut generics_names_raw = vec![];
        let mut consts_names_raw = vec![];
        let mut generics_names = quote!();
        if !input.generics.params.is_empty() {
            input.generics.params.iter().for_each(|x| {
                match x {
                    syn::GenericParam::Type(t) => {
                        generics_names.extend(t.ident.to_token_stream());
                        generics_names_raw.push(t.ident.to_string());
                    }
                    syn::GenericParam::Lifetime(l) => {
                        generics_names.extend(l.lifetime.to_token_stream());
                    }
                    syn::GenericParam::Const(c) => {
                        generics_names.extend(c.ident.to_token_stream());
                        consts_names_raw.push(c.ident.to_string());
                    }
                };
                generics_names.extend(quote!(,))
            });
            input.generics.params.into_iter().for_each(|x| match x {
                syn::GenericParam::Type(t) => {
                    let mut t = t.clone();
                    for trait_to_add in traits_to_add.iter() {
                        t.bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
                            paren_token: None,
                            modifier: syn::TraitBoundModifier::None,
                            lifetimes: None,
                            path: trait_to_add.clone(),
                        }));
                    }
                    for lifetime_to_add in lifetimes_to_add.iter() {
                        t.bounds
                            .push(syn::TypeParamBound::Lifetime(lifetime_to_add.clone()));
                    }
                    generics.extend(quote!(#t,));
                }
                x => {
                    generics.extend(x.to_token_stream());
                    generics.extend(quote!(,))
                }
            });
        }

        let where_clause = input
            .generics
            .where_clause
            .map(|x| x.to_token_stream())
            .unwrap_or(quote!(where));

        Self {
            name: name,
            generics: generics,
            generics_names: generics_names,
            where_clause: where_clause,
            generics_names_raw,
            consts_names_raw,
        }
    }
}

#[proc_macro_derive(Serialize)]
pub fn epserde_serialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let CommonDeriveInput {
        name,
        generics,
        generics_names,
        where_clause,
        ..
    } = CommonDeriveInput::new(
        input.clone(),
        vec![syn::parse_quote!(epserde_trait::ser::SerializeInner)],
        vec![],
    );
    // We have to play with this to get type parameters working

    let out = match input.data {
        Data::Struct(s) => {
            let mut write_all_opt = quote!(true);
            s.fields.iter().for_each(|field| {
                let ty = &field.ty;
                write_all_opt.extend(
                    quote!(&& <#ty as epserde_trait::ser::SerializeInner>::WRITE_ALL_OPTIMIZATION),
                );
            });

            let fields = s.fields.into_iter().map(|field| field.ident.unwrap());
            quote! {
                #[automatically_derived]
                impl<#generics> epserde_trait::ser::SerializeInner for #name<#generics_names> #where_clause {
                    const WRITE_ALL_OPTIMIZATION: bool = #write_all_opt;

                    #[inline(always)]
                    fn _serialize_inner<F: epserde_trait::ser::WriteWithPosNoStd>(&self, mut backend: F) -> epserde_trait::ser::Result<F> {
                        if Self::WRITE_ALL_OPTIMIZATION {
                            backend = <Self>::pad_align_to::<F>(backend)?;
                        }
                        #(
                            backend= backend.add_field(stringify!(#fields), &self.#fields)?;
                        )*
                        Ok(backend)
                    }
                }
            }
        }
        _ => todo!(),
    };
    out.into()
}

#[proc_macro_derive(Deserialize)]
pub fn epserde_deserialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let CommonDeriveInput {
        generics: generics_fc,
        generics_names: generics_names_fc,
        where_clause: where_clause_fc,
        ..
    } = CommonDeriveInput::new(
        input.clone(),
        vec![syn::parse_quote!(epserde_trait::des::DeserializeInner)],
        vec![],
    );
    let CommonDeriveInput {
        name,
        generics: generics_zc,
        generics_names: generics_names_zc,
        generics_names_raw,
        where_clause: where_clause_zc,
        ..
    } = CommonDeriveInput::new(input.clone(), vec![], vec![syn::parse_quote!('static)]);

    let out = match input.data {
        Data::Struct(s) => {
            let fields = s
                .fields
                .iter()
                .map(|field| field.ident.to_owned().unwrap())
                .collect::<Vec<_>>();

            let fields_types = s
                .fields
                .iter()
                .map(|field| field.ty.to_owned())
                .collect::<Vec<_>>();

            let mut non_generic_fields = vec![];
            let mut non_generic_types = vec![];
            let mut generic_fields = vec![];
            let mut generic_types = vec![];

            s.fields.iter().for_each(|field| {
                let ty = &field.ty;
                let field_name = field.ident.clone().unwrap();
                if generics_names_raw.contains(&ty.to_token_stream().to_string()) {
                    generic_fields.push(field_name);
                    generic_types.push(ty);
                } else {
                    non_generic_fields.push(field_name);
                    non_generic_types.push(ty);
                }
            });

            quote! {
                #[automatically_derived]
                impl<#generics_fc> epserde_trait::des::DeserializeInner for #name<#generics_names_fc> #where_clause_fc{
                    fn deserialize_inner<'epserde_trait_lifetime>(
                        backend: epserde_trait::des::Cursor<'epserde_trait_lifetime>,
                    ) -> core::result::Result<(Self, epserde_trait::des::Cursor<'epserde_trait_lifetime>), epserde_trait::des::DeserializeError> {
                        use epserde_trait::des::DeserializeInner;
                        #(
                            let (#fields, backend) = <#fields_types>::deserialize_inner(backend)?;
                        )*
                        Ok((#name{
                            #(#fields),*
                        }, backend))
                    }
                }

                #[automatically_derived]
                impl<#generics_zc> epserde_trait::des::DeserializeZeroCopyInner for epserde_trait::des::DesWrap<#name<#generics_names_zc>> #where_clause_zc
                    #(
                        for<'epserde_trait_lifetime_generic> &'epserde_trait_lifetime_generic &'epserde_trait_lifetime_generic &'epserde_trait_lifetime_generic epserde_trait::des::DesWrap<#generic_types>: epserde_trait::des::DeserializeZeroCopyInner,
                    )*
                {

                    type DesType<'b> = #name<#(
                        <&'b &'b &'b epserde_trait::des::DesWrap<#generic_types> as epserde_trait::des::DeserializeZeroCopyInner>::DesType<'b>
                    ,)*>;

                    fn deserialize_zc_inner<'epserde_trait_lifetime>(
                        backend: epserde_trait::des::Cursor<'epserde_trait_lifetime>,
                    ) -> core::result::Result<(Self::DesType<'epserde_trait_lifetime>, epserde_trait::des::Cursor<'epserde_trait_lifetime>), epserde_trait::des::DeserializeError>{
                        use epserde_trait::des::DeserializeZeroCopyInner;
                        use epserde_trait::des::DeserializeInner;
                        #(
                            let (#generic_fields, backend) = <&'epserde_trait_lifetime &'epserde_trait_lifetime &'epserde_trait_lifetime epserde_trait::des::DesWrap<#generic_types>>::deserialize_zc_inner(backend)?;
                        )*
                        #(
                            let (#non_generic_fields, backend) = <#non_generic_types>::deserialize_inner(backend)?;
                        )*
                        Ok((#name{
                            #(#fields),*
                        }, backend))
                    }
                }

            }
        }
        _ => todo!(),
    };
    out.into()
}

#[proc_macro_derive(MemSize)]
pub fn epserde_mem_size(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let CommonDeriveInput {
        name,
        generics,
        generics_names,
        where_clause,
        ..
    } = CommonDeriveInput::new(
        input.clone(),
        vec![syn::parse_quote!(epserde_trait::MemSize)],
        vec![],
    );

    let out = match input.data {
        Data::Struct(s) => {
            let fields = s
                .fields
                .iter()
                .map(|field| field.ident.to_owned().unwrap())
                .collect::<Vec<_>>();

            quote! {
                #[automatically_derived]
                impl<#generics> epserde_trait::MemSize for #name<#generics_names> #where_clause{
                    fn mem_size(&self) -> usize {
                        let mut bytes = 0;
                        #(bytes += self.#fields.mem_size();)*
                        bytes
                    }

                    fn _mem_dbg_recourse_on<W: core::fmt::Write>(
                        &self,
                        writer: &mut W,
                        depth: usize,
                        max_depth: usize,
                        type_name: bool,
                        humanize: bool,
                    ) -> core::fmt::Result {
                        #(self.#fields.mem_dbg_depth_on(writer, depth + 1, max_depth, Some(stringify!(#fields)), type_name, humanize)?;)*
                        Ok(())
                    }
                }
            }
        }
        _ => todo!(),
    };
    out.into()
}

#[proc_macro_derive(TypeName)]
pub fn epserde_type_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let CommonDeriveInput {
        name,
        generics,
        generics_names,
        where_clause,
        generics_names_raw,
        consts_names_raw,
    } = CommonDeriveInput::new(
        input.clone(),
        vec![syn::parse_quote!(epserde_trait::TypeName)],
        vec![],
    );

    let out = match input.data {
        Data::Struct(s) => {
            let fields_names = s
                .fields
                .iter()
                .map(|field| field.ident.to_owned().unwrap().to_string())
                .collect::<Vec<_>>();

            let fields_types = s
                .fields
                .iter()
                .map(|field| field.ty.to_owned())
                .collect::<Vec<_>>();

            let type_name: proc_macro2::TokenStream = if generics.is_empty() {
                format!("\"{}\".into()", name)
            } else {
                let mut res = "format!(\"".to_string();
                res += &name.to_string();
                res += "<";
                for _ in 0..generics_names_raw.len() + consts_names_raw.len() {
                    res += "{}, ";
                }
                res.pop();
                res.pop();
                res += ">\",";

                for gn in generics_names_raw.iter() {
                    res += &format!("{}::type_name()", gn);
                    res += ",";
                }
                res.pop();
                res += ")";
                res
            }
            .parse()
            .unwrap();

            let name_literal = format!("\"{}\"", type_name);

            quote! {
                #[automatically_derived]
                impl<#generics> epserde_trait::TypeName for #name<#generics_names> #where_clause{
                    /// Just the type name, without the module path.
                    fn type_name() -> String {
                        #type_name
                    }

                    #[inline(always)]
                    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
                        use core::hash::Hash;
                        #name_literal.hash(hasher);
                        #(
                            #fields_names.hash(hasher);
                        )*
                        #(
                            <#fields_types as epserde_trait::TypeName>::type_hash(hasher);
                        )*
                    }
                }
            }
        }
        _ => todo!(),
    };
    out.into()
}
