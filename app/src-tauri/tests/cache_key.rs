use app_lib::commands::ai::cache_key;

#[test]
fn cache_key_is_deterministic() {
    let left = cache_key("what is alpha?", "a.md\0Alpha", "provider:model:fast", "1");
    let right = cache_key("what is alpha?", "a.md\0Alpha", "provider:model:fast", "1");

    assert_eq!(left, right);
}

#[test]
fn cache_key_changes_with_query_or_vault_epoch() {
    let base = cache_key("what is alpha?", "a.md\0Alpha", "provider:model:fast", "1");
    let changed_query = cache_key("what is beta?", "a.md\0Alpha", "provider:model:fast", "1");
    let changed_epoch = cache_key("what is alpha?", "a.md\0Alpha", "provider:model:fast", "2");

    assert_ne!(base, changed_query);
    assert_ne!(base, changed_epoch);
}
