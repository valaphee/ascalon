use std::{env, fs, path::PathBuf};

use ascalon_protocol_schema::{Message, MessageField, MessageGroup, Protocol, parse};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Item, Type, parse_quote};

fn ty(name: &str) -> Type {
    match name {
        "Byte" => parse_quote!(u8),
        "Word" => parse_quote!(u16),
        "Dword" => parse_quote!(u32),
        "Qword" => parse_quote!(u64),
        "Float" => parse_quote!(f32),
        "Float2" => parse_quote!([f32; 2]),
        "Float3" => parse_quote!([f32; 3]),
        "Float4" => parse_quote!([f32; 4]),
        "Point3" => parse_quote!(Point3),
        "Guid" => parse_quote!(uuid::Uuid),
        "Address" => parse_quote!(std::net::SocketAddr),
        "String" => parse_quote!(Box<[u16]>),
        "CString" => parse_quote!(String),
        _ => {
            let name = format_ident!("{name}");
            parse_quote!(#name)
        }
    }
}

fn field_ty(parent: &Ident, field: &MessageField) -> Type {
    let size = field.size;

    match field.r#type.as_str() {
        "Byte" | "Word" | "Dword" | "Qword" | "Float" | "Float2" | "Float3" | "Float4"
        | "Point3" | "Guid" | "Address" | "String" | "CString" => ty(&field.r#type),
        "Optional" => {
            let name = field
                .type_name
                .clone()
                .unwrap_or_else(|| format!("{parent}_{}", field.name));

            let ty = ty(&name);
            parse_quote!(Option<#ty>)
        }
        "ArrayFixed" => {
            let name = field
                .type_name
                .clone()
                .unwrap_or_else(|| format!("{parent}_{}", field.name));

            let ty = ty(&name);
            parse_quote!([#ty; #size])
        }
        "ArrayVarSmall" | "ArrayVarLarge" => {
            let name = field
                .type_name
                .clone()
                .unwrap_or_else(|| format!("{parent}_{}", field.name));

            let ty = ty(&name);
            parse_quote!(Box<[#ty]>)
        }
        "BufferFixed" => parse_quote!([u8; #size]),
        "BufferVarSmall" | "BufferVarLarge" => parse_quote!(Box<[u8]>),
        _ => todo!(),
    }
}

fn encode_stmt(field: &MessageField, value: TokenStream) -> TokenStream {
    let size = field.size;

    match field.r#type.as_str() {
        "String" => quote! {
            if #value.len() > #size {
                return Err(());
            }

            for value in #value.iter() {
                value.encode(buf)?;
            }
            0u16.encode(buf)?;
        },
        "CString" => quote! {
            if !#value.is_ascii() {
                return Err(());
            }

            let bytes = #value.as_bytes();
            if bytes.len() >= #size {
                return Err(());
            }

            buf.put_slice(bytes);
            0u8.encode(buf)?;
        },
        "ArrayVarSmall" | "ArrayVarLarge" => {
            let len_ty = if field.r#type == "ArrayVarSmall" {
                quote!(u8)
            } else {
                quote!(u16)
            };

            quote! {
                if #value.len() > #size {
                    return Err(());
                }

                (#value.len() as #len_ty).encode(buf)?;
                #value.iter().try_for_each(|v| v.encode(buf))?;
            }
        }
        "BufferVarSmall" | "BufferVarLarge" => {
            let len_ty = if field.r#type == "BufferVarSmall" {
                quote!(u8)
            } else {
                quote!(u16)
            };

            quote! {
                if #value.len() > #size {
                    return Err(());
                }

                (#value.len() as #len_ty).encode(buf)?;
                buf.put_slice(#value.as_ref());
            }
        }
        _ => quote!(#value.encode(buf)?;),
    }
}

fn decode_expr(parent: &Ident, field: &MessageField) -> TokenStream {
    let size = field.size;

    match field.r#type.as_str() {
        "String" => quote!({
            let mut value = Vec::with_capacity(#size);

            loop {
                let word = u16::decode(buf)?;
                if word == 0 {
                    break;
                }

                if value.len() >= #size {
                    return Err(());
                }

                value.push(word);
            }

            value.into_boxed_slice()
        }),
        "CString" => quote!({
            let end = buf
                .iter()
                .take(#size)
                .position(|&b| b == 0)
                .ok_or(())?;

            let value = std::str::from_utf8(&buf[..end])
                .map_err(|_| ())?
                .to_owned();

            *buf = &buf[end + 1..];

            value
        }),
        "ArrayVarSmall" | "ArrayVarLarge" => {
            let len = if field.r#type == "ArrayVarSmall" {
                quote!(u8::decode(buf)?)
            } else {
                quote!(u16::decode(buf)?)
            };

            let name = field
                .type_name
                .clone()
                .unwrap_or_else(|| format!("{parent}_{}", field.name));
            let ty = ty(&name);

            quote!({
                let len = #len as usize;
                if len > #size {
                    return Err(());
                }

                (0..len)
                    .map(|_| <#ty>::decode(buf))
                    .collect::<Result<_, _>>()?
            })
        }
        "BufferVarSmall" | "BufferVarLarge" => {
            let len = if field.r#type == "BufferVarSmall" {
                quote!(u8::decode(buf)?)
            } else {
                quote!(u16::decode(buf)?)
            };

            quote!({
                let len = #len as usize;
                if len > #size {
                    return Err(());
                }

                let value = Box::<[u8]>::from(buf.get(..len).ok_or(())?);
                *buf = &buf[len..];

                value
            })
        }
        _ => quote!(Decode::decode(buf)?),
    }
}

