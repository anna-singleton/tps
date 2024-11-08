use std::{collections::HashSet, env::current_dir, fs::exists, io::stdin, path::PathBuf};

use directories::BaseDirs;
use git2::Repository;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ConfigFileFormat {
    project_homes: Vec<String>,

    #[serde(default)]
    projects: Vec<String>,

    #[serde(default)]
    skip_current: bool,

    sort_mode: Option<String>,

    cache_path: Option<String>,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub enum SortMode {
    #[default]
    Alphabetical,
    Recent
}

impl From<&str> for SortMode {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "alphabetical" => Self::Alphabetical,
            "recent" => Self::Recent,
            _ => {
                eprintln!("Could not parse SortMode, please check spelling. Accepted \
                          Strings: 'alphabetical', 'recent'. Defaulting to alphabetical");
                Self::Alphabetical
            }
        }
    }
}

pub struct Config {
    pub projects: Vec<PathBuf>,
    pub skip_current: bool,
    pub sort_mode: SortMode,
    pub cache_path: PathBuf,
}

impl Config {
    pub fn load() -> Option<Self> {
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

        while let Some(p) = open_dirs.pop() {
            closed_dirs.insert(p.clone());
            if ! exists(&p).unwrap_or(false) {
                continue;
            }
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
                                for w in worktrees.iter().flatten() {
                                    let mut pathb = PathBuf::new();
                                    pathb.push(&path);
                                    pathb.push(w);
                                    projects.push(pathb);
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
        let skip_current = conf.skip_current;

        if skip_current {
            projects.retain(|path| *path != current_dir().expect("couldnt get current path"));
        }

        if projects.is_empty() {
            eprintln!("No projects found. Press ENTER to exit.");
            let mut _s = String::new();
            stdin().read_line(&mut _s).expect("couldnt read from stdin.");
            return None;
        }

        projects.sort();

        let path: PathBuf = if let Some(raw_path) = conf.cache_path {
            raw_path.into()
        } else {
            let cache_dir = base_dirs.cache_dir();
            cache_dir.join("tps/access_cache")

        };

        let sort_mode = if let Some(mode_str) = conf.sort_mode {
            mode_str.as_str().into()
        } else {
            SortMode::default()
        };

        return Some(Config {
            projects,
            skip_current,
            sort_mode,
            cache_path: path
        });
    }
}
