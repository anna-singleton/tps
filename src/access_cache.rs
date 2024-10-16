use std::{cmp::Ordering, collections::HashMap, error::Error, fs, path::PathBuf};
use chrono::Utc;
use itertools::Itertools;

use serde::{Deserialize, Serialize};

use crate::Project;

#[derive(Serialize, Deserialize)]
pub struct AccessCache {
    cache: HashMap<PathBuf, i64>,
    path: Option<PathBuf>,
    capacity: usize
}

impl Drop for AccessCache {
    fn drop(&mut self) {
        if self.path.is_none() {
            return;
        }
        if let Some(path) = &self.path {
            // if we have a path set, dump the hashmap back to the file,
            // otherwise we have nowhere to write it so dont.
            let b = toml::to_string_pretty(&self.cache).expect("[E001]: could not serialize access cache.
                please report this error.");
            if !fs::exists(&path.parent().expect("[E003] Error whilst trying to create parent cache directory")).unwrap_or(false) {
                fs::create_dir_all(&path.parent().unwrap()).expect("[E002] Could not create cache directory");
            }
            if let Err(e) = fs::write(path, b) {
                eprintln!("Could not write access cache. Recent accesses could not be updated.
                    Error: {}", e)
            }
        }
    }
}

impl AccessCache {
    pub fn load_from_file(path: PathBuf, capacity: usize) -> Result<Self, Box<dyn Error>> {
        if !fs::exists(&path)? {
            return Ok(AccessCache::load_blank(Some(path), capacity));
        }
        let raw = fs::read_to_string(&path)?;
        let cache = toml::from_str(&raw)?;
        return Ok(AccessCache { cache, path: Some(path), capacity });
    }

    pub fn load_blank(path: Option<PathBuf>, capacity: usize) -> Self {
        return Self {
            cache: HashMap::with_capacity(capacity),
            path,
            capacity
        };
    }

    pub fn register_access(&mut self, project: &Project) {
        if !self.cache.contains_key(&project.path) {
            self.eject_oldest_to_size(self.capacity-1);
        }
        self.cache.insert(project.path.clone(), Utc::now().timestamp());
    }

    fn get_access_time_for_project(&self, project: &Project) -> i64 {
        return *self.cache.get(&project.path).unwrap_or(&0);
    }

    pub fn cmp_projects_by_access_cache_time(&self, a: &Project, b: &Project) -> Ordering {
        return self.get_access_time_for_project(b).cmp(&self.get_access_time_for_project(a));
    }

    fn eject_oldest_to_size(&mut self, capacity: usize) {
        if self.cache.len() < capacity {
            return;
        }
        self.cache = self.cache
            .clone()
            .into_iter()
            .sorted_by_key(|(_k,v)| *v)
            .take(capacity)
            .collect();
    }
}
