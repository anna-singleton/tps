# Tmux project sessioniser

The aim of this program is to be able to create tmux sessions per project and 
effortlessly create new projects and switch between them effortlessly.

## Terminology

- **Project** - A single directory which `tps` recognises as a target for switching
to.

- **Project Home** - A directory which contains many projects. There are 2 types of 
project home, implicit and explicit. Explicit project homes are defined in a 
config file and are where projects are discovered from. Implicit project homes
are directories within another project home that actually contain multiple 
projects. The initial example of this is a bare git repo being used for 
git-worktrees. The program will automatically ignore a `.bare` directory in a 
git worktree, which is how I use it. The script I use is in my `.dotfiles` repo
here: [repo](https://github.com/anna-singleton/dotfiles/blob/main/opt/git-clone-worktree) 

- **Session** - A named tmux session which links to a project.

## Recommendations

I use neovim to edit code within each project, and I find that using a neovim
session restorer goes really nicely with this, as for reasons described below
restoring the state of a tmux session after a reboot is... tricky.


## Config

An example config file can be found below. On Linux, this is stored at 
`~/.config/tps/config.toml`

```toml
project_homes = [
"~/uni/",
"~/proj"
]

projects = [
"~/.dotfiles"
]

skip_current = true
sort_mode = "recent"
```

### Config Options
- `project_homes` is the parent directory of your projects.
- `projects` is other projects that live elsewhere, these are simply added to
the list and do not search.
- `skip_current` boolean for whether to exclude the current project
from the list (N.B. this functionality could be improved but currently just checks
current directory, so it will not work if you execute tps from a subdirectory of a
project).
- `sort_mode` is the ordering of the projects. Current options are `alphabetical`
and `recent`. `recent` requires a small cache to be stored, which is stored in your 
default cache directory under `tps/`, usually `~/.cache`.


## Pain Points

`tps` unfortunately cannot retain session data across machine reboots. I have
not the energy to figure out how to wire up tmux-resurrect or tmuxinator to do 
this nicely. PRs very welcome for this though :)
