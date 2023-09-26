/*
 * Copyright (c) 2023 xvanc <xvancm@gmail.com>
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

use alloc::ffi::CString;
use core::{
    ffi::{c_char, c_int, c_uint, c_void, CStr},
    fmt,
    mem::MaybeUninit,
    ptr::{self, NonNull},
};

use super::ACPI_ROOT;
use crate::vm::{PhysAddr, VirtAddr};

#[allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]
pub mod lai_sys {
    include!(concat!(env!("OUT_DIR"), "/lai.rs"));
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    OutOfMemory      = 1,
    TypeMismatch     = 2,
    NoSuchNode       = 3,
    OutOfBounds      = 4,
    ExecutionFailure = 5,
    IllegalArguments = 6,
    UnexpectedResult = 7,
    EndReached       = 8,
    Unsupported      = 9,
}

trait ToResult {
    fn to_result(self) -> Result<()>;
}

impl ToResult for lai_sys::lai_api_error_t {
    fn to_result(self) -> Result<()> {
        match self {
            lai_sys::lai_api_error_LAI_ERROR_NONE => Ok(()),
            x @ 1..=9 => Err(unsafe { core::mem::transmute(x) }),
            _ => panic!(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[repr(transparent)]
pub struct Variable {
    inner: lai_sys::lai_variable_t,
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Variable")
            .field("value", &self.value())
            .finish()
    }
}

#[derive(Debug)]
pub enum Value<'a> {
    Integer(u64),
    String(&'a str),
    Buffer(&'a [u8]),
    Package(&'a [Variable]),

    Handle,
    LazyHandle,

    ArgRef,
    LocalRef,
    NodeRef,

    StringIndex,
    BufferIndex,
    PackageIndex,
}

impl Variable {
    fn new(f: impl FnOnce(*mut lai_sys::lai_variable_t) -> Result<()>) -> Result<Self> {
        let mut uninit = MaybeUninit::zeroed();
        f(uninit.as_mut_ptr())?;
        Ok(Self {
            inner: unsafe { uninit.assume_init() },
        })
    }

    pub fn value(&self) -> Value {
        match self.inner.type_ as _ {
            lai_sys::LAI_INTEGER => Value::Integer(self.inner.integer),
            lai_sys::LAI_STRING => {
                let data =
                    unsafe { CStr::from_ptr((*self.inner.__bindgen_anon_1.string_ptr).content) };
                Value::String(data.to_str().unwrap())
            }
            lai_sys::LAI_BUFFER => unsafe {
                let data = (*self.inner.__bindgen_anon_1.buffer_ptr).content;
                let len = (*self.inner.__bindgen_anon_1.buffer_ptr).size;
                Value::Buffer(core::slice::from_raw_parts(data, len))
            },
            lai_sys::LAI_PACKAGE => unsafe {
                let data = (*self.inner.__bindgen_anon_1.pkg_ptr).elems;
                let len = (*self.inner.__bindgen_anon_1.pkg_ptr).size as usize;
                // NOTE: The cast is safe because `Variable` is `#[repr(transparent)]` over
                // the inner `lai_variable_t`.
                Value::Package(core::slice::from_raw_parts(data.cast(), len))
            },
            x => todo!("Value::{x}"),
        }
    }

    pub fn as_str(&mut self) -> Option<&str> {
        if self.inner.type_ == lai_sys::LAI_STRING as i32 {
            let data = unsafe { CStr::from_ptr((*self.inner.__bindgen_anon_1.string_ptr).content) };
            data.to_str().ok()
        } else {
            None
        }
    }
}

impl Drop for Variable {
    fn drop(&mut self) {
        unsafe { lai_sys::lai_var_finalize(&mut self.inner) };
    }
}

#[repr(transparent)]
pub struct State {
    inner: lai_sys::lai_state_t,
}

pub fn with_state<F, T>(f: F) -> T
where
    F: FnOnce(&mut State) -> T,
{
    let mut state = MaybeUninit::<State>::uninit();
    unsafe {
        lai_sys::lai_init_state(state.as_mut_ptr().cast());
        let result = f(state.assume_init_mut());
        lai_sys::lai_finalize_state(state.as_mut_ptr().cast());
        result
    }
}

impl State {
    pub fn eval(&mut self, handle: &mut NsNode) -> Result<Variable> {
        Variable::new(|ptr| unsafe {
            lai_sys::lai_eval(ptr, handle.inner.as_ptr(), &mut self.inner).to_result()
        })
    }
}

pub struct NsNode {
    inner: NonNull<lai_sys::lai_nsnode_t>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Null,
    Root,
    Evaluatable,
    Device,
    Mutex,
    Processor,
    ThermalZone,
    Event,
    PowerResource,
    OpRegion,
}

impl NsNode {
    fn new(inner: *mut lai_sys::lai_nsnode_t) -> Option<Self> {
        NonNull::new(inner).map(|inner| Self { inner })
    }

    pub fn name(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                (*self.inner.as_ptr()).name.as_ptr().cast(),
                4,
            ))
        }
    }

    pub fn kind(&self) -> NodeKind {
        let raw = unsafe { lai_sys::lai_ns_get_node_type(self.inner.as_ptr()) };
        unsafe { core::mem::transmute(raw) }
    }

    pub fn root() -> NsNode {
        let inner = unsafe { lai_sys::lai_ns_get_root() };
        Self {
            inner: NonNull::new(inner).unwrap(),
        }
    }

    pub fn parent(&mut self) -> Option<NsNode> {
        let inner = unsafe { lai_sys::lai_ns_get_parent(self.inner.as_ptr()) };
        Self::new(inner)
    }

    pub fn child(&mut self, name: &str) -> Option<NsNode> {
        let name = CString::new(name).unwrap();
        let inner = unsafe { lai_sys::lai_ns_get_child(self.inner.as_ptr(), name.as_ptr()) };
        Self::new(inner)
    }

    pub fn children(&mut self) -> NsChildIter {
        NsChildIter::new(Self { inner: self.inner })
    }

    pub fn eval(&mut self) -> Result<Variable> {
        with_state(|state| self.eval_with(state))
    }

    pub fn eval_with(&mut self, state: &mut State) -> Result<Variable> {
        state.eval(self)
    }

    pub fn resolve_path(&mut self, path: &str) -> Option<NsNode> {
        let path = CString::new(path).unwrap();
        let inner = unsafe { lai_sys::lai_resolve_path(self.inner.as_ptr(), path.as_ptr()) };
        Self::new(inner)
    }

    pub fn resolve_search(&mut self, path: &str) -> Option<NsNode> {
        let path = CString::new(path).unwrap();
        let inner = unsafe { lai_sys::lai_resolve_path(self.inner.as_ptr(), path.as_ptr()) };
        Self::new(inner)
    }
}

pub fn resolve_path(path: &str) -> Option<NsNode> {
    let path = CString::new(path).unwrap();
    let inner = unsafe { lai_sys::lai_resolve_path(ptr::null_mut(), path.as_ptr()) };
    NsNode::new(inner)
}

pub struct NsIter {
    inner: lai_sys::lai_ns_iterator,
}

impl NsIter {
    pub fn new() -> NsIter {
        // lai_sys::lai_initialize_ns_iterator();
        Self {
            inner: lai_sys::lai_ns_iterator { i: 0 },
        }
    }
}

impl Iterator for NsIter {
    type Item = NsNode;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = unsafe { lai_sys::lai_ns_iterate(&mut self.inner) };
        NsNode::new(inner)
    }
}

pub struct NsChildIter {
    inner: lai_sys::lai_ns_child_iterator,
}

impl NsChildIter {
    pub fn new(node: NsNode) -> NsChildIter {
        Self {
            inner: lai_sys::lai_ns_child_iterator {
                i:      0,
                parent: node.inner.as_ptr(),
            },
        }
    }
}

impl Iterator for NsChildIter {
    type Item = NsNode;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = unsafe { lai_sys::lai_ns_child_iterate(&mut self.inner) };
        NsNode::new(inner)
    }
}

/*






*/

