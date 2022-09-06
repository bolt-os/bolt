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
use quote::{__private::TokenStream as QuoteStream, format_ident, quote, ToTokens};
use syn::{
    braced, parse::Parse, parse_macro_input, punctuated::Punctuated, Attribute, Expr, Ident, Token,
    Type, Visibility,
};

#[derive(Clone)]
struct Bitstruct {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    ty: Type,
    fields: Punctuated<Bitfield, Token![;]>,
}

impl Parse for Bitstruct {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        let parse_buf;
        braced!(parse_buf in input);
        let fields = parse_buf.parse_terminated(Bitfield::parse)?;

        Ok(Bitstruct {
            attrs,
            vis,
            ident,
            ty,
            fields,
        })
    }
}

struct BoundType {
    at_token: Token![@],
}

#[derive(Clone)]
struct Bitfield {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    bit_width: Expr,
    bit_shift: Expr,
    ty: Option<Type>,
}

fn foo(expr: Expr) {
    match expr {
        Expr::Lit(lit) => {
            let syn::ExprLit { attrs, lit } = lit;
            match lit {
                syn::Lit::Int(int) => {
                    // int.
                }
                syn::Lit::Str(_) => todo!(),
                syn::Lit::ByteStr(_) => todo!(),
                syn::Lit::Byte(_) => todo!(),
                syn::Lit::Char(_) => todo!(),
                syn::Lit::Float(_) => todo!(),
                syn::Lit::Bool(_) => todo!(),
                syn::Lit::Verbatim(_) => todo!(),
            }
        }
        Expr::Array(_) => todo!(),
        Expr::Assign(_) => todo!(),
        Expr::AssignOp(_) => todo!(),
        Expr::Async(_) => todo!(),
        Expr::Await(_) => todo!(),
        Expr::Binary(_) => todo!(),
        Expr::Block(_) => todo!(),
        Expr::Box(_) => todo!(),
        Expr::Break(_) => todo!(),
        Expr::Call(_) => todo!(),
        Expr::Cast(_) => todo!(),
        Expr::Closure(_) => todo!(),
        Expr::Continue(_) => todo!(),
        Expr::Field(_) => todo!(),
        Expr::ForLoop(_) => todo!(),
        Expr::Group(_) => todo!(),
        Expr::If(_) => todo!(),
        Expr::Index(_) => todo!(),
        Expr::Let(_) => todo!(),
        Expr::Loop(_) => todo!(),
        Expr::Macro(_) => todo!(),
        Expr::Match(_) => todo!(),
        Expr::MethodCall(_) => todo!(),
        Expr::Paren(_) => todo!(),
        Expr::Path(_) => todo!(),
        Expr::Range(_) => todo!(),
        Expr::Reference(_) => todo!(),
        Expr::Unary(_) => todo!(),
        Expr::Verbatim(_) => todo!(),
        _ => todo!(),
    }
}

impl Parse for Bitfield {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![const]>()?;
        let ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let bit_width = input.parse::<Expr>()?;
        foo(bit_width.clone());
        input.parse::<Token![,]>()?;
        let bit_shift = input.parse()?;
        let ty = if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Bitfield {
            attrs,
            vis,
            ident,
            bit_width,
            bit_shift,
            ty,
        })
    }
}

