use super::*;
use std::ptr;

fn wide(text: &str) -> Vec<HplWChar> { text.bytes().map(HplWChar::from).collect() }
fn narrow(text: &[u8]) -> HplStr { HplStr { ptr: text.as_ptr(), len: text.len() } }

struct Owner(*mut HplFileSearcher);
impl Owner {
    fn new() -> Self { Self(unsafe { hpl_resources_file_searcher_new() }) }
    fn get(&self, name: &[u8], count: *mut i32) -> usize {
        unsafe { hpl_resources_file_searcher_get_file_path(self.0, narrow(name), count) }
    }
}
impl Drop for Owner {
    fn drop(&mut self) { unsafe { hpl_resources_file_searcher_free(self.0); } }
}

#[derive(Clone, Copy, Default)]
enum Action {
    #[default]
    None,
    Clear,
    Assign(*const HplFileSearcher),
}

#[derive(Default)]
struct Fixture {
    target: *mut HplFileSearcher,
    files: Vec<Vec<HplWChar>>,
    paths: Vec<Vec<HplWChar>>,
    assignment_paths: Vec<Vec<HplWChar>>,
    list_calls: usize,
    full_calls: usize,
    retain_calls: usize,
    fail_list: bool,
    fail_full: usize,
    fail_retain: usize,
    action: Action,
}

impl Fixture {
    fn new(target: &Owner, files: &[&str]) -> Box<Self> {
        Box::new(Self { target: target.0, files: files.iter().map(|s| wide(s)).collect(), ..Self::default() })
    }
}

unsafe extern "C" fn list(ctx: *mut c_void, _path: HplWStr, folders: bool, out: HplWStrOut) -> bool {
    let (files, fail) = {
        let fixture = unsafe { &mut *ctx.cast::<Fixture>() };
        fixture.list_calls += 1;
        (if folders { Vec::new() } else { fixture.files.clone() }, fixture.fail_list)
    };
    for file in files {
        unsafe { out.sink.unwrap()(out.ctx, file.as_ptr(), file.len()); }
    }
    // A native operation may have filled part/all of its local list before
    // throwing. Rust must discard that result rather than process the prefix.
    !fail
}

unsafe extern "C" fn full_path(ctx: *mut c_void, path: HplWStr, out: HplWStrOut) -> bool {
    let resolved = unsafe { raw_slice(path.ptr, path.len) }.to_vec();
    let (target, action, replacement, fail) = {
        let fixture = unsafe { &mut *ctx.cast::<Fixture>() };
        fixture.full_calls += 1;
        (fixture.target, std::mem::take(&mut fixture.action), fixture.assignment_paths.clone(),
         fixture.fail_full == fixture.full_calls)
    };
    // No Rust reference to the fixture or searcher survives these FFI calls.
    match action {
        Action::None => {}
        Action::Clear => {
            unsafe { hpl_resources_file_searcher_clear(target); }
            unsafe { &mut *ctx.cast::<Fixture>() }.paths.clear();
        }
        Action::Assign(source) => {
            unsafe { hpl_resources_file_searcher_assign(target, source); }
            unsafe { &mut *ctx.cast::<Fixture>() }.paths = replacement;
        }
    }
    if fail { return false; }
    unsafe { out.sink.unwrap()(out.ctx, resolved.as_ptr(), resolved.len()); }
    true
}

unsafe extern "C" fn retain_path(ctx: *mut c_void, path: HplWStr, index: *mut usize) -> bool {
    let fixture = unsafe { &mut *ctx.cast::<Fixture>() };
    fixture.retain_calls += 1;
    if fixture.fail_retain == fixture.retain_calls { return false; }
    let saved = unsafe { raw_slice(path.ptr, path.len) }.to_vec();
    unsafe { index.write(fixture.paths.len()); }
    fixture.paths.push(saved);
    true
}

