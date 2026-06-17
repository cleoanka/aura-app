use app_lib::links::{self, LinkKind, RawLink};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn extracts_and_resolves_project_links_across_filetypes() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("aura-links-extract-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(root.join("docs")).map_err(|err| err.to_string())?;
    fs::create_dir_all(root.join("src")).map_err(|err| err.to_string())?;

    let files = [
        root.join("main.py"),
        root.join("utils.py"),
        root.join("main.c"),
        root.join("defs.h"),
        root.join("src/lib.rs"),
        root.join("src/util.rs"),
        root.join("app.ts"),
        root.join("helper.ts"),
        root.join("README.md"),
        root.join("docs/Guide.md"),
        root.join("renderer.o"),
    ];
    for file in files {
        fs::write(file, "").map_err(|err| err.to_string())?;
    }

    let project_files = collect_files(&root)?;
    let known_basenames = links::known_basename_index(&project_files);

    assert_resolves(
        &root,
        &root.join("main.py"),
        "import utils\nfrom package.sub import thing\n",
        &known_basenames,
        &project_files,
        RawLink {
            target_hint: "utils".to_string(),
            kind: LinkKind::Import,
        },
        &root.join("utils.py"),
    );
    assert_resolves(
        &root,
        &root.join("main.c"),
        "#include \"defs.h\"\n",
        &known_basenames,
        &project_files,
        RawLink {
            target_hint: "defs.h".to_string(),
            kind: LinkKind::Include,
        },
        &root.join("defs.h"),
    );
    assert_resolves(
        &root,
        &root.join("src/lib.rs"),
        "use crate::util::Thing;\nmod util;\n",
        &known_basenames,
        &project_files,
        RawLink {
            target_hint: "crate::util::Thing".to_string(),
            kind: LinkKind::Use,
        },
        &root.join("src/util.rs"),
    );
    assert_resolves(
        &root,
        &root.join("app.ts"),
        "import { helper } from './helper';\nconst lazy = import('./helper');\n",
        &known_basenames,
        &project_files,
        RawLink {
            target_hint: "./helper".to_string(),
            kind: LinkKind::Import,
        },
        &root.join("helper.ts"),
    );
    assert_resolves(
        &root,
        &root.join("README.md"),
        "[[Guide#Intro|Read]] and [guide](docs/Guide.md) mention renderer.o\n",
        &known_basenames,
        &project_files,
        RawLink {
            target_hint: "Guide".to_string(),
            kind: LinkKind::Wikilink,
        },
        &root.join("docs/Guide.md"),
    );
    assert_resolves(
        &root,
        &root.join("README.md"),
        "[[Guide#Intro|Read]] and [guide](docs/Guide.md) mention renderer.o\n",
        &known_basenames,
        &project_files,
        RawLink {
            target_hint: "docs/Guide.md".to_string(),
            kind: LinkKind::MdLink,
        },
        &root.join("docs/Guide.md"),
    );
    assert_resolves(
        &root,
        &root.join("README.md"),
        "[[Guide#Intro|Read]] and [guide](docs/Guide.md) mention renderer.o\n",
        &known_basenames,
        &project_files,
        RawLink {
            target_hint: "renderer.o".to_string(),
            kind: LinkKind::Mention,
        },
        &root.join("renderer.o"),
    );

    fs::remove_dir_all(&root).map_err(|err| err.to_string())?;
    Ok(())
}

fn assert_resolves(
    root: &Path,
    source: &Path,
    content: &str,
    known_basenames: &HashMap<String, Vec<String>>,
    project_files: &[PathBuf],
    expected_raw: RawLink,
    expected_target: &Path,
) {
    let raw_links = links::extract_links_with_mentions(source, content, known_basenames);
    assert!(
        raw_links.contains(&expected_raw),
        "missing raw link {expected_raw:?} in {raw_links:?}"
    );
    let resolved = links::resolve_links(root, source, &raw_links, project_files);
    assert!(
        resolved.iter().any(|(raw, link)| {
            raw == &expected_raw
                && link.resolved
                && PathBuf::from(&link.target_path) == expected_target
        }),
        "missing resolved link {expected_raw:?} -> {} in {resolved:?}",
        expected_target.display()
    );
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