pub fn bitstruct(ts: TokenStream) -> TokenStream {
    let bts = parse_macro_input!(ts as Bitstruct);

    let Bitstruct {
        attrs,
        vis,
        ident,
        ty,
        fields,
    } = bts.clone();

    let mut output = quote!();

    for attr in attrs {
        attr.to_tokens(&mut output);
    }
    output.extend(quote! {
        #[repr(transparent)]
        #[derive(Clone, Copy, Eq, Hash, PartialEq)]
        #vis struct #ident {
            bits: #ty,
        }
    });

    let mut s_consts = quote!();
    let mut s_impls = quote!();
    let mut all_fields = Punctuated::<_, Token![|]>::new();
    let sident = ident;
    let stype = ty;

    for field in fields {
        let Bitfield {
            attrs,
            vis,
            ident,
            bit_width,
            bit_shift,
            ty,
        } = field;

        s_consts.extend(attrs.iter().map(ToTokens::to_token_stream));
        s_consts.extend(quote! {
            #[allow(clippy::identity_op)]
            #vis const #ident: #sident = #sident { bits: ((1 << #bit_width) - 1) << #bit_shift };
        });

        if let Some(bound_type) = ty {
            let namel = ident.to_string().to_lowercase();
            let getter = format_ident!("{namel}", span = ident.span());
            let setter = format_ident!("set_{namel}", span = ident.span());

            // match &bound_type {
            //     Type::Array(_) => todo!(),
            //     Type::BareFn(_) => todo!(),
            //     Type::Group(_) => todo!(),
            //     Type::ImplTrait(_) => todo!(),
            //     Type::Infer(_) => todo!(),
            //     Type::Macro(_) => todo!(),
            //     Type::Never(_) => todo!(),
            //     Type::Paren(_) => todo!(),
            //     Type::Path(_) => todo!(),
            //     Type::Ptr(_) => todo!(),
            //     Type::Reference(_) => todo!(),
            //     Type::Slice(_) => todo!(),
            //     Type::TraitObject(_) => todo!(),
            //     Type::Tuple(_) => todo!(),
            //     Type::Verbatim(_) => todo!(),
            //     _ => todo!(),
            // }

            s_impls.extend(quote! {
                #vis const fn #getter(self) -> #bound_type {
                    <#bound_type as ::core::convert::From<#stype>>::from(
                        self.bits >> #bit_shift & ((1 << #bit_width) - 1)
                    )
                }

                #vis const fn #setter(&mut self, value: #bound_type) -> &mut Self {
                    self.bits |= <#stype as ::core::convert::From<#bound_type>>::from(value)
                                & ((1 << #bit_width) - 1) << #bit_shift;
                    self
                }
            });
        }

        all_fields.push(quote!(#sident::#ident));
    }

    s_impls.extend(quote! {
        pub const fn bits(self) -> #stype {
            self.bits
        }
    });

    let all_fields_init = if all_fields.is_empty() {
        quote!(Self::empty())
    } else {
        all_fields.to_token_stream()
    };
    s_impls.extend(quote! {
        const ALL: #sident = #all_fields_init;
    });

    output.extend(quote! {
        impl #sident {
            #s_consts
        }
        impl #sident {
            #s_impls
        }
    });

    output.extend(do_impls(bts));

    output.into()
}

fn do_impls(bts: Bitstruct) -> QuoteStream {
    let Bitstruct { ident, ty, .. } = bts;

    quote! {
        impl #ident {
            #[doc = concat!("Create a new `", stringify!(#ident), "`")]
            ///
            /// # Panics
            ///
            /// This function panics if `bits` contains any undefined `1` bits.
            pub const fn new(bits: #ty) -> #ident {
                assert!(Self::check_bits(bits));
                unsafe { Self::new_unchecked(bits) }
            }

            #[doc = concat!("Create a new `", stringify!(#ident), "`, masking undefined bits")]
            pub const fn new_masked(bits: #ty) -> #ident {
                unsafe { Self::new_unchecked(bits & Self::ALL.bits) }
            }

            #[doc = concat!("Create a new `", stringify!(#ident), "` without performing any checks")]
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `bits` does not contain an illegal bit pattern.
            pub const unsafe fn new_unchecked(bits: #ty) -> #ident {
                Self { bits }
            }

            pub const fn check_bits(bits: #ty) -> bool {
                bits & !Self::ALL.bits == 0
            }

            pub const fn all() -> #ident {
                Self::ALL
            }

            pub const fn empty() -> #ident {
                Self::new(0)
            }

            pub const fn contains(self, other: Self) -> bool {
                self.bits & other.bits == other.bits
            }
        }

        impl const ::core::ops::BitOr for #ident {
            type Output = #ident;

            fn bitor(self, rhs: #ident) -> #ident {
                Self { bits: self.bits | rhs.bits }
            }
        }
    }
}
