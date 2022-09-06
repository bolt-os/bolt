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
use quote::{format_ident, quote};
use syn::{parse_macro_input, Item};

pub fn test(_ats: TokenStream, ts: TokenStream) -> TokenStream {
    let output = match parse_macro_input!(ts as _) {
        Item::Const(_) => todo!(),
        Item::Fn(func) => {
            let func_name = func.sig.ident.clone();
            let test_name = format_ident!("bolt_test_{}", func_name);
            let should_panic = func
                .attrs
                .iter()
                .any(|attr| attr.path.is_ident("should_panic"));

            let mut s_test = quote! {
                #[test_case]
                #[allow(non_upper_case_globals)]
                static #test_name: ::bolt::test::Test = bolt::test::Test::new(#func_name)
            };

            if should_panic {
                s_test.extend(quote!(.should_panic()));
            }

            quote! {
                #func
                #s_test;
            }
        }
        Item::Struct(_) => todo!(),
        _ => unimplemented!(),
    };

    output.into()
}
