use crate::{
    attributes::{self, Field},
    bounded_generics,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataStruct, DeriveInput};

pub fn expand(
    input: &DeriveInput,
    data: &DataStruct,
    progressive: bool,
    krate: &TokenStream,
) -> syn::Result<TokenStream> {
    let mut fields = attributes::fields(data, progressive)?;
    fields.sort_by_key(|field| field.position);
    let types: Vec<_> = fields.iter().map(|field| &field.ty).collect();
    let generics = bounded_generics(input, &types, krate);
    let (params, args, bounds) = generics.split_for_impl();
    let name = &input.ident;
    let (descriptor, mask) = metadata(&fields, progressive, krate);
    let trusted = trusted_fixed_layout(&fields, progressive, krate);
    let encode = encode_method(&fields, krate, progressive);
    let decode = decode_method(&fields, krate, progressive);
    let bulk = bulk_methods(&fields, krate, progressive);
    let root = root_method(&fields, &mask, krate);
    Ok(quote! {
        impl #params #krate::codec::Ssz for #name #args #bounds {
            const TYPE: &'static #krate::type_info::TypeDescriptor = &#descriptor;
            const TRUSTED_FIXED_LAYOUT: bool = #trusted;
            #encode #decode #bulk #root
        }
    })
}

fn encode_method(fields: &[Field], krate: &TokenStream, progressive: bool) -> TokenStream {
    let append = append_method(fields, krate);
    if progressive {
        let encode = slow_encode(fields, krate);
        return quote!(#encode #append);
    }
    let types: Vec<_> = fields.iter().map(|field| &field.ty).collect();
    let layout = const_fixed_layout(&types, krate);
    let classify = classify_fixed(fields, krate);
    let writes = write_fixed(fields, krate);
    let allocating = allocating_encode(&layout, &classify, &writes, fields, krate);
    let encode = fast_encode_impl(
        layout,
        classify,
        writes,
        slow_len_body(fields, krate),
        slow_into_body(fields, krate),
        allocating,
        krate,
    );
    quote!(#encode #append)
}

fn allocating_encode(
    layout: &TokenStream,
    classify: &TokenStream,
    writes: &TokenStream,
    fields: &[Field],
    krate: &TokenStream,
) -> TokenStream {
    let direct = write_declared_fixed(fields, krate);
    quote! {
        fn encode(&self) -> ::core::result::Result<::std::vec::Vec<u8>, #krate::error::CodecError> {
            if let ::core::result::Result::Ok(::core::option::Option::Some((__ssz_total, __ssz_widths))) = #layout {
                const { #krate::codec::validate_type::<Self>() }?;
                if const { Self::TRUSTED_FIXED_LAYOUT } {
                    let mut __ssz_output = ::std::vec![0; __ssz_total];
                    let mut __ssz_rest = __ssz_output.as_mut_slice();
                    #direct
                    return ::core::result::Result::Ok(__ssz_output);
                }
                #classify
                if !__ssz_slow {
                    let __ssz_total = #krate::encoding::finish_fixed_total(__ssz_total, __ssz_oversize)?;
                    let mut __ssz_output = ::std::vec![0; __ssz_total];
                    let mut __ssz_rest = __ssz_output.as_mut_slice();
                    #writes
                    if !__ssz_rest.is_empty() {
                        return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
                    }
                    return ::core::result::Result::Ok(__ssz_output);
                }
            }
            let mut output = ::std::vec::Vec::new();
            self.encode_append(&mut output)?;
            ::core::result::Result::Ok(output)
        }
    }
}

fn fast_encode_impl(
    layout: TokenStream,
    classify: TokenStream,
    writes: TokenStream,
    length: TokenStream,
    write: TokenStream,
    allocating: TokenStream,
    krate: &TokenStream,
) -> TokenStream {
    quote! {
        fn encoded_len(&self) -> ::core::result::Result<usize, #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            if ::core::matches!(#layout, ::core::result::Result::Ok(::core::option::Option::Some(_))) {
                #classify
                if !__ssz_slow {
                    return #krate::encoding::finish_fixed_total(__ssz_total, __ssz_oversize);
                }
            }
            #length
        }
        fn encode_into(&self, output: &mut [u8]) -> ::core::result::Result<(), #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            if ::core::matches!(#layout, ::core::result::Result::Ok(::core::option::Option::Some(_))) {
                #classify
                if !__ssz_slow {
                    let __ssz_total = #krate::encoding::finish_fixed_total(__ssz_total, __ssz_oversize)?;
                    #krate::encoding::check_length(output, __ssz_total)?;
                    let mut __ssz_rest = output;
                    #writes
                    return if __ssz_rest.is_empty() {
                        ::core::result::Result::Ok(())
                    } else {
                        ::core::result::Result::Err(#krate::error::CodecError::InvalidLength)
                    };
                }
            }
            #write
        }
        #allocating
    }
}

