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
git-worktrees.

- **Session** - A named tmux session which links to a project.

## Recommendations

I use neovim to edit code within each project, and I find that using a neovim
session restorer goes really nicely with this, as for reasons described below
restoring the state of a tmux session after a reboot is... tricky.


## Pain Points

`tps` unfortunately cannot retain session data across machine reboots. I have
not the energy to figure out how to wire up tmux-resurrect or tmuxinator to do 
this nicely. PRs very welcome for this though :)
