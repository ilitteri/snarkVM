// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, Index, Type};

enum IdentOrIndex {
    Ident(proc_macro2::Ident),
    Index(Index),
}

impl ToTokens for IdentOrIndex {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::Index(index) => index.to_tokens(tokens),
        }
    }
}

fn impl_serialize_field(
    serialize_body: &mut Vec<TokenStream>,
    serialized_size_body: &mut Vec<TokenStream>,
    serialize_uncompressed_body: &mut Vec<TokenStream>,
    uncompressed_size_body: &mut Vec<TokenStream>,
    idents: &mut Vec<IdentOrIndex>,
    ty: &Type,
) {
    // Check if type is a tuple.
    match ty {
        Type::Tuple(tuple) => {
            for (i, elem_ty) in tuple.elems.iter().enumerate() {
                let index = Index::from(i);
                idents.push(IdentOrIndex::Index(index));
                impl_serialize_field(
                    serialize_body,
                    serialized_size_body,
                    serialize_uncompressed_body,
                    uncompressed_size_body,
                    idents,
                    elem_ty,
                );
                idents.pop();
            }
        }
        _ => {
            serialize_body.push(quote! { CanonicalSerialize::serialize(&self.#(#idents).*, writer)?; });
            serialized_size_body.push(quote! { size += CanonicalSerialize::serialized_size(&self.#(#idents).*); });
            serialize_uncompressed_body
                .push(quote! { CanonicalSerialize::serialize_uncompressed(&self.#(#idents).*, writer)?; });
            uncompressed_size_body.push(quote! { size += CanonicalSerialize::uncompressed_size(&self.#(#idents).*); });
        }
    }
}

pub(crate) fn impl_canonical_serialize(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let len = if let Data::Struct(ref data_struct) = ast.data {
        data_struct.fields.len()
    } else {
        panic!("Serialize can only be derived for structs, {} is not a struct", name);
    };

    let mut serialize_body = Vec::<TokenStream>::with_capacity(len);
    let mut serialized_size_body = Vec::<TokenStream>::with_capacity(len);
    let mut serialize_uncompressed_body = Vec::<TokenStream>::with_capacity(len);
    let mut uncompressed_size_body = Vec::<TokenStream>::with_capacity(len);

    match ast.data {
        Data::Struct(ref data_struct) => {
            let mut idents = Vec::<IdentOrIndex>::new();

            for (i, field) in data_struct.fields.iter().enumerate() {
                match field.ident {
                    None => {
                        let index = Index::from(i);
                        idents.push(IdentOrIndex::Index(index));
                    }
                    Some(ref ident) => {
                        idents.push(IdentOrIndex::Ident(ident.clone()));
                    }
                }

                impl_serialize_field(
                    &mut serialize_body,
                    &mut serialized_size_body,
                    &mut serialize_uncompressed_body,
                    &mut uncompressed_size_body,
                    &mut idents,
                    &field.ty,
                );

                idents.clear();
            }
        }
        _ => panic!("Serialize can only be derived for structs, {} is not a struct", name),
    };

    let gen = quote! {
        impl #impl_generics CanonicalSerialize for #name #ty_generics #where_clause {
            #[allow(unused_mut, unused_variables)]
            fn serialize<W: snarkvm_utilities::Write>(&self, writer: &mut W) -> Result<(), snarkvm_utilities::SerializationError> {
                #(#serialize_body)*
                Ok(())
            }
            #[allow(unused_mut, unused_variables)]
            fn serialized_size(&self) -> usize {
                let mut size = 0;
                #(#serialized_size_body)*
                size
            }
            #[allow(unused_mut, unused_variables)]
            fn serialize_uncompressed<W: snarkvm_utilities::Write>(&self, writer: &mut W) -> Result<(), snarkvm_utilities::SerializationError> {
                #(#serialize_uncompressed_body)*
                Ok(())
            }
            #[allow(unused_mut, unused_variables)]
            fn uncompressed_size(&self) -> usize {
                let mut size = 0;
                #(#uncompressed_size_body)*
                size
            }
        }
    };
    gen
}
