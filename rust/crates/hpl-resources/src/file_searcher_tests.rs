use super::*;

fn wide(text: &str) -> Vec<WChar> { text.bytes().map(WChar::from).collect() }

#[derive(Default)]
struct Host {
    files: BTreeMap<Vec<WChar>, Vec<Vec<WChar>>>,
    folders: BTreeMap<Vec<WChar>, Vec<Vec<WChar>>>,
    resolved: BTreeMap<Vec<WChar>, Vec<WChar>>,
    retained: Vec<Vec<WChar>>,
    events: Vec<(char, Vec<WChar>)>,
    calls: BTreeMap<char, usize>,
    fail: Option<(char, usize)>,
    target: *mut FileSearcher,
    on_list: Option<fn(&mut Host)>,
    on_folders: Option<fn(&mut Host)>,
    on_full: Option<fn(&mut Host)>,
    assignment: Option<(FileSearcher, Vec<Vec<WChar>>)>,
}

impl Host {
    fn directory(&mut self, path: &str, names: &[&str]) {
        self.files.insert(wide(path), names.iter().map(|s| wide(s)).collect());
    }

    fn event(&mut self, kind: char, path: &[WChar]) -> bool {
        self.events.push((kind, path.to_vec()));
        let count = self.calls.entry(kind).or_default();
        *count += 1;
        self.fail != Some((kind, *count))
    }
}

impl SearchHost for Host {
    fn list(&mut self, path: &[WChar], folders: bool) -> Option<Vec<Vec<WChar>>> {
        let success = self.event(if folders { 'D' } else { 'L' }, path);
        let action = if folders { self.on_folders.take() } else { self.on_list.take() };
        if let Some(action) = action { action(self); }
        if !success { return None; }
        let map = if folders { &self.folders } else { &self.files };
        Some(map.get(path).cloned().unwrap_or_default())
    }

    fn full_path(&mut self, path: &[WChar]) -> Option<Vec<WChar>> {
        let success = self.event('F', path);
        if let Some(action) = self.on_full.take() { action(self); }
        if !success { return None; }
        Some(self.resolved.get(path).cloned().unwrap_or_else(|| path.to_vec()))
    }

    fn retain_path(&mut self, path: &[WChar]) -> Option<usize> {
        if !self.event('R', path) { return None; }
        let index = self.retained.len();
        self.retained.push(path.to_vec());
        Some(index)
    }

    fn retained_path(&mut self, index: usize) -> Vec<WChar> { self.retained[index].clone() }
}

fn scan(searcher: *mut FileSearcher, host: &mut Host, path: &str, recurse: bool) -> bool {
    host.target = searcher;
    // The fixture keeps only a raw pointer while a native-style callback may
    // mutate the searcher. Query assertions happen after the operation returns.
    unsafe { FileSearcher::add_directory(searcher, &wide(path), recurse, host) }
}

fn add(searcher: *mut FileSearcher, host: &mut Host, root: &str, name: &str, full: &str) {
    host.directory(root, &[name]);
    host.resolved.insert(wide(&format!("{root}/{name}")), wide(full));
    assert!(scan(searcher, host, root, false));
}

#[test]
fn missing_names_request_zero_even_when_no_count_is_requested() {
    let searcher = FileSearcher::default();
    assert_eq!(searcher.get_file_path(b"missing.txt", true), (None, Some(0)));
    assert_eq!(searcher.get_file_path(b"missing.txt", false), (None, Some(0)));
}

#[test]
fn file_keys_ignore_ascii_case_but_directory_scores_do_not() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "one", "ITEM.TXT", "Root/UPPER/ITEM.TXT");
    add(&raw mut searcher, &mut host, "two", "item.txt", "Root/upper/item.txt");
    assert_eq!(searcher.get_file_path(b"upper/ItEm.TxT", true), (Some(1), Some(1)));
    assert_eq!(searcher.get_file_path(b"UPPER/ItEm.TxT", true), (Some(0), Some(1)));
    assert_eq!(searcher.get_file_path(b"Upper/ItEm.TxT", true), (Some(0), Some(0)));
}

#[test]
fn bare_extension_name_leaves_count_untouched_with_one_or_many_entries() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "one", "a.txt", "one/a.txt");
    assert_eq!(searcher.get_file_path(b"a.txt", true), (Some(0), None));
    add(&raw mut searcher, &mut host, "two", "a.txt", "two/a.txt");
    assert_eq!(searcher.get_file_path(b"a.txt", true), (Some(0), None));
}

