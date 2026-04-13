use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, SkillState};

#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct Skill {
    /// Stable config key, derived from the central store directory name.
    pub key: String,
    pub meta: SkillMeta,
    pub active: bool,
    /// Path inside central store
    pub store_path: PathBuf,
}

/// Represents a skill found in a target directory that isn't managed yet.
#[derive(Debug, Clone)]
pub struct UnmanagedSkill {
    pub name: String,
    pub source_path: PathBuf,
    pub is_symlink: bool,
    /// If it's a symlink, where does it point?
    pub symlink_target: Option<PathBuf>,
    pub meta: SkillMeta,
}

/// Parse YAML frontmatter from a SKILL.md file.
pub fn parse_frontmatter(path: &Path) -> Option<SkillMeta> {
    let content = fs::read_to_string(path).ok()?;
    let content = content.trim();

    if !content.starts_with("---") {
        return None;
    }

    let after_first = &content[3..];
    let end = after_first.find("---")?;
    let frontmatter = &after_first[..end];

    let mut name = String::new();
    let mut description = String::new();
    let mut version = String::new();
    let mut author = String::new();
    let mut in_description = false;
    let mut in_metadata = false;
    let mut desc_lines: Vec<String> = Vec::new();

    for line in frontmatter.lines() {
        let trimmed = line.trim();
        // Detect indented continuation lines (starts with whitespace)
        let is_indented = !line.is_empty() && (line.starts_with(' ') || line.starts_with('\t'));

        if is_indented && in_description && !trimmed.is_empty() {
            desc_lines.push(trimmed.to_string());
            continue;
        }

        // Handle indented keys under metadata:
        if is_indented && in_metadata && trimmed.contains(':') {
            let first_colon = trimmed.find(':').unwrap();
            let key = &trimmed[..first_colon];
            let val = trimmed[first_colon + 1..]
                .trim()
                .trim_matches('\'')
                .trim_matches('"');
            match key {
                "author" => {
                    if author.is_empty() {
                        author = val.to_string();
                    }
                }
                "version" => {
                    if version.is_empty() {
                        version = val.to_string();
                    }
                }
                _ => {}
            }
            continue;
        }

        // Check if this line starts a new top-level key (not indented, has a colon)
        if !is_indented && !trimmed.is_empty() && trimmed.contains(':') {
            let first_colon = trimmed.find(':').unwrap();
            let key_part = &trimmed[..first_colon];
            // Only treat as a new key if the key part has no spaces (simple YAML key)
            if !key_part.contains(' ') {
                // If we were collecting description lines, stop
                if in_description {
                    in_description = false;
                }

                let val = trimmed[first_colon + 1..].trim();

                match key_part {
                    "name" => name = val.to_string(),
                    "description" => {
                        if val.is_empty() || val == "|" || val == ">" {
                            // Multi-line description (block scalar or plain flow)
                            in_description = true;
                            desc_lines.clear();
                        } else {
                            description = val.to_string();
                        }
                    }
                    "version" => {
                        // Handle quoted versions like '1.0.0'
                        version = val.trim_matches('\'').trim_matches('"').to_string();
                    }
                    "author" => author = val.to_string(),
                    "metadata" => {
                        in_metadata = true;
                    }
                    _ => {
                        in_metadata = false;
                    }
                }
                continue;
            }
        }

        // Non-indented empty line or new section ends description collection
        if in_description && !is_indented {
            in_description = false;
        }
    }

    if !desc_lines.is_empty() {
        description = desc_lines.join(" ").trim().to_string();
    }

    // Strip surrounding quotes from description
    if (description.starts_with('"') && description.ends_with('"'))
        || (description.starts_with('\'') && description.ends_with('\''))
    {
        description = description[1..description.len() - 1].to_string();
    }

    // Fall back to directory name if no name in frontmatter
    if name.is_empty() {
        name = path.parent()?.file_name()?.to_string_lossy().to_string();
    }

    Some(SkillMeta {
        name,
        description,
        version,
        author,
    })
}

/// Scan the central store and build the skill list from config state.
pub fn load_managed_skills(config: &Config) -> Vec<Skill> {
    let store = Config::central_store();
    let mut skills = Vec::new();

    if !store.exists() {
        return skills;
    }

    let Ok(entries) = fs::read_dir(&store) else {
        return skills;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }

        let dir_name = path.file_name().unwrap().to_string_lossy().to_string();

        let meta = parse_frontmatter(&skill_md).unwrap_or(SkillMeta {
            name: dir_name.clone(),
            description: String::new(),
            version: String::new(),
            author: String::new(),
        });

        let active = config
            .skills
            .get(&dir_name)
            .map(|s| s.active)
            .unwrap_or(false);

        skills.push(Skill {
            key: dir_name,
            meta,
            active,
            store_path: path,
        });
    }

    skills.sort_by(|a, b| a.meta.name.cmp(&b.meta.name));
    skills
}