unsafe extern "C" fn retained_path(ctx: *mut c_void, index: usize) -> HplWStr {
    let path = &unsafe { &*ctx.cast::<Fixture>() }.paths[index];
    view(path)
}

fn host(fixture: *mut Fixture) -> HplFileSearchHost {
    HplFileSearchHost { ctx: fixture.cast(), list, full_path, retain_path, retained_path }
}

fn add(target: &Owner, fixture: *mut Fixture, directory: &str) -> bool {
    let directory = wide(directory);
    unsafe { hpl_resources_file_searcher_add_directory(target.0, view(&directory), false, &host(fixture)) }
}

#[test]
fn index_zero_is_a_match_and_missing_is_size_max() {
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["Foo.DAT"]);
    assert!(add(&target, &mut *fixture, "Root"));
    let mut count = 739;
    assert_eq!(target.get(b"foo.dat", &mut count), 0);
    assert_eq!(count, 739); // Empty wanted path never writes the count.
    assert_eq!(target.get(b"Root/foo.dat", &mut count), 0);
    assert_eq!(count, 1);
    assert_eq!(target.get(b"foo.dat", ptr::null_mut()), 0);
    assert_eq!(target.get(b"missing.dat", &mut count), usize::MAX);
    assert_eq!(count, 0);
    assert_eq!(target.get(b"missing.dat", ptr::null_mut()), usize::MAX);
}

#[test]
fn empty_lookup_and_null_free_are_supported() {
    let target = Owner::new();
    let mut count = -31;
    assert_eq!(target.get(b"", &mut count), usize::MAX);
    assert_eq!(count, 0);
    unsafe { hpl_resources_file_searcher_free(ptr::null_mut()); }
}

#[test]
fn clear_allows_native_indices_to_restart_at_zero() {
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["old.dat"]);
    assert!(add(&target, &mut *fixture, "Root"));
    unsafe { hpl_resources_file_searcher_clear(target.0); }
    fixture.paths.clear();
    fixture.files = vec![wide("new.dat")];
    assert!(add(&target, &mut *fixture, "Other"));
    assert_eq!(target.get(b"old.dat", ptr::null_mut()), usize::MAX);
    assert_eq!(target.get(b"new.dat", ptr::null_mut()), 0);
    assert_eq!(fixture.paths, vec![wide("Other/new.dat")]);
}

#[test]
fn clone_assignment_and_self_assignment_have_independent_data() {
    let source = Owner::new();
    let mut fixture = Fixture::new(&source, &["first.dat", "second.dat"]);
    assert!(add(&source, &mut *fixture, "Root"));
    let cloned = Owner(unsafe { hpl_resources_file_searcher_clone(source.0) });
    let assigned = Owner::new();
    unsafe {
        hpl_resources_file_searcher_assign(assigned.0, source.0);
        hpl_resources_file_searcher_assign(assigned.0, assigned.0);
        hpl_resources_file_searcher_clear(source.0);
    }
    for copy in [&cloned, &assigned] {
        assert_eq!(copy.get(b"FIRST.DAT", ptr::null_mut()), 0);
        assert_eq!(copy.get(b"second.dat", ptr::null_mut()), 1);
    }
    assert_eq!(source.get(b"first.dat", ptr::null_mut()), usize::MAX);
    unsafe { hpl_resources_file_searcher_clear(cloned.0); }
    assert_eq!(assigned.get(b"second.dat", ptr::null_mut()), 1);
}

#[test]
fn failed_listing_discards_emitted_names_and_keeps_existing_entries() {
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["kept.dat"]);
    assert!(add(&target, &mut *fixture, "Root"));
    fixture.files = vec![wide("ignored.dat")];
    fixture.fail_list = true;
    assert!(!add(&target, &mut *fixture, "Other"));
    assert_eq!(fixture.full_calls, 1);
    assert_eq!(fixture.paths.len(), 1);
    assert_eq!(target.get(b"kept.dat", ptr::null_mut()), 0);
    assert_eq!(target.get(b"ignored.dat", ptr::null_mut()), usize::MAX);
}

