# oh-my-todo

Local-first terminal task manager built with Rust, Ratatui, Clap, and RON.

## Current highlights

- Shared application core for both CLI and TUI
- Space, task, archive, restore, and purge lifecycles
- Mouse-first TUI with task tree + details workspace
- Phase 5 TUI enhancements:
  - task filter dialog
  - help overlay
  - active/all space toggle
  - TUI space archive/restore/purge
  - manual-sort task reordering with `Move Up` / `Move Down`

## Build and run

```bash
cargo run
```

- `cargo run` launches the TUI
- `cargo run -- task list` runs a CLI command
- `cargo test` runs the automated test suite

## Useful commands

```bash
todo
todo tui --space personal --view archive --sort manual
todo space add personal
todo space use personal
todo task add "Run 5km"
todo task list --view all --sort manual
```

## TUI tips

- Click `Filter` in the top bar or press `/` to filter tasks by title, description, logs, or ids
- Click `Help` in the footer or press `?` to open the in-app guide
- Use `Active` / `All` in the `Spaces` row to reveal archived spaces
- Archived spaces are browsable but read-only until restored
- When sort mode is `manual`, use `Move Up` / `Move Down` in `Details` to reorder siblings
- `Ctrl+C` remains the global safe exit
