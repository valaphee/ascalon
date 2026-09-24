use std::{collections::HashSet, env, fs, path::PathBuf};

use ascalon_packfile_schema::{Field, Packfile};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::Type;

fn ident(name: &str) -> Ident {
    let name = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    syn::parse_str(&name).unwrap_or_else(|_| Ident::new_raw(&name, Span::call_site()))
}

fn field_type(field: &Field) -> Option<Type> {
    Some(match field.r#type.as_str() {
        "Struct" => {
            let ty = ident(field.type_name.as_deref()?);
            syn::parse_quote!(#ty)
        }
        "Array" => {
            let ty = ident(field.type_name.as_deref()?);
            let size = field.size;
            syn::parse_quote!([#ty; #size])
        }
        "Ptr" => {
            let ty = ident(field.type_name.as_deref()?);
            syn::parse_quote!(Ptr<#ty>)
        }
        "ArrayPtr" => {
            let ty = ident(field.type_name.as_deref()?);
            syn::parse_quote!(ArrayPtr<#ty>)
        }
        ty => {
            let ty = ident(ty);
            syn::parse_quote!(#ty)
        }
    })
}

fn emit_struct(name: &str, fields: &[Field], seen: &mut HashSet<String>) -> TokenStream {
    let name = ident(name);
    let key = name.to_string();

    if !seen.insert(key) {
        return TokenStream::new();
    }

    let fields = fields.iter().filter_map(|field| {
        let name = ident(&field.name);
        let ty = field_type(field)?;

        Some(quote! {
            pub #name: #ty,
        })
    });

    quote! {
        #[derive(Debug, FromBytes, KnownLayout, Immutable)]
        #[repr(C)]
        pub struct #name {
            #(#fields)*
        }
    }
}

fn emit_nested(fields: &[Field], seen: &mut HashSet<String>) -> TokenStream {
    let mut output = TokenStream::new();

    for field in fields {
        if field.fields.is_empty() {
            continue;
        }

        let Some(name) = field.type_name.as_deref() else {
            continue;
        };

        output.extend(emit_struct(name, &field.fields, seen));
        output.extend(emit_nested(&field.fields, seen));
    }

    output
}

fn emit_packfile(packfile: &Packfile) -> TokenStream {
    let module = format_ident!("{}", packfile.name.to_ascii_lowercase());

    let mut seen = HashSet::new();
    let mut structs = TokenStream::new();

    for chunk in &packfile.chunks {
        for version in &chunk.versions {
            structs.extend(emit_struct(&version.type_name, &version.fields, &mut seen));
            structs.extend(emit_nested(&version.fields, &mut seen));
        }
    }

    quote! {
        #[allow(non_snake_case)]
        pub mod #module {
            use super::*;

            #structs
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=packfiles.xml");

    let xml = fs::read_to_string("packfiles.xml").unwrap();
    let packfiles = ascalon_packfile_schema::parse(&xml);

    let modules = packfiles.iter().map(emit_packfile);

    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("packfiles.rs"),
        quote!(#(#modules)*).to_string(),
    )
    .unwrap();
}