fn generate_struct(name: Ident, fields: &[MessageField]) -> Vec<Item> {
    let mut items = Vec::new();

    for field in fields.iter().filter(|field| !field.elem.is_empty()) {
        let nested_name = format_ident!("{name}_{}", field.name);
        items.extend(generate_struct(nested_name, &field.elem));
    }

    let names = fields
        .iter()
        .map(|field| format_ident!("{}", field.name))
        .collect::<Vec<_>>();

    let types = fields
        .iter()
        .map(|field| field_ty(&name, field))
        .collect::<Vec<_>>();

    let encode = fields
        .iter()
        .zip(&names)
        .map(|(field, name)| encode_stmt(field, quote!(self.#name)))
        .collect::<Vec<_>>();

    let decode = fields
        .iter()
        .map(|field| decode_expr(&name, field))
        .collect::<Vec<_>>();

    items.push(parse_quote! {
        #[derive(Debug)]
        pub struct #name {
            #(pub #names: #types),*
        }
    });

    items.push(parse_quote! {
        impl Encode for #name {
            fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), ()> {
                #(#encode)*
                Ok(())
            }
        }
    });

    items.push(parse_quote! {
        impl Decode for #name {
            fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
                Ok(Self {
                    #(#names: #decode),*
                })
            }
        }
    });

    items
}

fn generate_messages(group_name: &str, messages: &[Message]) -> Vec<Item> {
    messages
        .iter()
        .flat_map(|message| {
            let name = format_ident!("{}", message.name);
            let id = message.id;

            let mut items = generate_struct(name.clone(), &message.elem);

            items.push(parse_quote! {
                impl super::private::Sealed for #name {}
            });

            items.push(parse_quote! {
                impl super::MessageType for #name {
                    const ID: u16 = #id;
                }
            });

            items.push(parse_quote! {
                impl super::Message for #name {
                    fn id(&self) -> u16 {
                        Self::ID
                    }

                    fn group_name(&self) -> &'static str {
                        #group_name
                    }
                }
            });

            items
        })
        .collect()
}

fn generate_group(group: &MessageGroup, server: bool) -> Option<Item> {
    let messages = if server { &group.server } else { &group.client };

    if messages.is_empty() {
        return None;
    }

    let module = format_ident!("{}", group.name);
    let messages = generate_messages(&group.name, messages);

    Some(parse_quote! {
        pub mod #module {
            use super::*;
            #(#messages)*
        }
    })
}

fn generate_direction(protocol: &Protocol, server: bool) -> Item {
    let module = format_ident!("{}", if server { "Server" } else { "Client" });

    let decode_arms = protocol
        .msgs
        .iter()
        .flat_map(|group| {
            let group_name = format_ident!("{}", group.name);
            let messages = if server { &group.server } else { &group.client };

            messages.iter().map(move |message| {
                let id = message.id;
                let ty = format_ident!("{}", message.name);

                quote! {
                    #id => Ok(Box::new(#group_name::#ty::decode(buf)?))
                }
            })
        })
        .collect::<Vec<_>>();

    let groups = protocol
        .msgs
        .iter()
        .filter_map(|group| generate_group(group, server))
        .collect::<Vec<_>>();

    parse_quote! {
        pub mod #module {
            use super::*;

            mod private {
                pub trait Sealed {}
            }

            pub trait MessageType: private::Sealed {
                const ID: u16;
            }

            pub trait Message: Encode + Debug + Send + Sync {
                fn id(&self) -> u16;
                fn group_name(&self) -> &'static str;
            }

            impl dyn Message {
                pub fn is<T: MessageType>(&self) -> bool {
                    self.id() == T::ID
                }

                pub fn downcast_ref<T: MessageType>(&self) -> Option<&T> {
                    (self.id() == T::ID).then(|| unsafe {
                        &*(self as *const dyn Message as *const T)
                    })
                }

                pub fn downcast_mut<T: MessageType>(&mut self) -> Option<&mut T> {
                    (self.id() == T::ID).then(|| unsafe {
                        &mut *(self as *mut dyn Message as *mut T)
                    })
                }
            }

            impl Encode for Box<dyn Message> {
                fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), ()> {
                    self.id().encode(buf)?;
                    (**self).encode(buf)
                }
            }

            impl Decode for Box<dyn Message> {
                fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
                    match u16::decode(buf)? {
                        #(#decode_arms,)*
                        _ => Err(()),
                    }
                }
            }

            #(#groups)*
        }
    }
}

fn generate_protocol(protocol: &Protocol) -> Item {
    let module = format_ident!("{}", protocol.name);
    let client = generate_direction(protocol, false);
    let server = generate_direction(protocol, true);

    parse_quote! {
        pub mod #module {
            use super::*;
            #client
            #server
        }
    }
}

fn main() {
    const INPUT: &str = "../research/protocols.xml";
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("protocols.rs");

    println!("cargo:rerun-if-changed={INPUT}");
    println!("cargo:rerun-if-changed=build.rs");

    let protocols = parse(&fs::read_to_string(INPUT).unwrap());

    let modules = protocols.iter().map(generate_protocol).collect::<Vec<_>>();

    fs::write(output, quote!(#(#modules)*).to_string()).unwrap();
}
