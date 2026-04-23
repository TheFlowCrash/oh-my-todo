use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "todo", version, about = "Local-first terminal task manager")]
pub struct TodoCli {
    #[command(subcommand)]
    pub command: RootCommand,
}

#[derive(Debug, Subcommand)]
pub enum RootCommand {
    Tui(TuiArgs),
    Task(TaskArgs),
    Space(SpaceArgs),
}

#[derive(Debug, Args)]
pub struct TuiArgs {
    #[arg(long)]
    pub space: Option<String>,
    #[arg(long)]
    pub view: Option<ViewArg>,
    #[arg(long)]
    pub sort: Option<SortArg>,
}

#[derive(Debug, Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommand,
}

#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    Add(TaskAddArgs),
    List(TaskListArgs),
    Show(TaskShowArgs),
    Edit(TaskEditArgs),
    Status(TaskStatusArgs),
    Done(TaskDoneArgs),
    Log(TaskLogArgs),
}

#[derive(Debug, Args)]
pub struct SpaceArgs {
    #[command(subcommand)]
    pub command: SpaceCommand,
}

#[derive(Debug, Subcommand)]
pub enum SpaceCommand {
    Add(SpaceAddArgs),
    List(SpaceListArgs),
    Show(SpaceShowArgs),
    Use(SpaceUseArgs),
    Rename(SpaceRenameArgs),
}

#[derive(Debug, Args)]
pub struct TaskAddArgs {
    pub title: String,
    #[arg(long)]
    pub space: Option<String>,
    #[arg(long = "desc")]
    pub description: Option<String>,
    #[arg(long)]
    pub parent: Option<String>,
    #[arg(long)]
    pub status: Option<ActiveTaskStatusArg>,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Args)]
pub struct TaskListArgs {
    #[arg(long)]
    pub space: Option<String>,
    #[arg(long)]
    pub view: Option<ViewArg>,
    #[arg(long)]
    pub sort: Option<SortArg>,
}

#[derive(Debug, Args)]
pub struct TaskShowArgs {
    pub task_ref: String,
}

#[derive(Debug, Args)]
pub struct TaskEditArgs {
    pub task_ref: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long = "desc", conflicts_with = "clear_desc")]
    pub description: Option<String>,
    #[arg(long = "clear-desc", conflicts_with = "description")]
    pub clear_desc: bool,
    #[arg(long)]
    pub status: Option<ActiveTaskStatusArg>,
    #[arg(long, conflicts_with = "clear_parent")]
    pub parent: Option<String>,
    #[arg(long = "clear-parent", conflicts_with = "parent")]
    pub clear_parent: bool,
    #[arg(long)]
    pub space: Option<String>,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Args)]
pub struct TaskStatusArgs {
    #[command(subcommand)]
    pub command: TaskStatusCommand,
}

#[derive(Debug, Subcommand)]
pub enum TaskStatusCommand {
    Set(TaskStatusSetArgs),
}

#[derive(Debug, Args)]
pub struct TaskStatusSetArgs {
    pub task_ref: String,
    pub status: ActiveTaskStatusArg,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Args)]
pub struct TaskDoneArgs {
    pub task_ref: String,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Args)]
pub struct TaskLogArgs {
    #[command(subcommand)]
    pub command: TaskLogCommand,
}

#[derive(Debug, Subcommand)]
pub enum TaskLogCommand {
    Add(TaskLogAddArgs),
}

#[derive(Debug, Args)]
pub struct TaskLogAddArgs {
    pub task_ref: String,
    pub text: String,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Args)]
pub struct SpaceAddArgs {
    pub name: String,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Args)]
pub struct SpaceListArgs {
    #[arg(long = "all")]
    pub include_archived: bool,
}

#[derive(Debug, Args)]
pub struct SpaceShowArgs {
    pub space_ref: String,
}

#[derive(Debug, Args)]
pub struct SpaceUseArgs {
    pub space_ref: String,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Args)]
pub struct SpaceRenameArgs {
    pub space_ref: String,
    pub new_name: String,
    #[arg(long)]
    pub tui: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ViewArg {
    Todo,
    Archive,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortArg {
    Created,
    Updated,
    Status,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ActiveTaskStatusArg {
    Todo,
    #[value(name = "in_progress")]
    InProgress,
    Done,
}

impl From<ViewArg> for crate::domain::ViewMode {
    fn from(value: ViewArg) -> Self {
        match value {
            ViewArg::Todo => Self::Todo,
            ViewArg::Archive => Self::Archive,
            ViewArg::All => Self::All,
        }
    }
}

impl From<SortArg> for crate::domain::SortMode {
    fn from(value: SortArg) -> Self {
        match value {
            SortArg::Created => Self::Created,
            SortArg::Updated => Self::Updated,
            SortArg::Status => Self::Status,
            SortArg::Manual => Self::Manual,
        }
    }
}

impl From<ActiveTaskStatusArg> for crate::domain::TaskStatus {
    fn from(value: ActiveTaskStatusArg) -> Self {
        match value {
            ActiveTaskStatusArg::Todo => Self::Todo,
            ActiveTaskStatusArg::InProgress => Self::InProgress,
            ActiveTaskStatusArg::Done => Self::Done,
        }
    }
}
