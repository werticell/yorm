#![forbid(unsafe_code)]

use proc_macro::{TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Attribute, DeriveInput, Field, NestedMeta, };
use syn::punctuated::Punctuated;
use syn::token::{Comma};

#[proc_macro_derive(Object, attributes(table_name, column_name))]
pub fn derive_object(input: TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = &input.ident;
    let table_name = if input.attrs.is_empty() {
        type_name.clone()
    } else {
        syn::Ident::new(&get_attribute_ident(&input.attrs[0]), syn::__private::Span::call_site())
    };


    let struct_ = match input.data {
        syn::Data::Struct(struct_) => struct_,
        _ => panic!("Not implemented for not structs"),
    };
    let named_fields = match struct_.fields {
        syn::Fields::Named(fields) => Some(fields.named),
        syn::Fields::Unit => None,
        _ => panic!("Not implemented for other type of fields"),
    };

    let field_names = make_field_names(named_fields.as_ref());
    let column_names = make_column_names(named_fields.as_ref());
    let column_types = make_column_types(named_fields.as_ref());

    let as_row = make_as_row(named_fields.as_ref());
    let from_row = make_from_row(named_fields.as_ref());

    let expanded = quote! {
        impl Object for #type_name {
            fn as_row(&self) -> orm::storage::Row {
                vec![#as_row]
            }
            fn from_row(mut row: orm::storage::Row) -> Self {
                Self { #from_row }
            }
            fn table_name() -> &'static str {
                stringify!(#table_name)
            }
            fn type_name() -> &'static str {
                stringify!(#type_name)
            }
            fn field_names() -> std::vec::Vec<&'static str> {
                vec![#field_names]
            }
            fn column_names() -> std::vec::Vec<&'static str> {
                vec![#column_names]
            }
            fn column_types() -> std::vec::Vec<DataType> {
                vec![#column_types]
            }
        }
    };
    TokenStream::from(expanded)
}

///////////////////////////////////////////////////////////////////////////////////////////////////

fn make_field_names(named_fields: Option<&Punctuated<Field, Comma>>) -> quote::__private::TokenStream {
    if named_fields.is_none() {
        return quote! {};
    }
    let recurse = named_fields
        .unwrap()
        .iter()
        .map(|p| {
            let field_name = p.ident.as_ref().unwrap();
            quote! {
                stringify!(#field_name)
            }
        });
    quote! { #(#recurse,)* }
}

fn make_column_names(named_fields: Option<&Punctuated<Field, Comma>>) -> quote::__private::TokenStream {
    if named_fields.is_none() {
        return quote! {};
    }
    let recurse = named_fields
        .unwrap()
        .iter()
        .map(|p| {
            let column_name = if p.attrs.is_empty() {
                p.ident.as_ref().unwrap().clone()
            } else {
                syn::Ident::new(&get_attribute_ident(&p.attrs[0]), syn::__private::Span::call_site())
            };
            quote! {
                stringify!(#column_name)
            }
        });
    quote! { #(#recurse,)* }
}

fn make_as_row(named_fields: Option<&Punctuated<Field, Comma>>) -> quote::__private::TokenStream {
    if named_fields.is_none() {
        return quote! {};
    }
    let recurse = named_fields
        .unwrap()
        .iter()
        .map(|p| {
            let ident = p.ident.as_ref().unwrap();
            quote! {
                self.#ident.clone().into()
            }
        });
    quote! { #(#recurse,)* }
}

fn make_from_row(named_fields: Option<&Punctuated<Field, Comma>>) -> quote::__private::TokenStream {
    if named_fields.is_none() {
        return quote! {};
    }
    let recurse = named_fields
        .unwrap()
        .iter()
        .rev()
        .map(|p| {
            let ident = p.ident.as_ref().unwrap();
            quote! {
                #ident: row.pop().unwrap().into()
            }
        });
    quote! { #(#recurse,)* }

}

fn make_column_types(named_fields: Option<&Punctuated<Field, Comma>>) -> quote::__private::TokenStream {
    if named_fields.is_none() {
        return quote! {};
    }
    let recurse = named_fields
        .unwrap()
        .iter()
        .map(|p| {
            let ident = p.ty.to_token_stream();
            quote! {
                stringify!(#ident).into()
            }
        });
    quote! { #(#recurse,)* }
}

///////////////////////////////////////////////////////////////////////////////////////////////////


fn get_attribute_ident(attr: &Attribute) -> String {
    match attr.parse_meta().unwrap() {
        syn::Meta::List(syn::MetaList {nested, ..} ) => {
            match nested.first().unwrap() {
                NestedMeta::Lit(syn::Lit::Str(lit_str)) => {
                    lit_str.value()
                }
                // NestedMeta::Meta(Meta::Path(syn::Path{segments, ..})) => {
                //     let syn::PathSegment{ident,..} = &segments[0];
                //     ident.clone()
                // },
                _ => panic!("not implemented 1")
            }
        },
        _ => panic!("Not implemented 2")
    }
}

