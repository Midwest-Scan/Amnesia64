//! Rust-owned `cFileSearcher`; native string objects stay in its C++ facade.
#![allow(clippy::missing_safety_doc)]

use crate::system::{HplStr, HplWChar, HplWStr, HplWStrOut};
use crate::types::raw_slice;
use hpl_resources::file_searcher::{FileSearcher, SearchHost};
use std::ffi::c_void;

pub struct HplFileSearcher { _private: [u8; 0] }

/// Native dependencies of AddDirectory. Callbacks catch and save exceptions;
/// false stops Rust, then C++ rethrows. Listing emits one string per entry;
/// full_path emits one resolved path. Input views expire on callback return.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplFileSearchHost {
    pub ctx: *mut c_void,
    pub list: unsafe extern "C" fn(ctx: *mut c_void, path: HplWStr, folders: bool, out: HplWStrOut) -> bool,
    pub full_path: unsafe extern "C" fn(ctx: *mut c_void, path: HplWStr, out: HplWStrOut) -> bool,
    pub retain_path: unsafe extern "C" fn(ctx: *mut c_void, path: HplWStr, index: *mut usize) -> bool,
    /// Nonthrowing view of a stable native string; read before another callback.
    pub retained_path: unsafe extern "C" fn(ctx: *mut c_void, index: usize) -> HplWStr,
}

fn view(path: &[HplWChar]) -> HplWStr { HplWStr { ptr: path.as_ptr(), len: path.len() } }

unsafe extern "C" fn push(ctx: *mut c_void, ptr: *const HplWChar, len: usize) {
    unsafe { &mut *ctx.cast::<Vec<Vec<HplWChar>>>() }.push(unsafe { raw_slice(ptr, len) }.to_vec());
}

impl SearchHost for HplFileSearchHost {
    fn list(&mut self, path: &[HplWChar], folders: bool) -> Option<Vec<Vec<HplWChar>>> {
        let mut values = Vec::<Vec<HplWChar>>::new();
        let out = HplWStrOut { sink: Some(push), ctx: (&mut values as *mut Vec<Vec<HplWChar>>).cast() };
        unsafe { (self.list)(self.ctx, view(path), folders, out) }.then_some(values)
    }
    fn full_path(&mut self, path: &[HplWChar]) -> Option<Vec<HplWChar>> {
        let mut values = Vec::<Vec<HplWChar>>::new();
        let out = HplWStrOut { sink: Some(push), ctx: (&mut values as *mut Vec<Vec<HplWChar>>).cast() };
        unsafe { (self.full_path)(self.ctx, view(path), out) }.then(|| values.pop().unwrap_or_default())
    }
    fn retain_path(&mut self, path: &[HplWChar]) -> Option<usize> {
        let mut index = 0;
        unsafe { (self.retain_path)(self.ctx, view(path), &mut index) }.then_some(index)
    }
    fn retained_path(&mut self, index: usize) -> Vec<HplWChar> {
        let path = unsafe { (self.retained_path)(self.ctx, index) };
        unsafe { raw_slice(path.ptr, path.len) }.to_vec()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_resources_file_searcher_new() -> *mut HplFileSearcher {
    Box::into_raw(Box::new(FileSearcher::default())).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_resources_file_searcher_clone(source: *const HplFileSearcher) -> *mut HplFileSearcher {
    Box::into_raw(Box::new(unsafe { &*source.cast::<FileSearcher>() }.clone())).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_resources_file_searcher_assign(target: *mut HplFileSearcher, source: *const HplFileSearcher) {
    let copy = unsafe { &*source.cast::<FileSearcher>() }.clone();
    unsafe { &mut *target.cast::<FileSearcher>() }.assign(copy);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_resources_file_searcher_free(searcher: *mut HplFileSearcher) {
    if searcher.is_null() { return; }
    drop(unsafe { Box::from_raw(searcher.cast::<FileSearcher>()) });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_resources_file_searcher_clear(searcher: *mut HplFileSearcher) {
    unsafe { &mut *searcher.cast::<FileSearcher>() }.clear();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_resources_file_searcher_add_directory(
    searcher: *mut HplFileSearcher, path: HplWStr, recurse: bool, host: *const HplFileSearchHost,
) -> bool {
    // Own the entry path before callbacks may clear the native result pool.
    let path = unsafe { raw_slice(path.ptr, path.len) }.to_vec();
    unsafe { FileSearcher::add_directory(searcher.cast(), &path, recurse, &mut host.read()) }
}

/// SIZE_MAX selects the facade's persistent msNull; only write count when the
/// original did. Read all input before the nullable output (possible alias).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_resources_file_searcher_get_file_path(
    searcher: *const HplFileSearcher, name: HplStr, count: *mut i32,
) -> usize {
    let (index, equal) = unsafe { &*searcher.cast::<FileSearcher>() }
        .get_file_path(unsafe { raw_slice(name.ptr, name.len) }, !count.is_null());
    if let Some(value) = equal { if !count.is_null() { unsafe { count.write(value); } } }
    index.unwrap_or(usize::MAX)
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