#[no_mangle]
unsafe extern fn laihost_log(level: c_int, msg: *const c_char) {
    let msg = CStr::from_ptr(msg).to_string_lossy();
    match level as _ {
        lai_sys::LAI_WARN_LOG => log::warn!("{msg}"),
        lai_sys::LAI_DEBUG_LOG => log::debug!("{msg}"),
        _ => log::error!("laihost_log: invalid level {level}"),
    }
}

#[no_mangle]
unsafe extern fn laihost_panic(msg: *const c_char) {
    let msg = CStr::from_ptr(msg).to_str().unwrap();
    panic!("lai error: {msg}");
}

#[no_mangle]
unsafe extern fn laihost_malloc(size: usize) -> *mut c_void {
    alloc::alloc::alloc_zeroed(core::alloc::Layout::from_size_align(size, 16).unwrap()).cast()
}

#[no_mangle]
unsafe extern fn laihost_realloc(
    ptr: *mut c_void,
    new_size: usize,
    old_size: usize,
) -> *mut c_void {
    if ptr.is_null() {
        return laihost_malloc(new_size);
    }
    alloc::alloc::realloc(
        ptr.cast(),
        core::alloc::Layout::from_size_align(old_size, 16).unwrap(),
        new_size,
    )
    .cast()
}

