use directories::BaseDirs;
use std::{path::PathBuf, str::from_utf8};
use toml;
use serde::Deserialize;
use git2::Repository;
use std::collections::HashSet;
use tmux_interface::{tmux::Tmux, list_sessions::ListSessions, SwitchClient, NewSession, AttachSession};
use skim::prelude::*;
use regex::{Regex, RegexBuilder};

#[derive(Deserialize, Debug)]
struct Config {
    project_homes: Vec<String>,
    projects: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct Session {
    name: String,
    window_count: u32,
    date_created: String,
    attached: bool,
}

#[derive(Debug)]
struct Project {
    path: PathBuf,
    path_name: String,
    session_name: String,
    session: Option<Session>,
}

impl Project {
    fn new(path: PathBuf, sessions: &Vec<Session>) -> Project {
        let path_name = path.display().to_string();
        let path_name_sanitised = path_name.replace(".", "_");
        for (i, s) in sessions.iter().enumerate() {
            if path_name == s.name {
                let s = sessions[i].clone();
                return Project {
                    path,
                    path_name,
                    session_name: path_name_sanitised,
                    session: Some(s) }
            }
        }

        // no session recorded.
        return Project {
            path,
            path_name,
            session_name: path_name_sanitised,
            session: None };
    }
}

impl SkimItem for Project {
    fn text(&self) -> Cow<str> {
        return Cow::Borrowed(&self.path_name);
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let s = match &self.session {
            Some(session) => format!("Test preview for {}, which has session name {}",
                        self.path_name, session.name),
            None => format!("Test preview for {}, which has no existing session",
                        self.path_name),
        };
        return ItemPreview::Text(s.to_owned());
    }
}

fn attach_from_outside_tmux(target_session: &str) {
    Tmux::with_command(NewSession::new()
                       .session_name("base")
                       .detached()
                       .build()).output().expect("failed to run tmux command");
    Tmux::with_command(AttachSession::new().target_session("base").build()).output().expect("failed to attach non-attached client to session");
    let output = std::process::Command::new("tmux")
        .arg("list-clients")
        .arg("-F")
        .arg("'#{client_session} #{client_tty} #{client_activity}'")
        .output()
        .expect("could not execute tmux command")
        .stdout;

    let s_output = std::str::from_utf8(&output).expect("did not receive utf8 output from tmux");

    let re = Regex::new(r"^(\S+?) (\S+?) (\S+?)$").expect("failed building regex");
    let mut latest:u32 = 0;
    let mut client = String::new();
    for line in s_output.lines() {
        let x = re.captures(line).unwrap();
        if &x[0] != "base" { continue }
        let timestamp = x[2].parse::<u32>().unwrap();
        if  timestamp > latest {
            client = x[1].to_string();
            latest = timestamp;
        }
    }
    std::process::Command::new("tmux")
        .arg("-c")
        .arg(client)
        .arg("switch-client")
        .arg("-t")
        .arg(target_session)
        .output()
        .expect("could not execute command");
}

fn main() {
    let project_paths = generate_project_dirs();
    let sessions = get_tmux_session_info();
    // println!("{:?}", sessions);

    let projects:Vec<_> = project_paths.into_iter().map(|path| Project::new(path, &sessions)).collect();

    let skim_opts = SkimOptionsBuilder::default()
        .multi(false)
        .build()
        .unwrap();

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

    for p in projects.into_iter() {
        tx.send(Arc::new(p)).expect("failed to send project {:?} to skim");
    }

    drop(tx);

    let Some(selection) = Skim::run_with(&skim_opts, Some(rx)) else {
        eprintln!("Internal Skim Error");
        return;
    };

    if selection.final_event == Event::EvActAbort {
        return;
    }

    // let selected_skim_items = Skim::run_with(&skim_opts, Some(rx))
    //     .map(|out| out.selected_items)
    //     .unwrap_or_else(Vec::new);

    let selected_proj = selection.selected_items.iter()
        .map(|selected| (**selected).as_any().downcast_ref::<Project>().unwrap().to_owned())
        .collect::<Vec<_>>()[0];


    println!("Attempting to switch to project: {:?}", selected_proj);

    // N.B. attaching with tmux_interface seems to create errors with the terminal getting confused
    // lets do it with a command instead.

    if let Err(e) = std::env::var("TMUX") {
        match e {
            std::env::VarError::NotPresent => attach_from_outside_tmux(&selected_proj.path_name, &selected_proj.session_name, selected_proj.session.is_some()), // we are not in tmux right now
            std::env::VarError::NotUnicode(_) => panic!("$TMUX is not unicode! this cannot be handled. exiting."),
        };
    } else {
        if selected_proj.session.is_none() {
            println!("Session does not exist yet! making one.");
            match Tmux::with_command(NewSession::new()
                                     .start_directory(&selected_proj.path_name)
                                     .session_name(&selected_proj.session_name)
                                     .detached()
                                     .build()).output() {
                Ok(s) => s,
                Err(e) => panic!("failed running tmux command with error {}", e)
            };
        }
        std::process::Command::new("tmux")
            .arg("switch-client")
            .arg("-t")
            .arg(&selected_proj.session_name)
            .output()
            .expect("could not execute command");
    }
    if selected_proj.session.is_none() {
        // println!("Session does not exist yet! making one.");
        let new_session = match Tmux::with_command(NewSession::new()
                           .start_directory(&selected_proj.path_name)
                           .session_name(&selected_proj.path_name)
                           .detached()
                           .build()).output() {
                            Ok(s) => s,
                            Err(e) => panic!("failed running tmux command with error {}", e),
                        };
        dbg!(&new_session);
    }

    // println!("Attempting to switch to project: {}", selected_proj.path_name);

    // N.B. attaching with tmux_interface seems to create errors with the terminal getting confused
    // lets do it with a command instead.


    if let Err(e) = std::env::var("TMUX") {
        match e {
            std::env::VarError::NotPresent => attach_from_outside_tmux(&selected_proj.path_name), // we are not in tmux right now
            std::env::VarError::NotUnicode(_) => panic!("$TMUX is not unicode! this cannot be handled. exiting."),
        };
    } else {
            std::process::Command::new("tmux")
                .arg("switch-client")
                .arg("-t")
                .arg(&selected_proj.path_name)
                .output()
                .expect("could not execute command");
        // if let Err(e) = Tmux::with_command(SwitchClient::new().target_session(&selected_proj.path_name).build()).output() {
        //     eprintln!("could not switch client with error {}", e);
        // }
    }

}

fn get_tmux_session_info() -> Vec<Session> {
    let cmd_output = Tmux::with_command(ListSessions::new().build()).output().expect("could not run tmux command").0.stdout;
    let s = std::str::from_utf8(&cmd_output).expect("could not convert output from utf8.");

    let re_data = RegexBuilder::new(r"(\S*?): (\d+) windows \(created (.*?)\)")
        .multi_line(true)
        .build()
        .unwrap();
    let re_attached = Regex::new(r"\(attached\)").unwrap();

    let mut sessions = Vec::new();
    for line in s.lines() {
        let hits = re_data.captures(line).unwrap();
        let name = hits.get(1).unwrap().as_str().to_string();
        let window_count: u32 = hits.get(2).unwrap().as_str().parse().unwrap();
        let date_created = hits.get(3).unwrap().as_str().to_string();
        let attached = re_attached.is_match(line);

        sessions.push(Session {
            name,
            window_count,
            date_created,
            attached,
        });
    }
    return sessions;
}

fn generate_project_dirs() -> Vec<PathBuf> {
    let base_dirs = BaseDirs::new().unwrap();
    let conf_dir = BaseDirs::config_dir(&base_dirs);
    let conf_file_path = conf_dir.join("tps/config.toml");

    let conf_file_contents = match std::fs::read_to_string(&conf_file_path) {
        Ok(s) => s,
        Err(e) => panic!("could not read config file with error: {}", e),
    };

    let conf = match toml::from_str::<Config>(&conf_file_contents) {
        Ok(s) => s,
        Err(e) => panic!("could not parse config file with error: {}", e),
    };

    let mut projects:Vec<PathBuf> = Vec::new();

    let home_dir = BaseDirs::home_dir(&base_dirs);

    let mut open_dirs:Vec<PathBuf> = Vec::new();

    if let Some(raw_paths) = conf.projects {
        for path_raw in raw_paths.iter() {
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
    return projects;
}
