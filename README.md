# oh-my-todo

Local-first terminal task manager built with Rust, Ratatui, Clap, and RON.

Install from crates.io:

```bash
cargo install oh-my-todo
```

After installation, run it with:

```bash
todo
```

## Current highlights

- Shared application core for both CLI and TUI
- Space, task, archive, restore, and purge lifecycles
- Mouse-first TUI with TODO, Inspector, and Detail panels
- Phase 5 TUI enhancements:
  - task filter dialog
  - help overlay
  - top-bar space manager popup with active/all toggle
  - TUI space archive/restore/purge
  - manual-sort task reordering with `Move Up` / `Move Down`

## Build and run locally

```bash
cargo run
```

- `cargo run` launches the TUI
- `cargo run -- task list` runs a CLI command
- `cargo test` runs the automated test suite

## Data directory

- By default, app data is stored in the OS-specific local data directory for `oh-my-todo`
- Set `OH_MY_TODO_DATA_DIR` to override the storage root during local runs, tests, or portable usage

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
- Click the `Space: ...` button in the top bar to open the space manager popup
- Click `Help` in the footer or press `?` to open the in-app guide
- Use `Active` / `All` inside the space manager to reveal archived spaces
- Archived spaces are browsable but read-only until restored
- When sort mode is `manual`, use `Move Up` / `Move Down` in `Inspector` to reorder siblings
- `Ctrl+C` remains the global safe exit