#[no_mangle]
unsafe extern fn laihost_free(ptr: *mut c_void, size: usize) {
    if ptr.is_null() {
        return;
    }
    alloc::alloc::dealloc(
        ptr.cast(),
        core::alloc::Layout::from_size_align(size, 16).unwrap(),
    );
}

#[no_mangle]
unsafe extern fn laihost_map(addr: PhysAddr, _size: usize) -> VirtAddr {
    addr.to_virt()
}

#[no_mangle]
unsafe extern fn laihost_unmap(_addr: VirtAddr, _size: usize) {}

#[no_mangle]
unsafe extern fn laihost_scan(sig: *const c_char, index: usize) -> VirtAddr {
    let root = ACPI_ROOT.read();

    let signature = sig.cast::<[u8; 4]>().read();
    if signature == *b"DSDT" {
        if let Some(fadt) = root.get_table::<::acpi::sdt::fadt::Fadt>(0) {
            return if (*fadt).header.revision >= 2 {
                PhysAddr((*fadt).x_dsdt as usize).to_virt()
            } else {
                PhysAddr((*fadt).dsdt as usize).to_virt()
            };
        }
        return VirtAddr(0);
    }

    root.get_table_by_signature(signature, index)
        .map_or(VirtAddr(0), |ptr| VirtAddr(ptr.addr()))
}

#[cfg(target_arch = "riscv64")]
mod port_io {
    #[no_mangle]
    unsafe extern fn laihost_outb(port: u16, val: u8) {
        (port as *mut u8).write_volatile(val);
    }
    #[no_mangle]
    unsafe extern fn laihost_outw(port: u16, val: u16) {
        (port as *mut u16).write_volatile(val);
    }
    #[no_mangle]
    unsafe extern fn laihost_outd(port: u16, val: u32) {
        (port as *mut u32).write_volatile(val);
    }

    #[no_mangle]
    unsafe extern fn laihost_inb(port: u16) -> u8 {
        (port as *const u8).read_volatile()
    }
    #[no_mangle]
    unsafe extern fn laihost_inw(port: u16) -> u16 {
        (port as *const u16).read_volatile()
    }
    #[no_mangle]
    unsafe extern fn laihost_ind(port: u16) -> u32 {
        (port as *const u32).read_volatile()
    }
}

#[no_mangle]
unsafe extern fn laihost_sleep(ms: u64) {
    for _ in 0..1000 * ms {
        core::hint::spin_loop();
    }

    // todo!("laihost_sleep({ms})");
}
// #[no_mangle]
// unsafe extern fn laihost_timer() -> u64 {
//     todo!()
// }

#[no_mangle]
unsafe extern fn laihost_handle_amldebug(_var: *mut lai_sys::lai_variable_t) {
    todo!()
}
#[no_mangle]
unsafe extern fn laihost_handle_global_notify(_node: *mut lai_sys::lai_nsnode_t, _code: c_int) {
    todo!()
}

#[no_mangle]
unsafe extern fn laihost_sync_wait(
    _sync: *mut lai_sys::lai_sync_state,
    _val: c_uint,
    _timeout: i64,
) -> c_int {
    todo!()
}
#[no_mangle]
unsafe extern fn laihost_sync_wake(_sync: *mut lai_sys::lai_sync_state) {
    todo!()
}