#[test]
fn unique_entry_skips_scoring_only_without_count_output() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "one", "a.txt", "Root/Folder/a.txt");
    assert_eq!(searcher.get_file_path(b"Folder/a.txt", false), (Some(0), None));
    assert_eq!(searcher.get_file_path(b"Folder/a.txt", true), (Some(0), Some(1)));
}

#[test]
fn extensionless_query_scores_the_stored_filename_component() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "root", "readme", "root/readme");
    assert_eq!(searcher.get_file_path(b"readme", true), (Some(0), Some(1)));
    assert_eq!(searcher.get_file_path(b"root/readme", true), (Some(0), Some(2)));
}

#[test]
fn first_equal_name_path_is_the_only_duplicate_rejection_target() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "one", "a.txt", "one/a.txt");
    assert!(scan(&raw mut searcher, &mut host, "one", false));
    assert_eq!(host.retained.len(), 1);
    add(&raw mut searcher, &mut host, "two", "a.txt", "two/a.txt");
    assert!(scan(&raw mut searcher, &mut host, "two", false));
    assert_eq!(host.retained.len(), 3);
    assert_eq!(searcher.files[b"a.txt".as_slice()].len(), 3);
}

#[test]
fn score_ties_keep_first_insertion_not_lexical_path_order() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "z", "a.txt", "Z/Shared/a.txt");
    add(&raw mut searcher, &mut host, "a", "a.txt", "A/Shared/a.txt");
    assert_eq!(searcher.get_file_path(b"Shared/a.txt", true), (Some(0), Some(1)));
}

#[test]
fn wanted_subsequence_skips_intervening_available_components() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "first", "a.txt", "Root/B/a.txt");
    add(&raw mut searcher, &mut host, "second", "a.txt", "Root/A/Gap/B/a.txt");
    assert_eq!(searcher.get_file_path(b"A/B/a.txt", true), (Some(1), Some(2)));
}

#[test]
fn available_subsequence_can_win_when_wanted_tail_does_not_match() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "first", "a.txt", "unmatched");
    add(&raw mut searcher, &mut host, "second", "a.txt", "a.txt");
    assert_eq!(searcher.get_file_path(b"a.txt/extra/a.txt", true), (Some(1), Some(1)));
}

#[test]
fn slashes_are_normalized_without_lowercasing_native_paths() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    host.directory("Root/Upper", &["A.TXT"]);
    host.resolved.insert(wide("Root/Upper/A.TXT"), wide("C:\\Mixed\\A.TXT"));
    assert!(scan(&raw mut searcher, &mut host, "Root\\Upper", false));
    assert_eq!(host.retained, vec![wide("C:/Mixed/A.TXT")]);
    assert_eq!(host.events[0], ('L', wide("Root/Upper")));
}

#[test]
fn empty_directory_keeps_the_original_leading_slash_join() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    host.directory("", &["a.txt"]);
    // Files-only listing handles an empty path; native Windows folder listing
    // does not, so this does not recurse into that original undefined case.
    assert!(scan(&raw mut searcher, &mut host, "", false));
    assert_eq!(host.retained, vec![wide("/a.txt")]);
}

#[test]
fn recursion_is_depth_first_after_all_current_directory_files() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    host.directory("root", &["a.txt", "b.txt"]);
    host.directory("root/z", &["c.txt"]);
    host.directory("root/a", &["d.txt"]);
    host.folders.insert(wide("root"), vec![wide("z"), wide("a")]);
    assert!(scan(&raw mut searcher, &mut host, "root", true));
    assert_eq!(host.retained, vec![wide("root/a.txt"), wide("root/b.txt"), wide("root/z/c.txt"), wide("root/a/d.txt")]);
    let scans: Vec<_> = host.events.iter().filter(|(kind, _)| *kind == 'L' || *kind == 'D').cloned().collect();
    assert_eq!(scans, vec![('L', wide("root")), ('D', wide("root")), ('L', wide("root/z")),
        ('D', wide("root/z")), ('L', wide("root/a")), ('D', wide("root/a"))]);
}

#[test]
fn repeated_scan_still_calls_platform_even_when_all_files_duplicate() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    host.directory("root", &["a.txt"]);
    assert!(scan(&raw mut searcher, &mut host, "root", false));
    assert!(scan(&raw mut searcher, &mut host, "root", false));
    assert_eq!(host.calls[&'L'], 2);
    assert_eq!(host.calls[&'F'], 2);
    assert_eq!(host.retained.len(), 1);
}

#[test]
fn native_failures_stop_at_the_completed_prefix() {
    for (failure, expected) in [(('L', 1), 0), (('F', 2), 1), (('R', 2), 1), (('D', 1), 3)] {
        let mut searcher = FileSearcher::default();
        let mut host = Host { fail: Some(failure), ..Host::default() };
        host.directory("root", &["a.txt", "b.txt", "c.txt"]);
        assert!(!scan(&raw mut searcher, &mut host, "root", true));
        assert_eq!(host.retained.len(), expected, "{failure:?}");
        assert_eq!(searcher.files.len(), expected, "{failure:?}");
    }
}