/// Repair old config entries that were keyed by frontmatter name instead of store dir.
pub fn normalize_skill_state_keys(config: &mut Config) -> bool {
    normalize_skill_state_keys_for_store(config, &Config::central_store())
}

fn normalize_skill_state_keys_for_store(config: &mut Config, store: &Path) -> bool {
    let Ok(entries) = fs::read_dir(store) else {
        return false;
    };

    let managed_dirs: std::collections::HashSet<String> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    let mut changed = false;

    for dir_name in &managed_dirs {
        let skill_md = store.join(dir_name).join("SKILL.md");
        let Some(meta) = parse_frontmatter(&skill_md) else {
            continue;
        };

        if meta.name == *dir_name || managed_dirs.contains(&meta.name) {
            continue;
        }

        let Some(alias_state) = config.skills.remove(&meta.name) else {
            continue;
        };

        let needs_update = config.skills.get(dir_name) != Some(&alias_state);
        if needs_update {
            config.skills.insert(dir_name.clone(), alias_state);
        }

        changed = true;
    }

    changed
}

/// Scan target directories for skills not yet in the central store.
pub fn find_unmanaged_skills(config: &Config) -> Vec<UnmanagedSkill> {
    let store = Config::central_store();
    let mut unmanaged = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for target_dir in config.expanded_target_dirs() {
        if !target_dir.exists() {
            continue;
        }

        let Ok(entries) = fs::read_dir(&target_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let dir_name = path.file_name().unwrap().to_string_lossy().to_string();

            // Skip hidden dirs like .claude
            if dir_name.starts_with('.') {
                continue;
            }

            let is_symlink = path
                .symlink_metadata()
                .map(|m| m.is_symlink())
                .unwrap_or(false);

            // If it's a symlink pointing to our central store, it's already managed
            if is_symlink {
                if let Ok(target) = fs::read_link(&path) {
                    let resolved = if target.is_absolute() {
                        target.clone()
                    } else {
                        path.parent().unwrap().join(&target)
                    };
                    if resolved.starts_with(&store) {
                        continue;
                    }
                }
            }

            // Must have a SKILL.md
            let actual_path = if is_symlink {
                // Follow the symlink to check for SKILL.md
                fs::canonicalize(&path).unwrap_or(path.clone())
            } else {
                path.clone()
            };

            let skill_md = actual_path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            if seen_names.contains(&dir_name) {
                continue;
            }
            seen_names.insert(dir_name.clone());

            // Already in central store? Skip.
            if store.join(&dir_name).exists() {
                continue;
            }

            let meta = parse_frontmatter(&skill_md).unwrap_or(SkillMeta {
                name: dir_name.clone(),
                description: String::new(),
                version: String::new(),
                author: String::new(),
            });

            let symlink_target = if is_symlink {
                fs::read_link(&path).ok()
            } else {
                None
            };

            unmanaged.push(UnmanagedSkill {
                name: dir_name,
                source_path: path,
                is_symlink,
                symlink_target,
                meta,
            });
        }
    }

    unmanaged.sort_by(|a, b| a.name.cmp(&b.name));
    unmanaged
}

/// Import unmanaged skills into the central store.
/// Returns the list of names imported.
pub fn import_skills(unmanaged: &[UnmanagedSkill], config: &mut Config) -> Vec<String> {
    let store = Config::central_store();
    fs::create_dir_all(&store).expect("Failed to create central store");

    let mut imported = Vec::new();

    for skill in unmanaged {
        let dest = store.join(&skill.name);

        // Resolve the actual source directory
        let source = if skill.is_symlink {
            match &skill.symlink_target {
                Some(target) => {
                    let resolved = if target.is_absolute() {
                        target.clone()
                    } else {
                        skill.source_path.parent().unwrap().join(target)
                    };
                    fs::canonicalize(&resolved).unwrap_or(resolved)
                }
                None => continue,
            }
        } else {
            skill.source_path.clone()
        };

        // Copy the skill directory into central store
        if let Err(e) = copy_dir_recursive(&source, &dest) {
            eprintln!("Failed to copy {}: {}", skill.name, e);
            continue;
        }

        // Add to config as active (preserving current state)
        config
            .skills
            .insert(skill.name.clone(), SkillState { active: true });

        imported.push(skill.name.clone());
    }

    // Now clean up: remove originals and symlinks from target dirs
    for target_dir in config.expanded_target_dirs() {
        for name in &imported {
            let target_path = target_dir.join(name);
            if target_path.exists() || target_path.symlink_metadata().is_ok() {
                if target_path
                    .symlink_metadata()
                    .map(|m| m.is_symlink())
                    .unwrap_or(false)
                {
                    let _ = fs::remove_file(&target_path);
                } else {
                    let _ = fs::remove_dir_all(&target_path);
                }
            }
        }
    }

    // Create symlinks for active skills
    sync_symlinks(config);

    imported
}

