use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagsIndex {
    pub tags: HashMap<String, Vec<String>>,
}

impl TagsIndex {
    fn path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".projector").join("tags.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(TagsIndex {
                tags: HashMap::new(),
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn tags_for_path(&self, path: &str) -> Vec<String> {
        let mut result = Vec::new();
        for (tag, paths) in &self.tags {
            if paths.iter().any(|p| p == path) {
                result.push(tag.clone());
            }
        }
        result.sort();
        result
    }

    pub fn paths_for_tag(&self, tag: &str) -> Vec<String> {
        self.tags
            .get(tag)
            .cloned()
            .unwrap_or_default()
    }

    pub fn add_tag(&mut self, path: &str, tag: &str) {
        self.tags
            .entry(tag.to_string())
            .or_default()
            .push(path.to_string());
        self.tags.get_mut(tag).unwrap().dedup();
    }

    pub fn remove_tag(&mut self, path: &str, tag: &str) {
        if let Some(paths) = self.tags.get_mut(tag) {
            paths.retain(|p| p != path);
            if paths.is_empty() {
                self.tags.remove(tag);
            }
        }
    }

    pub fn clear_path(&mut self, path: &str) {
        let mut empty_tags = Vec::new();
        for (tag, paths) in self.tags.iter_mut() {
            paths.retain(|p| p != path);
            if paths.is_empty() {
                empty_tags.push(tag.clone());
            }
        }
        for tag in &empty_tags {
            self.tags.remove(tag);
        }
    }

    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.tags.keys().cloned().collect();
        tags.sort();
        tags
    }

    pub fn has_tag(&self, path: &str, tag: &str) -> bool {
        self.tags
            .get(tag)
            .map(|paths| paths.iter().any(|p| p == path))
            .unwrap_or(false)
    }
}