fn slow_len_body(fields: &[Field], krate: &TokenStream) -> TokenStream {
    let sizing = fields.iter().map(|field| {
        let name = &field.ident;
        quote!(size.push(&self.#name)?;)
    });
    quote! {
        let mut size = #krate::encoding::ContainerSize::default();
        #(#sizing)*
        size.finish().map(|(_, total)| total)
    }
}

fn slow_into_body(fields: &[Field], krate: &TokenStream) -> TokenStream {
    let prepare = fields.iter().enumerate().map(|(index, field)| {
        let name = &field.ident;
        let local = format_ident!("__ssz_size_{}", index);
        quote!(let #local = size.push(&self.#name)?;)
    });
    let writes = fields.iter().enumerate().map(|(index, field)| {
        let name = &field.ident;
        let local = format_ident!("__ssz_size_{}", index);
        quote!(writer.push(&self.#name, #local.ok_or(#krate::error::CodecError::NativeOutputOverflow)?)?;)
    });
    quote! {
        let mut size = #krate::encoding::ContainerSize::default();
        #(#prepare)*
        let (leading, total) = size.finish()?;
        #krate::encoding::check_length(output, total)?;
        let mut writer = #krate::encoding::ContainerWriter::new(output, leading)?;
        #(#writes)*
        writer.finish()
    }
}

fn slow_encode(fields: &[Field], krate: &TokenStream) -> TokenStream {
    let allocating = quote! {
        fn encode(&self) -> ::core::result::Result<::std::vec::Vec<u8>, #krate::error::CodecError> {
            let mut output = ::std::vec::Vec::new();
            self.encode_append(&mut output)?;
            ::core::result::Result::Ok(output)
        }
    };
    let length = slow_len_body(fields, krate);
    let write = slow_into_body(fields, krate);
    quote! {
        #allocating
        fn encoded_len(&self) -> ::core::result::Result<usize, #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            #length
        }
        fn encode_into(&self, output: &mut [u8]) -> ::core::result::Result<(), #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            #write
        }
    }
}

fn trusted_fixed_layout(fields: &[Field], progressive: bool, krate: &TokenStream) -> TokenStream {
    if progressive {
        return quote!(false);
    }
    let types: Vec<_> = fields.iter().map(|field| &field.ty).collect();
    let layout = const_fixed_layout(&types, krate);
    let trusted = types
        .iter()
        .map(|ty| quote!(<#ty as #krate::codec::Ssz>::TRUSTED_FIXED_LAYOUT));
    quote!(
        ::core::matches!(#layout, ::core::result::Result::Ok(::core::option::Option::Some(_)))
            #(&& #trusted)*
    )
}

fn write_declared_fixed(fields: &[Field], krate: &TokenStream) -> TokenStream {
    let writes = fields.iter().enumerate().map(|(index, field)| {
        let name = &field.ident;
        quote! {
            let ::core::option::Option::Some((__ssz_slot, __ssz_next)) =
                __ssz_rest.split_at_mut_checked(__ssz_widths[#index]) else {
                    return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
                };
            __ssz_rest = __ssz_next;
            #krate::codec::Ssz::encode_into(&self.#name, __ssz_slot)?;
        }
    });
    quote! {
        #(#writes)*
        if !__ssz_rest.is_empty() {
            return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
        }
    }
}

fn append_header_field(index: usize, field: &Field, krate: &TokenStream) -> TokenStream {
    let name = &field.ident;
    let ty = &field.ty;
    let local = format_ident!("__ssz_append_size_{}", index);
    quote! {
        match const { #krate::codec::fixed_size::<#ty>() } {
            ::core::result::Result::Ok(::core::option::Option::None) => {
                let offset = u32::try_from(__ssz_body)
                    .map_err(|_| #krate::error::CodecError::SizeOverflow)?;
                output.extend_from_slice(&offset.to_le_bytes());
                __ssz_body += #local.ok_or(#krate::error::CodecError::NativeOutputOverflow)?;
            }
            ::core::result::Result::Ok(::core::option::Option::Some(_))
            | ::core::result::Result::Err(#krate::error::CodecError::SizeOverflow) => {
                #krate::encoding::append_value(
                    output,
                    &self.#name,
                    #local.ok_or(#krate::error::CodecError::NativeOutputOverflow)?,
                )?;
            }
            ::core::result::Result::Err(error) => return ::core::result::Result::Err(error),
        }
    }
}

fn append_body_field(index: usize, field: &Field, krate: &TokenStream) -> TokenStream {
    let name = &field.ident;
    let ty = &field.ty;
    let local = format_ident!("__ssz_append_size_{}", index);
    quote! {
        if ::core::matches!(
            const { #krate::codec::fixed_size::<#ty>() },
            ::core::result::Result::Ok(::core::option::Option::None)
        ) {
            #krate::encoding::append_value(
                output,
                &self.#name,
                #local.ok_or(#krate::error::CodecError::NativeOutputOverflow)?,
            )?;
        }
    }
}

fn append_method(fields: &[Field], krate: &TokenStream) -> TokenStream {
    let prepare = fields.iter().enumerate().map(|(index, field)| {
        let name = &field.ident;
        let local = format_ident!("__ssz_append_size_{}", index);
        quote!(let #local = size.push(&self.#name)?;)
    });
    let headers = fields
        .iter()
        .enumerate()
        .map(|(index, field)| append_header_field(index, field, krate));
    let bodies = fields
        .iter()
        .enumerate()
        .map(|(index, field)| append_body_field(index, field, krate));
    quote! {
        fn encode_append(&self, output: &mut ::std::vec::Vec<u8>) -> ::core::result::Result<(), #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            let mut size = #krate::encoding::ContainerSize::default();
            #(#prepare)*
            let (leading, total) = size.finish()?;
            let start = output.len();
            output.reserve_exact(total);
            let result = (|| {
                let mut __ssz_body = leading;
                #(#headers)*
                #(#bodies)*
                let expected = start.checked_add(total)
                    .ok_or(#krate::error::CodecError::NativeOutputOverflow)?;
                if output.len() != expected {
                    return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
                }
                ::core::result::Result::Ok(())
            })();
            if let ::core::result::Result::Err(error) = result {
                output.truncate(start);
                return ::core::result::Result::Err(error);
            }
            ::core::result::Result::Ok(())
        }
    }
}

fn classify_fixed(fields: &[Field], krate: &TokenStream) -> TokenStream {
    let fields = fields.iter().enumerate().map(|(index, field)| {
        let name = &field.ident;
        let local = format_ident!("__ssz_len_{}", index);
        quote! {
            let #local = match #krate::encoding::fixed_child(&self.#name)? {
                #krate::encoding::FixedChild::Fixed(length) => {
                    __ssz_total = match __ssz_total.checked_add(length as u128) {
                        ::core::option::Option::Some(next) => next,
                        ::core::option::Option::None => {
                            return ::core::result::Result::Err(#krate::error::CodecError::SizeOverflow);
                        }
                    };
                    ::core::option::Option::Some(length)
                }
                #krate::encoding::FixedChild::DeferredOverflow => {
                    __ssz_oversize = true;
                    ::core::option::Option::None
                }
                #krate::encoding::FixedChild::NotFixed => {
                    __ssz_slow = true;
                    ::core::option::Option::None
                }
            };
        }
    });
    quote! {
        let mut __ssz_total = 0_u128;
        let mut __ssz_oversize = false;
        let mut __ssz_slow = false;
        #(#fields)*
    }
}

fn write_fixed(fields: &[Field], krate: &TokenStream) -> TokenStream {
    let writes = fields.iter().enumerate().map(|(index, field)| {
        let name = &field.ident;
        let local = format_ident!("__ssz_len_{}", index);
        quote! {
            let __ssz_width = #local.ok_or(#krate::error::CodecError::InvalidLength)?;
            if __ssz_width > __ssz_rest.len() {
                return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
            }
            let (__ssz_slot, __ssz_next) = __ssz_rest.split_at_mut(__ssz_width);
            __ssz_rest = __ssz_next;
            #krate::codec::Ssz::encode_into(&self.#name, __ssz_slot)?;
        }
    });
    quote!(#(#writes)*)
}

fn bulk_methods(fields: &[Field], krate: &TokenStream, progressive: bool) -> TokenStream {
    if progressive {
        return quote!();
    }
    let types: Vec<_> = fields.iter().map(|field| &field.ty).collect();
    let layout = const_fixed_layout(&types, krate);
    let encode = bulk_encode_method(fields, &layout, krate);
    let decode = bulk_decode_method(fields, &layout, krate);
    quote!(#encode #decode)
}

fn bulk_encode_method(fields: &[Field], layout: &TokenStream, krate: &TokenStream) -> TokenStream {
    let writes = fields
        .iter()
        .enumerate()
        .map(|(index, field)| bulk_encode_field(index, field, krate));
    quote! {
        fn encode_fixed_sequence(
            values: &[Self],
            output: &mut [u8],
        ) -> ::core::option::Option<::core::result::Result<(), #krate::error::CodecError>> {
            let ::core::result::Result::Ok(::core::option::Option::Some((width, __ssz_widths))) = (#layout) else {
                return ::core::option::Option::None;
            };
            ::core::option::Option::Some((|| {
                let expected = width.checked_mul(values.len())
                    .ok_or(#krate::error::CodecError::SizeOverflow)?;
                #krate::encoding::check_length(output, expected)?;
                let mut __ssz_rest = output;
                for __ssz_value in values {
                    let ::core::option::Option::Some((__ssz_item, __ssz_next)) =
                        __ssz_rest.split_at_mut_checked(width) else {
                            return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
                        };
                    __ssz_rest = __ssz_next;
                    let mut __ssz_item_rest = __ssz_item;
                    #(#writes)*
                    if !__ssz_item_rest.is_empty() {
                        return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
                    }
                }
                #krate::encoding::check_length(__ssz_rest, 0)
            })())
        }
    }
}

fn bulk_encode_field(index: usize, field: &Field, krate: &TokenStream) -> TokenStream {
    let name = &field.ident;
    quote! {
        let ::core::option::Option::Some((__ssz_slot, __ssz_field_next)) =
            __ssz_item_rest.split_at_mut_checked(__ssz_widths[#index]) else {
                return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
            };
        __ssz_item_rest = __ssz_field_next;
        #krate::codec::Ssz::encode_into(&__ssz_value.#name, __ssz_slot)?;
    }
}

fn bulk_decode_method(fields: &[Field], layout: &TokenStream, krate: &TokenStream) -> TokenStream {
    let names: Vec<_> = fields.iter().map(|field| &field.ident).collect();
    let reads = fields
        .iter()
        .enumerate()
        .map(|(index, field)| bulk_decode_field(index, field, krate));
    let locals: Vec<_> = (0..fields.len())
        .map(|index| format_ident!("__ssz_bulk_field_{}", index))
        .collect();
    quote! {
        fn decode_fixed_sequence(bytes: &[u8]) -> ::core::result::Result<::std::vec::Vec<Self>, #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            let ::core::result::Result::Ok(::core::option::Option::Some((width, __ssz_widths))) = (#layout) else {
                return #krate::codec::decode_fixed_sequence_default::<Self>(bytes);
            };
            let count = #krate::sequence::fixed_list_count(bytes.len(), width, ::core::option::Option::None)?;
            let mut values = ::std::vec::Vec::with_capacity(count);
            let mut __ssz_sequence_rest = bytes;
            while !__ssz_sequence_rest.is_empty() {
                let ::core::option::Option::Some((__ssz_item, __ssz_next)) =
                    __ssz_sequence_rest.split_at_checked(width) else {
                        return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
                    };
                __ssz_sequence_rest = __ssz_next;
                let mut __ssz_item_rest = __ssz_item;
                #(#reads)*
                values.push(Self { #(#names: #locals,)* });
            }
            ::core::result::Result::Ok(values)
        }
    }
}

fn bulk_decode_field(index: usize, field: &Field, krate: &TokenStream) -> TokenStream {
    let local = format_ident!("__ssz_bulk_field_{}", index);
    let ty = &field.ty;
    quote! {
        let ::core::option::Option::Some((__ssz_slice, __ssz_field_next)) =
            __ssz_item_rest.split_at_checked(__ssz_widths[#index]) else {
                return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
            };
        __ssz_item_rest = __ssz_field_next;
        let #local = <#ty as #krate::codec::Ssz>::decode(__ssz_slice)?;
    }
}

fn root_method(fields: &[Field], mask: &TokenStream, krate: &TokenStream) -> TokenStream {
    let roots = fields.iter().map(|field| {
        let name = &field.ident;
        quote!(#krate::codec::Ssz::hash_tree_root(&self.#name)?)
    });
    quote! {
        fn hash_tree_root(&self) -> ::core::result::Result<[u8;32], #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            #krate::composite::root_fields(&[#(#roots,)*], #mask)
        }
    }
}

fn metadata(
    fields: &[Field],
    progressive: bool,
    krate: &TokenStream,
) -> (TokenStream, TokenStream) {
    let names = fields.iter().map(|field| &field.name);
    let types = fields.iter().map(|field| &field.ty);
    if !progressive {
        return (
            quote!(#krate::type_info::TypeDescriptor::Container(&[#(#names,)*], &[#(<#types as #krate::codec::Ssz>::TYPE,)*])),
            quote!(::core::option::Option::None),
        );
    }
    let mut mask = vec![false; fields.last().expect("nonempty fields").position + 1];
    for field in fields {
        mask[field.position] = true;
    }
    (
        quote!(#krate::type_info::TypeDescriptor::ProgressiveContainer(&[#(#mask,)*], &[#(#names,)*], &[#(<#types as #krate::codec::Ssz>::TYPE,)*])),
        quote!(::core::option::Option::Some(&[#(#mask,)*])),
    )
}

fn decode_method(fields: &[Field], krate: &TokenStream, progressive: bool) -> TokenStream {
    let names: Vec<_> = fields.iter().map(|field| &field.ident).collect();
    let types: Vec<_> = fields.iter().map(|field| &field.ty).collect();
    let fixed = if progressive {
        quote!()
    } else {
        fixed_decode(&names, &types, krate)
    };
    let layout = const_container_layout(&types, krate);
    let values = fields
        .iter()
        .enumerate()
        .map(|(index, field)| decode_field(index, field, krate));
    quote! {
        fn decode(bytes: &[u8]) -> ::core::result::Result<Self, #krate::error::CodecError> {
            const { #krate::codec::validate_type::<Self>() }?;
            #fixed
            let __ssz_ranges = #krate::container::derived_container_ranges(bytes, #layout)?;
            ::core::result::Result::Ok(Self { #(#values,)* })
        }
    }
}

fn fixed_decode(names: &[&syn::Ident], types: &[&syn::Type], krate: &TokenStream) -> TokenStream {
    let layout = const_fixed_layout(types, krate);
    let locals: Vec<_> = (0..names.len())
        .map(|index| format_ident!("__ssz_field_{}", index))
        .collect();
    let reads = locals
        .iter()
        .zip(types)
        .enumerate()
        .map(|(index, (local, ty))| fixed_split(index, local, ty, krate));
    quote! {
        if bytes.len() as u128 >= 4_294_967_296 {
            return ::core::result::Result::Err(#krate::error::CodecError::SizeOverflow);
        }
        match #layout {
            ::core::result::Result::Ok(::core::option::Option::Some((total, __ssz_widths))) => {
                if bytes.len() != total {
                    return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
                }
                let mut __ssz_rest = bytes;
                #(#reads)*
                return ::core::result::Result::Ok(Self {
                    #(#names: #locals,)*
                });
            }
            ::core::result::Result::Ok(::core::option::Option::None) => {}
            ::core::result::Result::Err(kind) => return ::core::result::Result::Err(kind),
        }
    }
}

fn const_fixed_layout(types: &[&syn::Type], krate: &TokenStream) -> TokenStream {
    let calls = types
        .iter()
        .map(|ty| quote!(#krate::codec::fixed_size::<#ty>()));
    quote! {
        const { #krate::codec::fixed_layout([#(#calls,)*]) }
    }
}

fn const_container_layout(types: &[&syn::Type], krate: &TokenStream) -> TokenStream {
    let calls = types
        .iter()
        .map(|ty| quote!(#krate::codec::fixed_size::<#ty>()));
    quote! {
        const { #krate::container::derived_container_layout([#(#calls,)*]) }
    }
}

fn fixed_split(
    index: usize,
    local: &syn::Ident,
    ty: &syn::Type,
    krate: &TokenStream,
) -> TokenStream {
    quote! {
        let ::core::option::Option::Some((__ssz_slice, __ssz_next)) =
            __ssz_rest.split_at_checked(__ssz_widths[#index]) else {
            return ::core::result::Result::Err(#krate::error::CodecError::InvalidLength);
        };
        __ssz_rest = __ssz_next;
        let #local = <#ty as #krate::codec::Ssz>::decode(__ssz_slice)?;
    }
}

fn decode_field(index: usize, field: &Field, krate: &TokenStream) -> TokenStream {
    let name = &field.ident;
    let ty = &field.ty;
    quote! {
        #name: {
            let __ssz_range = __ssz_ranges[#index];
            <#ty as #krate::codec::Ssz>::decode(&bytes[__ssz_range.start..__ssz_range.end])?
        }
    }
}
