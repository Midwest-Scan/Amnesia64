//! `resources/cFileSearcher`: owns lookup buckets and recursive scan decisions.
//!
//! The facade retains native strings solely to return stable `const tWString&`.
//! Each entry stores its string's index; indices are copied with the native
//! string pool. Platform listing/path resolution and string materialization
//! stay native. No reference into this object survives a platform callback.
//!
//! Audit: the original lowercases names only, suppresses duplicates against
//! only the first equal-name entry, and includes the filename in path scoring.
//! The loaded-directory set is unused: rescans and recursion are not deduped.
//! Native empty-path/full-path failure hazards and directory cycles stay with
//! the platform/caller; this code does not normalize case, dots or recursion.

use hpl_system::WChar;
use hpl_system::string::{self as s, Separators};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
struct Entry {
    dirs: Vec<Vec<WChar>>,
    native: usize,
}

#[derive(Debug)]
pub struct FileSearcher {
    files: BTreeMap<Vec<u8>, Vec<Entry>>,
    lifetime: Rc<Cell<bool>>,
}

impl Default for FileSearcher {
    fn default() -> Self { Self { files: BTreeMap::new(), lifetime: Rc::new(Cell::new(true)) } }
}

impl Clone for FileSearcher {
    fn clone(&self) -> Self { Self { files: self.files.clone(), lifetime: Rc::new(Cell::new(true)) } }
}

impl Drop for FileSearcher {
    fn drop(&mut self) { self.lifetime.set(false); }
}

/// None is a saved native exception: stop immediately, preserving the prefix.
/// Listing collects the entire native list before processing any entries.
pub trait SearchHost {
    /// The host rereads the caller's original mask on every files-list call.
    fn list(&mut self, path: &[WChar], folders: bool) -> Option<Vec<Vec<WChar>>>;
    fn full_path(&mut self, path: &[WChar]) -> Option<Vec<WChar>>;
    fn retain_path(&mut self, path: &[WChar]) -> Option<usize>;
    fn retained_path(&mut self, index: usize) -> Vec<WChar>;
}

fn slashes(path: &[WChar]) -> Vec<WChar> {
    s::replace_char_to(path, &[b'\\' as WChar], &[b'/' as WChar])
}

fn components(path: &[WChar]) -> Vec<Vec<WChar>> {
    s::get_string_vec(path, Some(&[b'/' as WChar, b'\\' as WChar]), Separators::WithCr)
}

fn last_index(len: usize) -> i32 {
    // Faithful: MSVC narrows size_t to its signed low 32 bits first.
    // Deviation: abort only if the subsequent subtraction overflows (C++ UB).
    (len as i32).checked_sub(1).expect("FileSearcher component index overflow")
}

fn reverse_matches(wanted: &[Vec<WChar>], available: &[Vec<WChar>]) -> i32 {
    let mut next = last_index(wanted.len());
    let mut j = last_index(available.len());
    let mut count = 0;
    while next >= 0 && j >= 0 {
        if wanted[next as usize] == available[j as usize] { count += 1; next -= 1; }
        j -= 1;
    }
    count
}

impl FileSearcher {
    pub fn clear(&mut self) { self.files.clear(); }

    /// Replace only owned data: assignment during a native callback must not
    /// move/free the object or invalidate the outer operation's lifetime token.
    pub fn assign(&mut self, mut source: Self) { self.files = std::mem::take(&mut source.files); }

    /// Returns native string index and an optional write to apEqualCount.
    pub fn get_file_path(&self, name: &[u8], want_count: bool) -> (Option<usize>, Option<i32>) {
        let key = s::to_lower_case(&s::get_file_name(name));
        let Some(entries) = self.files.get(&key) else { return (None, Some(0)); };
        let first = entries[0].native;
        // Faithful: only the null-count overload bypasses scoring for one entry.
        if entries.len() == 1 && !want_count { return (Some(first), None); }
        let path: Vec<WChar> = s::to_16char(&s::get_file_path(name));
        // Faithful: even a non-null count pointer stays untouched here.
        if path.is_empty() { return (Some(first), None); }
        let wanted = components(&path);
        let mut best = first;
        let mut best_count = 0;
        for entry in entries {
            // Faithful: two asymmetric subsequences, including the stored
            // filename component, not a common suffix or normalized path.
            let count = reverse_matches(&wanted, &entry.dirs).max(reverse_matches(&entry.dirs, &wanted));
            if count > best_count { best_count = count; best = entry.native; }
        }
        (Some(best), Some(best_count))
    }

    /// # Safety
    /// `searcher` must be alive at entry. Reentry may clear, assign, add to or
    /// destroy it. After destruction only completion without another member
    /// access is valid. Hosts must not retain local input slices after return.
    pub unsafe fn add_directory<H: SearchHost>(searcher: *mut Self, path: &[WChar], recurse: bool, host: &mut H) -> bool {
        // The token outlives Self. A terminal callback may delete the facade
        // and return no files (or throw), after which C++ never accesses this.
        let lifetime = unsafe { &*searcher }.lifetime.clone();
        unsafe { Self::scan(searcher, path, recurse, host, &lifetime) }.is_some()
    }

    unsafe fn scan<H: SearchHost>(searcher: *mut Self, path: &[WChar], recurse: bool, host: &mut H, lifetime: &Cell<bool>) -> Option<()> {
        // Deviation: abort only on a subsequent member use after destruction,
        // which is undefined in C++. Merely returning/propagating is allowed.
        assert!(lifetime.get(), "FileSearcher member call after destruction");
        let path = slashes(path);
        let files = host.list(&path, false)?;
        for file in files {
            let key = s::to_lower_case(&s::to_8char(&file));
            let full = slashes(&host.full_path(&s::set_file_path(&file, &path))?);
            // Faithful: lookup happens after the platform call (which may
            // reenter); only the first equal-name path suppresses insertion.
            assert!(lifetime.get(), "FileSearcher lookup after destruction");
            let first = unsafe { &*searcher }.files.get(&key)
                .and_then(|entries| entries.first()).map(|entry| entry.native);
            // The returned native string is authoritative: even a legal
            // const_cast edit changes duplicate detection, not saved dirs.
            let duplicate = first.is_some_and(|index| host.retained_path(index) == full);
            if duplicate { continue; }
            let dirs = components(&full);
            let native = host.retain_path(&full)?;
            assert!(lifetime.get(), "FileSearcher insertion after destruction");
            unsafe { &mut *searcher }.files.entry(key).or_default().push(Entry { dirs, native });
        }
        if recurse {
            let folders = host.list(&path, true)?;
            for folder in folders {
                unsafe { Self::scan(searcher, &s::set_file_path(&folder, &path), true, host, lifetime) }?;
            }
        }
        Some(())
    }
}

#[cfg(test)]
#[path = "file_searcher_tests.rs"]
mod tests;
