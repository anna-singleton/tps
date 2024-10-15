use std::{collections::HashSet, path::PathBuf};

use directories::BaseDirs;
use git2::Repository;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ConfigFileFormat {
    project_homes: Vec<String>,

    #[serde(default)]
    projects: Vec<String>,

    #[serde(default)]
    skip_current: bool
}

pub struct Config {
    pub projects: Vec<PathBuf>,
    pub skip_current: bool,
}

impl Config {
    pub fn load() -> Self {
        let base_dirs = BaseDirs::new().unwrap();
        let conf_dir = BaseDirs::config_dir(&base_dirs);
        let conf_file_path = conf_dir.join("tps/config.toml");

        let home_dir = BaseDirs::home_dir(&base_dirs);
        let mut open_dirs:Vec<PathBuf> = Vec::new();

        let conf_file_contents = match std::fs::read_to_string(&conf_file_path) {
            Ok(s) => s,
            Err(e) => panic!("could not read config file with error: {}", e),
        };

        let conf = match toml::from_str::<ConfigFileFormat>(&conf_file_contents) {
            Ok(s) => s,
            Err(e) => panic!("could not parse config file with error: {}", e),
        };
        let mut projects = vec![];

        for path_raw in conf.projects.iter() {
            let mut path = PathBuf::new();
            if path_raw.chars().nth(0).unwrap() == '~' {
                path.push(home_dir);
                let path_raw_truncated = path_raw.split_at(2).1;
                path.push(path_raw_truncated);
                projects.push(path);
            } else {
                projects.push(path);
            }
        }

        for path_raw in conf.project_homes.iter() {
            let mut path = PathBuf::new();
            if path_raw.chars().nth(0).unwrap() == '~' {
                path.push(home_dir);
                let path_raw_truncated = path_raw.split_at(2).1;
                path.push(path_raw_truncated);
                open_dirs.push(path);
            } else {
                open_dirs.push(path);
            }
        }

        let mut closed_dirs:HashSet<PathBuf> = HashSet::new();

        while ! open_dirs.is_empty() {
            let p = open_dirs.pop().unwrap();
            closed_dirs.insert(p.clone());
            let subdirs = match p.read_dir() {
                Ok(subdirs) => subdirs,
                Err(e) => panic!("Could not read directory {} with error {}", p.display(), e),
            };
            for subdir in subdirs {
                let path = match subdir {
                    Ok(x) => x.path(),
                    Err(e) => panic!("Could not read subdir with error {}", e),
                };
                if path.file_name().unwrap() == ".git" {
                    continue;
                }
                if path.is_dir() && (! closed_dirs.contains(&path)){
                    if let Ok(repo) = Repository::open(&path) {
                        if repo.is_bare() {
                            if let Ok(worktrees) = repo.worktrees() {
                                for w in worktrees.iter() {
                                    if let Some(w) = w {
                                        let mut pathb = PathBuf::new();
                                        pathb.push(&path);
                                        pathb.push(w);
                                        projects.push(pathb);
                                    }
                                }
                            }
                            continue;
                        }
                    }
                    closed_dirs.insert(path.clone());
                    projects.push(path);
                }
            }
        }
        projects.sort();

        return Config {
            projects,
            skip_current: conf.skip_current,
        };
    }
}