#[test]
fn clearing_removes_lookup_data_and_allows_new_native_indices() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "old", "a.txt", "old/a.txt");
    searcher.clear();
    host.retained.clear();
    assert_eq!(searcher.get_file_path(b"a.txt", true), (None, Some(0)));
    add(&raw mut searcher, &mut host, "new", "b.txt", "new/b.txt");
    assert_eq!(searcher.get_file_path(b"b.txt", false), (Some(0), None));
}

#[test]
fn cloning_is_independent_and_preserves_indices_and_duplicate_order() {
    let mut source = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut source, &mut host, "one", "a.txt", "one/a.txt");
    add(&raw mut source, &mut host, "two", "a.txt", "two/a.txt");
    let copy = source.clone();
    source.clear();
    assert_eq!(copy.get_file_path(b"two/a.txt", true), (Some(1), Some(1)));
    assert_eq!(copy.get_file_path(b"missing/a.txt", true), (Some(0), Some(0)));
}

#[test]
fn changed_native_string_affects_duplicate_check_but_not_saved_components() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "one", "a.txt", "old/a.txt");
    host.retained[0] = wide("new/a.txt");
    assert_eq!(searcher.get_file_path(b"old/a.txt", true), (Some(0), Some(1)));
    assert!(scan(&raw mut searcher, &mut host, "one", false));
    assert_eq!(host.retained, vec![wide("new/a.txt"), wide("old/a.txt")]);
    assert_eq!(searcher.get_file_path(b"old/a.txt", true), (Some(0), Some(1)));
}

#[test]
fn clear_during_full_path_is_seen_before_duplicate_check_and_insertion() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut searcher, &mut host, "root", "a.txt", "root/a.txt");
    host.on_full = Some(|host| {
        unsafe { &mut *host.target }.clear();
        host.retained.clear();
    });
    assert!(scan(&raw mut searcher, &mut host, "root", false));
    assert_eq!(searcher.get_file_path(b"a.txt", false), (Some(0), None));
    assert_eq!(host.retained, vec![wide("root/a.txt")]);
}

#[test]
fn assignment_during_full_path_preserves_outer_operation_and_replaces_data() {
    let mut source = FileSearcher::default();
    let mut source_host = Host::default();
    add(&raw mut source, &mut source_host, "copy", "copied.txt", "copy/copied.txt");
    let mut target = FileSearcher::default();
    let mut host = Host::default();
    add(&raw mut target, &mut host, "old", "old.txt", "old/old.txt");
    host.assignment = Some((source.clone(), source_host.retained.clone()));
    host.on_full = Some(|host| {
        let (copy, paths) = host.assignment.take().unwrap();
        let lifetime = unsafe { &*host.target }.lifetime.clone();
        unsafe { &mut *host.target }.assign(copy);
        host.retained = paths;
        assert!(Rc::ptr_eq(&lifetime, &unsafe { &*host.target }.lifetime));
        assert!(lifetime.get());
    });
    add(&raw mut target, &mut host, "outer", "outer.txt", "outer/outer.txt");
    assert_eq!(target.get_file_path(b"copied.txt", false), (Some(0), None));
    assert_eq!(target.get_file_path(b"outer.txt", false), (Some(1), None));
    assert_eq!(target.get_file_path(b"old.txt", false), (None, Some(0)));
}

#[test]
fn nested_add_during_full_path_finishes_before_outer_insertion() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    host.directory("inner", &["inner.txt"]);
    host.directory("outer", &["outer.txt"]);
    host.on_full = Some(|host| {
        let target = host.target;
        let lifetime = unsafe { &*target }.lifetime.clone();
        assert!(scan(target, host, "inner", false));
        assert!(Rc::ptr_eq(&lifetime, &unsafe { &*target }.lifetime));
        assert!(lifetime.get());
    });
    assert!(scan(&raw mut searcher, &mut host, "outer", false));
    assert_eq!(host.retained, vec![wide("inner/inner.txt"), wide("outer/outer.txt")]);
    assert_eq!(searcher.get_file_path(b"inner.txt", false), (Some(0), None));
    assert_eq!(searcher.get_file_path(b"outer.txt", false), (Some(1), None));
}

