/*
 * Copyright (c) 2022 xvanc <xvancm@gmail.com>
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its contributors
 *    may be used to endorse or promote products derived from this software without
 *    specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY
 * EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 */

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Meta, NestedMeta};

// TODO: Allow overriding the symbols we export, default is `StructName.field_name`.
//      We want to be able to export something like `name_size` and `prefix_field_name`, so
//      instead of `TrapFrame_size` and `TrapFrame.<field_name>` we can have `trapf_size` and
//      `tf_<field_name>`
//      `#[asm_export(name = "trapf", prefix = "tf_")]`
pub fn asm_export(ats: TokenStream, ts: TokenStream) -> TokenStream {
    let args = parse_macro_input!(ats as syn::AttributeArgs);

    let mut prefix = None;

    for arg in args {
        match arg {
            NestedMeta::Meta(Meta::NameValue(arg)) => {
                if arg.path.is_ident("prefix") {
                    match arg.lit {
                        syn::Lit::Str(ref p) => prefix = Some((arg.span(), p.clone())),
                        _ => todo!(),
                    }
                } else {
                    todo!();
                }
            }
            _ => todo!(),
        }
    }

    let mut symbols = vec![];

    let output = match parse_macro_input!(ts as syn::Item) {
        syn::Item::Enum(_) => todo!(),
        syn::Item::Const(cons) => {
            if let Some((span, _)) = prefix {
                return syn::Error::new(span, "The `prefix` option is invalid on constants.")
                    .into_compile_error()
                    .into();
            }

            let cons_name = cons.ident.clone();
            let inline_asm = syn::LitStr::new(
                &format!(".global {cons_name}\n.set {cons_name}, {{}}"),
                cons.span(),
            );

            symbols.push(cons_name.to_string());

            quote! {
                #cons
                ::core::arch::global_asm!(#inline_asm, const #cons_name);
            }
        }
        syn::Item::Struct(struc) => {
            let struc_name = struc.ident.clone();
            let sym = format!("{struc_name}_size");
            let inline_asm =
                syn::LitStr::new(&format!(".global {sym}\n.set {sym}, {{}}"), struc.span());

            symbols.push(sym);

            let mut output = quote! {
                #struc
                ::core::arch::global_asm!(#inline_asm, const ::core::mem::size_of::<#struc_name>());
            };

            match struc.fields {
                syn::Fields::Named(fields) => {
                    for field in fields.named {
                        let field_name = field.ident.clone().unwrap();
                        let sym = if let Some((_, ref prefix)) = prefix {
                            format!("{}{field_name}", prefix.value())
                        } else {
                            format!("{struc_name}.{field_name}")
                        };
                        let inline_asm = syn::LitStr::new(
                            &format!(".global {sym}\n.set {sym}, {{}}"),
                            field.span(),
                        );

                        symbols.push(sym);

                        output.extend(quote! {
                            ::core::arch::global_asm!(
                                #inline_asm,
                                const offset_of!(#struc_name, #field_name)
                            );
                        });
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    for (index, field) in fields.unnamed.into_iter().enumerate() {
                        let index = syn::LitInt::new(&format!("{index}"), field.span());
                        let sym = format!("{struc_name}.{index}");
                        let inline_asm = syn::LitStr::new(
                            &format!(".global {sym}\n.set {sym}, {{}}"),
                            field.span(),
                        );

                        symbols.push(sym);

                        output.extend(quote! {
                            ::core::arch::global_asm!(
                                #inline_asm,
                                const offset_of!(#struc_name, #index)
                            );
                        });
                    }
                }
                syn::Fields::Unit => panic!("nothing to export for unit struct"),
            }

            output
        }
        syn::Item::Union(unio) => {
            let unio_name = unio.ident.clone();
            let sym = format!("{unio_name}_size");
            let inline_asm =
                syn::LitStr::new(&format!(".global {sym}\n.set {sym}, {{}}"), unio.span());

            symbols.push(sym);

            let mut output = quote! {
                #unio
                ::core::arch::global_asm!(#inline_asm, const core::mem::size_of::<#unio_name>());
            };

            for field in unio.fields.named {
                let field_name = field.ident.clone().unwrap();
                let sym = format!("{unio_name}.{field_name}");
                let inline_asm =
                    syn::LitStr::new(&format!(".global {sym}\n.set {sym}, {{}}"), field.span());

                symbols.push(sym);

                output.extend(quote! {
                    ::core::arch::global_asm!(
                        #inline_asm,
                        const offset_of!(#unio_name, #field_name)
                    );
                });
            }

            output
        }
        _ => unimplemented!(),
    };

    output.into()
}
