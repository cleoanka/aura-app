use app_lib::commands::vault::{resolve_note_path, resolve_note_path_for_write};
use app_lib::settings::Settings;
use std::fs;

#[test]
fn read_guard_rejects_path_outside_configured_roots() -> Result<(), String> {
    let (root, outside) = test_paths("read")?;
    fs::write(root.join("inside.md"), "# Inside\n").map_err(|err| err.to_string())?;
    fs::write(outside.join("outside.md"), "# Outside\n").map_err(|err| err.to_string())?;

    let settings = settings_for(&root);

    assert!(resolve_note_path(&root.join("inside.md").to_string_lossy(), &settings).is_ok());
    assert!(resolve_note_path(&outside.join("outside.md").to_string_lossy(), &settings).is_err());

    cleanup(&root, &outside);
    Ok(())
}

#[test]
fn write_guard_blocks_path_traversal_outside_configured_roots() -> Result<(), String> {
    let (root, outside) = test_paths("write")?;
    let traversal = root.join("..").join(
        outside
            .file_name()
            .ok_or_else(|| "outside test dir has no file name".to_string())?,
    );
    let traversal = traversal.join("escaped.md");
    let settings = settings_for(&root);

    assert!(resolve_note_path_for_write(&traversal.to_string_lossy(), &settings).is_err());
    assert!(resolve_note_path_for_write(&root.join("new.md").to_string_lossy(), &settings).is_ok());

    cleanup(&root, &outside);
    Ok(())
}

fn settings_for(root: &std::path::Path) -> Settings {
    let mut settings = Settings::default();
    settings.vault_roots = vec![root.to_string_lossy().into_owned()];
    settings
}

fn test_paths(name: &str) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let base = std::env::temp_dir().join(format!("aura-vault-guard-{name}-{}", std::process::id()));
    let root = base.join("root");
    let outside = base.join("outside");
    if base.exists() {
        fs::remove_dir_all(&base).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(&root).map_err(|err| err.to_string())?;
    fs::create_dir_all(&outside).map_err(|err| err.to_string())?;
    Ok((root, outside))
}

fn cleanup(root: &std::path::Path, outside: &std::path::Path) {
    if let Some(base) = root.parent() {
        let _ = fs::remove_dir_all(base);
    }
    if let Some(base) = outside.parent() {
        let _ = fs::remove_dir_all(base);
    }
}
