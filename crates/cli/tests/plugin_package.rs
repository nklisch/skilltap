use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use serde_json::Value;
use skilltap_core::{
    domain::{AbsolutePath, HarnessId, HarnessSet},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, SystemExternalTreeObserver,
    },
    skill::ValidatedSkillTree,
    skill_compatibility::{SkillCompatibility, SkillCompatibilityClass},
};
use skilltap_test_support::TempRoot;

#[derive(Clone, Copy, Debug)]
enum Channel {
    Claude,
    Codex,
}

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("plugin")
}

fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn read_json(root: &Path, relative: &str) -> Result<Value, String> {
    let path = root.join(relative);
    let bytes = fs::read(&path).map_err(|error| format!("{relative}: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{relative}: invalid JSON: {error}"))
}

fn string_field<'a>(value: &'a Value, field: &str, context: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{context}: missing non-empty `{field}`"))
}

fn object_field<'a>(
    value: &'a Value,
    field: &str,
    context: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{context}: missing object `{field}`"))
}

fn assert_manifest(
    root: &Path,
    relative: &str,
    expected_name: &str,
    expected_version: &str,
    channel: Channel,
) -> Result<(), String> {
    let value = read_json(root, relative)?;
    let context = format!("manifest {relative}");
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: expected JSON object"))?;
    if string_field(&value, "name", &context)? != expected_name {
        return Err(format!("{context}: name does not match `{expected_name}`"));
    }
    if string_field(&value, "version", &context)? != expected_version {
        return Err(format!(
            "{context}: version does not match `{expected_version}`"
        ));
    }
    string_field(&value, "description", &context)?;
    let author = object_field(&value, "author", &context)?;
    if author
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .is_empty()
    {
        return Err(format!("{context}: author.name is required"));
    }
    if object.get("repository").and_then(Value::as_str).is_none() {
        return Err(format!("{context}: repository is required"));
    }
    if object.get("license").and_then(Value::as_str).is_none() {
        return Err(format!("{context}: license is required"));
    }
    match channel {
        Channel::Claude => {
            if object.contains_key("skills") {
                return Err(format!(
                    "{context}: Claude manifest must not declare Codex skills pointer"
                ));
            }
        }
        Channel::Codex => {
            if object.get("skills").and_then(Value::as_str) != Some("./skills/") {
                return Err(format!(
                    "{context}: Codex manifest must declare skills ./skills/"
                ));
            }
            object_field(&value, "interface", &context)?;
            for component in ["mcpServers", "apps", "hooks"] {
                if object.contains_key(component) {
                    return Err(format!(
                        "{context}: unsupported empty package component `{component}`"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn source_resolves_inside_package(value: &Value) -> bool {
    let source = value
        .as_str()
        .or_else(|| value.get("path").and_then(Value::as_str));
    let Some(source) = source else {
        return false;
    };
    let path = Path::new(source);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::CurDir | Component::Normal(_)))
}

fn assert_marketplace(
    root: &Path,
    relative: &str,
    channel: Channel,
    expected_version: &str,
) -> Result<(), String> {
    let value = read_json(root, relative)?;
    let context = format!("marketplace {relative}");
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: expected JSON object"))?;
    string_field(&value, "name", &context)?;
    match channel {
        Channel::Claude => {
            let owner = object_field(&value, "owner", &context)?;
            if owner
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .is_empty()
            {
                return Err(format!("{context}: owner.name is required"));
            }
        }
        Channel::Codex => {
            object_field(&value, "interface", &context)?;
        }
    }
    let plugins = object
        .get("plugins")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: plugins must be an array"))?;
    if plugins.len() != 1 {
        return Err(format!("{context}: expected exactly one plugin entry"));
    }
    let plugin = plugins[0]
        .as_object()
        .ok_or_else(|| format!("{context}: plugin entry must be an object"))?;
    if plugin.get("name").and_then(Value::as_str) != Some("skilltap") {
        return Err(format!("{context}: plugin entry name must be skilltap"));
    }
    if plugin.get("version").and_then(Value::as_str) != Some(expected_version) {
        return Err(format!(
            "{context}: plugin entry version does not match {expected_version}"
        ));
    }
    if plugin
        .get("description")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(format!("{context}: plugin description is required"));
    }
    if plugin.get("category").and_then(Value::as_str).is_none() {
        return Err(format!("{context}: plugin category is required"));
    }
    let policy = plugin
        .get("policy")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{context}: plugin policy is required"))?;
    if policy.get("installation").and_then(Value::as_str) != Some("AVAILABLE")
        || policy.get("authentication").and_then(Value::as_str) != Some("ON_INSTALL")
    {
        return Err(format!("{context}: plugin policy is not installable"));
    }
    let source = plugin
        .get("source")
        .ok_or_else(|| format!("{context}: plugin source is required"))?;
    if !source_resolves_inside_package(source) {
        return Err(format!("{context}: plugin source escapes package root"));
    }
    match channel {
        Channel::Claude if source.as_str() != Some("./") => {
            return Err(format!("{context}: Claude source must be ./"));
        }
        Channel::Codex => {
            let source_object = source
                .as_object()
                .ok_or_else(|| format!("{context}: Codex source must be an object"))?;
            if source_object.get("source").and_then(Value::as_str) != Some("local")
                || source_object.get("path").and_then(Value::as_str) != Some("./")
            {
                return Err(format!("{context}: Codex source must be local ./"));
            }
        }
        _ => {}
    }
    Ok(())
}

fn validated_skill(root: &Path) -> Result<ValidatedSkillTree, String> {
    let metadata = fs::symlink_metadata(root).map_err(|error| format!("skill root: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("skill root must be a regular directory".to_owned());
    }
    let skill_file = root.join("SKILL.md");
    let metadata =
        fs::symlink_metadata(&skill_file).map_err(|error| format!("skill SKILL.md: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("skill SKILL.md must be a regular file".to_owned());
    }
    let path = AbsolutePath::new(root.to_string_lossy().into_owned())
        .map_err(|error| format!("skill root path: {error}"))?;
    let limits = ExternalTreeLimits::new(64, 256, 1_048_576, 4_194_304, 4_096)
        .map_err(|error| format!("skill tree limits: {error}"))?;
    let snapshot = SystemExternalTreeObserver
        .observe(&ExternalTreeRequest::new(path, limits))
        .map_err(|error| format!("skill tree observation: {error}"))?;
    let skill = ValidatedSkillTree::validate(&snapshot)
        .map_err(|error| format!("complete skill validation: {error}"))?;
    if skill.declared_name().as_ref().map(|name| name.as_str()) != Some("skilltap") {
        return Err("skill frontmatter name must be skilltap".to_owned());
    }
    let targets = HarnessSet::new([
        HarnessId::new("codex").map_err(|error| error.to_string())?,
        HarnessId::new("claude").map_err(|error| error.to_string())?,
    ])
    .map_err(|error| error.to_string())?;
    let compatibility = SkillCompatibility::evaluate(&skill, &targets);
    if compatibility
        .iter()
        .any(|value| value.class() != SkillCompatibilityClass::Compatible)
    {
        return Err("skill frontmatter is not strict for both channels".to_owned());
    }
    Ok(skill)
}

fn validate_guidance(skill_root: &Path) -> Result<(), String> {
    let body = fs::read_to_string(skill_root.join("SKILL.md"))
        .map_err(|error| format!("guidance SKILL.md: {error}"))?;
    for reference in [
        "references/configuration.md",
        "references/instructions.md",
        "references/diagnostics.md",
    ] {
        let path = skill_root.join(reference);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("guidance reference {reference}: {error}"))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!("guidance reference {reference} must be regular"));
        }
        if !body.contains(&format!(
            "references/{}",
            reference.rsplit('/').next().unwrap()
        )) {
            return Err(format!("SKILL.md does not link {reference}"));
        }
    }
    if body.contains("skilltap skill install --source") {
        return Err("SKILL.md contains a duplicated command grammar".to_owned());
    }
    let normalized = body.to_ascii_lowercase();
    if normalized.contains("search for skills")
        || normalized.contains("browse marketplace contents")
        || normalized.contains("recommend a skill")
    {
        return Err("SKILL.md contains discovery instructions".to_owned());
    }
    Ok(())
}

fn validate_package(root: &Path) -> Result<ValidatedSkillTree, String> {
    let expected_version = expected_version();
    assert_manifest(
        root,
        ".claude-plugin/plugin.json",
        "skilltap",
        expected_version,
        Channel::Claude,
    )?;
    assert_manifest(
        root,
        ".codex-plugin/plugin.json",
        "skilltap",
        expected_version,
        Channel::Codex,
    )?;
    assert_marketplace(
        root,
        ".claude-plugin/marketplace.json",
        Channel::Claude,
        expected_version,
    )?;
    assert_marketplace(
        root,
        ".agents/plugins/marketplace.json",
        Channel::Codex,
        expected_version,
    )?;
    let skill = validated_skill(&root.join("skills/skilltap"))?;
    validate_guidance(&root.join("skills/skilltap"))?;
    Ok(skill)
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn fixture() -> (TempRoot, PathBuf) {
    let temporary = TempRoot::new("skilltap-plugin-package-fixture").unwrap();
    let root = temporary.join("plugin");
    copy_tree(&package_root(), &root).unwrap();
    (temporary, root)
}

#[test]
fn canonical_package_validates_each_native_channel_and_preserves_siblings() {
    let root = package_root();
    let skill = validate_package(&root).unwrap();
    assert!(
        skill
            .tree()
            .files()
            .keys()
            .any(|path| path.as_str() == "SKILL.md")
    );

    let (_temporary, fixture_root) = fixture();
    fs::create_dir_all(fixture_root.join("skills/skilltap/references")).unwrap();
    fs::write(
        fixture_root.join("skills/skilltap/references/example.md"),
        "supporting resource",
    )
    .unwrap();
    let skill = validate_package(&fixture_root).unwrap();
    assert!(
        skill
            .tree()
            .files()
            .keys()
            .any(|path| path.as_str() == "references/example.md")
    );
}

#[test]
fn package_validation_rejects_malformed_channel_documents_and_sources() {
    let (_temporary, root) = fixture();
    fs::write(root.join(".claude-plugin/plugin.json"), b"{").unwrap();
    assert!(
        validate_package(&root)
            .unwrap_err()
            .contains("invalid JSON")
    );

    let (_temporary, root) = fixture();
    let mut marketplace = read_json(&root, ".claude-plugin/marketplace.json").unwrap();
    marketplace.as_object_mut().unwrap().remove("owner");
    fs::write(
        root.join(".claude-plugin/marketplace.json"),
        serde_json::to_vec_pretty(&marketplace).unwrap(),
    )
    .unwrap();
    assert!(validate_package(&root).unwrap_err().contains("owner"));

    let (_temporary, root) = fixture();
    let mut manifest = read_json(&root, ".codex-plugin/plugin.json").unwrap();
    manifest["version"] = Value::String("9.9.9".to_owned());
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    assert!(validate_package(&root).unwrap_err().contains("version"));

    let (_temporary, root) = fixture();
    let mut marketplace = read_json(&root, ".claude-plugin/marketplace.json").unwrap();
    marketplace["plugins"][0]["source"] = Value::String("../outside".to_owned());
    fs::write(
        root.join(".claude-plugin/marketplace.json"),
        serde_json::to_vec_pretty(&marketplace).unwrap(),
    )
    .unwrap();
    assert!(validate_package(&root).unwrap_err().contains("escapes"));

    let (_temporary, root) = fixture();
    let mut marketplace = read_json(&root, ".claude-plugin/marketplace.json").unwrap();
    marketplace["plugins"][0]["source"] = Value::String("./other".to_owned());
    fs::write(
        root.join(".claude-plugin/marketplace.json"),
        serde_json::to_vec_pretty(&marketplace).unwrap(),
    )
    .unwrap();
    assert!(validate_package(&root).unwrap_err().contains("must be ./"));
}

#[test]
fn package_validation_rejects_incomplete_or_unsafe_skill_trees() {
    let (_temporary, root) = fixture();
    fs::remove_file(root.join("skills/skilltap/SKILL.md")).unwrap();
    assert!(validate_package(&root).unwrap_err().contains("SKILL.md"));

    let (_temporary, root) = fixture();
    fs::remove_file(root.join("skills/skilltap/SKILL.md")).unwrap();
    fs::create_dir(root.join("skills/skilltap/SKILL.md")).unwrap();
    assert!(
        validate_package(&root)
            .unwrap_err()
            .contains("regular file")
    );

    let (_temporary, root) = fixture();
    fs::write(
        root.join("skills/skilltap/SKILL.md"),
        "name: skilltap\ndescription: missing delimiters\n",
    )
    .unwrap();
    assert!(validate_package(&root).unwrap_err().contains("frontmatter"));

    #[cfg(unix)]
    {
        let (_temporary, root) = fixture();
        let outside = root.join("outside.md");
        fs::write(&outside, "secret").unwrap();
        fs::remove_file(root.join("skills/skilltap/SKILL.md")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("skills/skilltap/SKILL.md")).unwrap();
        assert!(
            validate_package(&root)
                .unwrap_err()
                .contains("regular file")
        );

        let (_temporary, root) = fixture();
        let outside = root.join("outside.md");
        fs::write(&outside, "secret").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("skills/skilltap/escape.md")).unwrap();
        assert!(validate_package(&root).unwrap_err().contains("symlink"));
    }
}

#[test]
fn guidance_validation_requires_references_and_rejects_discovery_or_duplicate_grammar() {
    let (_temporary, root) = fixture();
    fs::remove_file(root.join("skills/skilltap/references/diagnostics.md")).unwrap();
    assert!(validate_package(&root).unwrap_err().contains("diagnostics"));

    let (_temporary, root) = fixture();
    let skill = root.join("skills/skilltap/SKILL.md");
    let mut body = fs::read_to_string(&skill).unwrap();
    body.push_str("\nRun `skilltap skill install --source` directly.\n");
    fs::write(&skill, body).unwrap();
    assert!(validate_package(&root).unwrap_err().contains("duplicated"));

    let (_temporary, root) = fixture();
    let skill = root.join("skills/skilltap/SKILL.md");
    let mut body = fs::read_to_string(&skill).unwrap();
    body.push_str("\nSearch for skills in every marketplace.\n");
    fs::write(&skill, body).unwrap();
    assert!(validate_package(&root).unwrap_err().contains("discovery"));
}