#[test]
fn windows_wide_filename_narrowing_uses_underscore_without_unicode_case_folding() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    let mut name = vec![0x100 as WChar];
    name.extend(wide(".TXT"));
    host.files.insert(wide("root"), vec![name]);
    assert!(scan(&raw mut searcher, &mut host, "root", false));
    // Both branches compile and run on their native platform; C-locale glibc
    // fails the whole conversion, whereas Windows narrows U+0100 to '_'.
    let key: &[u8] = if cfg!(windows) { b"_.txt" } else { b"" };
    assert_eq!(searcher.files[key].len(), 1);
}

#[test]
fn clone_during_callback_has_independent_lifetime() {
    let mut searcher = FileSearcher::default();
    let mut host = Host::default();
    host.directory("root", &["a.txt"]);
    host.on_full = Some(|host| {
        let copy = unsafe { &*host.target }.clone();
        assert!(!Rc::ptr_eq(&copy.lifetime, &unsafe { &*host.target }.lifetime));
        host.assignment = Some((copy, host.retained.clone()));
    });
    assert!(scan(&raw mut searcher, &mut host, "root", false));
    let lifetime = searcher.lifetime.clone();
    let copy = host.assignment.take().unwrap().0;
    drop(searcher);
    assert!(!lifetime.get());
    assert!(copy.lifetime.get());
}

fn delete_target(host: &mut Host) {
    let target = std::mem::take(&mut host.target);
    assert!(!target.is_null());
    unsafe { drop(Box::from_raw(target)); }
    host.retained.clear();
}

#[test]
fn terminal_empty_listing_can_delete_the_searcher_and_complete() {
    for recurse in [false, true] {
        let target = Box::into_raw(Box::new(FileSearcher::default()));
        let lifetime = unsafe { &*target }.lifetime.clone();
        let mut host = Host { on_list: Some(delete_target), ..Host::default() };
        assert!(scan(target, &mut host, "empty", recurse));
        assert!(!lifetime.get());
        assert!(host.target.is_null());
        assert_eq!(host.calls.get(&'D').copied().unwrap_or(0), usize::from(recurse));
    }
}

#[test]
fn deletion_followed_by_native_failure_propagates_without_member_access() {
    for kind in ['L', 'F', 'D'] {
        let target = Box::into_raw(Box::new(FileSearcher::default()));
        let lifetime = unsafe { &*target }.lifetime.clone();
        let mut host = Host { fail: Some((kind, 1)), ..Host::default() };
        match kind {
            'L' => host.on_list = Some(delete_target),
            'F' => { host.directory("root", &["a.txt"]); host.on_full = Some(delete_target); }
            'D' => host.on_folders = Some(delete_target),
            _ => unreachable!(),
        }
        assert!(!scan(target, &mut host, "root", true));
        assert!(!lifetime.get());
        assert!(host.target.is_null());
    }
}

#[test]
fn deletion_in_last_recursive_child_can_finish_outer_scan() {
    fn delete_in_child(host: &mut Host) {
        if host.calls[&'L'] == 2 { delete_target(host); }
        else { host.on_list = Some(delete_in_child); }
    }
    let target = Box::into_raw(Box::new(FileSearcher::default()));
    let lifetime = unsafe { &*target }.lifetime.clone();
    let mut host = Host { on_list: Some(delete_in_child), ..Host::default() };
    host.folders.insert(wide("root"), vec![wide("child")]);
    assert!(scan(target, &mut host, "root", true));
    assert!(!lifetime.get());
    assert!(host.target.is_null());
    assert_eq!(host.calls[&'L'], 2);
}

#[test]
#[should_panic(expected = "FileSearcher lookup after destruction")]
fn lookup_after_callback_deletion_is_rejected_before_dereference() {
    let target = Box::into_raw(Box::new(FileSearcher::default()));
    let mut host = Host { on_full: Some(delete_target), ..Host::default() };
    host.directory("root", &["a.txt"]);
    scan(target, &mut host, "root", false);
}

#[test]
fn component_last_index_uses_native_signed_low_bits() {
    assert_eq!(last_index(0), -1);
    assert_eq!(last_index(1), 0);
    assert_eq!(last_index(i32::MAX as usize), i32::MAX - 1);
    assert_eq!(last_index(i32::MAX as usize + 2), i32::MIN);
    assert_eq!(last_index(u32::MAX as usize), -2);
    if usize::BITS > 32 {
        // No giant allocation: these are the size_t -> int boundary values
        // used when scoring an already constructed component vector.
        assert_eq!(last_index((1u64 << 32) as usize), -1);
        assert_eq!(last_index(((1u64 << 32) + 2) as usize), 1);
    }
}

#[test]
#[should_panic(expected = "FileSearcher component index overflow")]
fn component_last_index_rejects_only_the_native_subtraction_overflow() {
    last_index(i32::MAX as usize + 1);
}