#[test]
fn failed_full_path_preserves_completed_prefix_and_stops() {
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["one.dat", "two.dat", "three.dat"]);
    fixture.fail_full = 2;
    assert!(!add(&target, &mut *fixture, "Root"));
    assert_eq!((fixture.full_calls, fixture.retain_calls), (2, 1));
    assert_eq!(target.get(b"one.dat", ptr::null_mut()), 0);
    assert_eq!(target.get(b"two.dat", ptr::null_mut()), usize::MAX);
    assert_eq!(target.get(b"three.dat", ptr::null_mut()), usize::MAX);
}

#[test]
fn failed_native_string_retention_does_not_publish_entry() {
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["one.dat", "two.dat", "three.dat"]);
    fixture.fail_retain = 2;
    assert!(!add(&target, &mut *fixture, "Root"));
    assert_eq!((fixture.full_calls, fixture.retain_calls), (2, 2));
    assert_eq!(fixture.paths, vec![wide("Root/one.dat")]);
    assert_eq!(target.get(b"one.dat", ptr::null_mut()), 0);
    assert_eq!(target.get(b"two.dat", ptr::null_mut()), usize::MAX);
}

#[test]
fn assignment_during_full_path_keeps_outer_object_and_uses_new_data() {
    let source = Owner::new();
    let mut source_fixture = Fixture::new(&source, &["kept.dat"]);
    assert!(add(&source, &mut *source_fixture, "Source"));
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["old.dat"]);
    assert!(add(&target, &mut *fixture, "Old"));
    fixture.files = vec![wide("new.dat")];
    fixture.assignment_paths = source_fixture.paths.clone();
    fixture.action = Action::Assign(source.0);
    assert!(add(&target, &mut *fixture, "New"));
    assert_eq!(target.get(b"old.dat", ptr::null_mut()), usize::MAX);
    assert_eq!(target.get(b"kept.dat", ptr::null_mut()), 0);
    assert_eq!(target.get(b"new.dat", ptr::null_mut()), 1);
    assert_eq!(fixture.paths, vec![wide("Source/kept.dat"), wide("New/new.dat")]);
}

#[test]
fn retained_native_path_edits_affect_duplicates_but_not_saved_components() {
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["asset.dat"]);
    assert!(add(&target, &mut *fixture, "Old"));
    fixture.paths[0] = wide("New/asset.dat");
    assert!(add(&target, &mut *fixture, "New"));
    assert_eq!(fixture.retain_calls, 1); // Live native path suppressed insertion.
    let mut count = -1;
    assert_eq!(target.get(b"Old/asset.dat", &mut count), 0);
    assert_eq!(count, 1); // Components remain those captured under Old.
    assert_eq!(target.get(b"New/asset.dat", &mut count), 0);
    assert_eq!(count, 0);
}

#[test]
fn aliased_entry_path_is_owned_before_reentrant_clear() {
    let target = Owner::new();
    let mut fixture = Fixture::new(&target, &["parent.dat"]);
    assert!(add(&target, &mut *fixture, "Root"));
    fixture.files = vec![wide("child.dat")];
    fixture.action = Action::Clear;
    let path = view(&fixture.paths[0]);
    let callbacks = host(&mut *fixture);
    // Native input points into the pool that the callback clears. The facade
    // API permits this: AddDirectory copies/normalizes its entry path first.
    assert!(unsafe { hpl_resources_file_searcher_add_directory(target.0, path, false, &callbacks) });
    assert_eq!(target.get(b"parent.dat", ptr::null_mut()), usize::MAX);
    assert_eq!(target.get(b"child.dat", ptr::null_mut()), 0);
    assert_eq!(fixture.paths, vec![wide("Root/parent.dat/child.dat")]);
}