/// Synchronize symlinks: create for active, remove for inactive.
pub fn sync_symlinks(config: &Config) {
    let store = Config::central_store();

    for target_dir in config.expanded_target_dirs() {
        fs::create_dir_all(&target_dir).ok();

        for (name, state) in &config.skills {
            let link_path = target_dir.join(name);
            let store_path = store.join(name);

            let link_exists = link_path.symlink_metadata().is_ok();

            let is_symlink = link_path
                .symlink_metadata()
                .map(|m| m.is_symlink())
                .unwrap_or(false);
            let is_correct_symlink = is_symlink
                && fs::read_link(&link_path).ok().map_or(false, |target| {
                    let resolved = if target.is_absolute() {
                        target
                    } else {
                        link_path.parent().unwrap().join(&target)
                    };
                    resolved.starts_with(&store)
                });

            if state.active && store_path.exists() {
                if is_correct_symlink {
                    // Already good
                } else if link_exists {
                    // Remove whatever is there (real dir or wrong symlink)
                    if is_symlink {
                        let _ = fs::remove_file(&link_path);
                    } else {
                        let _ = fs::remove_dir_all(&link_path);
                    }
                    let _ = unix_fs::symlink(&store_path, &link_path);
                } else {
                    let _ = unix_fs::symlink(&store_path, &link_path);
                }
            } else if !state.active && link_exists {
                // Remove symlink or real dir for inactive skills
                if is_symlink {
                    let _ = fs::remove_file(&link_path);
                } else {
                    let _ = fs::remove_dir_all(&link_path);
                }
            }
        }
    }
}

/// Delete a skill entirely: remove from central store, symlinks, and config.
pub fn delete_skill(name: &str, config: &mut Config) {
    let store = Config::central_store();
    let store_path = store.join(name);

    // Remove symlinks from all target dirs
    for target_dir in config.expanded_target_dirs() {
        let link_path = target_dir.join(name);
        if link_path.symlink_metadata().is_ok() {
            if link_path
                .symlink_metadata()
                .map(|m| m.is_symlink())
                .unwrap_or(false)
            {
                let _ = fs::remove_file(&link_path);
            } else {
                let _ = fs::remove_dir_all(&link_path);
            }
        }
    }

    // Remove from central store
    if store_path.exists() {
        let _ = fs::remove_dir_all(&store_path);
    }

    // Remove from config
    config.skills.remove(name);
    config.save();
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TargetsConfig;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_STORE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn make_temp_store() -> PathBuf {
        let counter = TEST_STORE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let unique = format!(
            "skill-manager-test-{}-{}-{}",
            std::process::id(),
            counter,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_skill(store: &Path, dir_name: &str, meta_name: &str) {
        let skill_dir = store.join(dir_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {}\ndescription: test\n---\n", meta_name),
        )
        .unwrap();
    }

    #[test]
    fn normalize_moves_alias_state_to_directory_key() {
        let store = make_temp_store();
        write_skill(&store, "gstack-review", "review");

        let mut config = Config {
            targets: TargetsConfig { dirs: vec![] },
            skills: BTreeMap::from([
                ("gstack-review".to_string(), SkillState { active: true }),
                ("review".to_string(), SkillState { active: false }),
            ]),
        };

        let changed = normalize_skill_state_keys_for_store(&mut config, &store);

        assert!(changed);
        assert_eq!(
            config.skills.get("gstack-review"),
            Some(&SkillState { active: false })
        );
        assert!(!config.skills.contains_key("review"));

        let _ = fs::remove_dir_all(store);
    }

    #[test]
    fn normalize_keeps_alias_when_it_is_a_real_skill_directory() {
        let store = make_temp_store();
        write_skill(&store, "gstack-review", "review");
        write_skill(&store, "review", "review");

        let mut config = Config {
            targets: TargetsConfig { dirs: vec![] },
            skills: BTreeMap::from([
                ("gstack-review".to_string(), SkillState { active: true }),
                ("review".to_string(), SkillState { active: false }),
            ]),
        };

        let changed = normalize_skill_state_keys_for_store(&mut config, &store);

        assert!(!changed);
        assert_eq!(
            config.skills.get("gstack-review"),
            Some(&SkillState { active: true })
        );
        assert_eq!(
            config.skills.get("review"),
            Some(&SkillState { active: false })
        );

        let _ = fs::remove_dir_all(store);
    }
}
