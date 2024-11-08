mod config;
mod access_cache;

use access_cache::AccessCache;
use config::{Config, SortMode};
use itertools::Itertools;
use std::{env::current_dir, fs, path::PathBuf, process::exit};
use tmux_interface::{tmux::Tmux, list_sessions::ListSessions, NewSession};
use skim::prelude::*;
use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone)]
struct Session {
    name: String,
    _window_count: u32,
    _date_created: String,
    _attached: bool,
}

#[derive(Debug)]
struct Project {
    path: PathBuf,
    path_name: String,
    session_name: String,
    session: Option<Session>,
}

impl Project {
    fn new(path: PathBuf, sessions: &[Session]) -> Project {
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
        let s = fs::read_dir(&self.path)
            .unwrap()
            .flat_map(|x| x)
            .map(|x| format!("{}", x
                    .path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()))
            .join("\n");
        return ItemPreview::Text(s);
    }
}

fn attach_from_outside_tmux(_path_name: &str, _session_name: &str, _exists: bool) {
    eprintln!("attaching from outside tmux is currently WIP, please open a tmux session and then call tps.");
    // if exists {
    //     let output = std::process::Command::new("tmux")
    //         .arg("attach")
    //         .arg("-t")
    //         .arg(session_name)
    //         .spawn();
    //     println!("{:?}", output);
    // } else {
    //     std::process::Command::new("tmux")
    //         .arg("new-session")
    //         .arg("-c")
    //         .arg(path_name)
    //         .arg("-s")
    //         .arg(session_name)
    //         .output()
    //         .expect("could not execute tmux command");
    // }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) = Config::load() else {
        exit(1);
    };
    let mut access_cache = match config.sort_mode {
        SortMode::Alphabetical => AccessCache::load_blank(None, 10),
        SortMode::Recent => AccessCache::load_from_file(config.cache_path, 50)?,
    };
    let sessions = get_tmux_session_info();

    let mut projects:Vec<_> = config.projects
        .into_iter()
        .map(|path| Project::new(path, &sessions))
        .collect();

    match config.sort_mode {
        SortMode::Alphabetical => (),
        SortMode::Recent => projects.sort_by(|a, b| access_cache.cmp_projects_by_access_cache_time(a, b)),
    }

    let skim_opts = SkimOptionsBuilder::default()
        .multi(false)
        .preview(Some(""))
        .build()
        .unwrap();

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

    for p in projects.into_iter() {
        tx.send(Arc::new(p)).expect("failed to send project {:?} to skim");
    }

    drop(tx);

    let Some(selection) = Skim::run_with(&skim_opts, Some(rx)) else {
        eprintln!("Internal Skim Error");
        return Ok(());
    };

    if selection.final_event == Event::EvActAbort || selection.selected_items.is_empty() {
        return Ok(());
    }

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
    };

    access_cache.register_access(selected_proj);

    return Ok(());
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
            _window_count: window_count,
            _date_created: date_created,
            _attached: attached,
        });
    }
    return sessions;
}
