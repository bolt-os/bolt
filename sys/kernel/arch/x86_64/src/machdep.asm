; Copyright (c) 2022 xvanc <xvancm@gmail.com>
;
; Redistribution and use in source and binary forms, with or without modification,
; are permitted provided that the following conditions are met:
;
; 1. Redistributions of source code must retain the above copyright notice,
;    this list of conditions and the following disclaimer.
;
; 2. Redistributions in binary form must reproduce the above copyright notice,
;    this list of conditions and the following disclaimer in the documentation
;    and/or other materials provided with the distribution.
;
; 3. Neither the name of the copyright holder nor the names of its contributors
;    may be used to endorse or promote products derived from this software without
;    specific prior written permission.
;
; THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY
; EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
; OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
; IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
; INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
; PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
; INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
; LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
; OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
;
; SPDX-License-Identifier: BSD-3-Clause

bits    64
default rel

extern tf_cs, trap_common

%define vector_has_error(v)     ((v) == 8 || ((v) >= 10 && (v) <= 14) || (v) == 17 || (v) == 21)
%define NUM_VECTORS             256

; Create a stub for each vector so we can get the vector number.
section .text.trap_stubs progbits alloc exec
align   16
%assign v 0
%rep    NUM_VECTORS

        trap%+v:
            %if !vector_has_error(v)
                push    0
            %endif
                push    v
                jmp     trap_common

%assign v v+1
%endrep

; Startup code will use this array of pointers to initialize the IDT.
section .data.rel.ro.trap_stubs progbits alloc write
global  trap_stubs:data (trap_stubs.end - trap_stubs)
align   16
trap_stubs:
%assign v 0
%rep 256
        dq      trap%+v
%assign v v+1
%endrep
.end:
