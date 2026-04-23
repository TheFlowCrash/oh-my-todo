use crate::application::bootstrap::AppContext;
use crate::application::commands::{
    AddTaskLogCommand, ArchiveSpaceCommand, ArchiveTaskCommand, CreateSpaceCommand,
    CreateTaskCommand, EditTaskCommand, PurgeSpaceCommand, PurgeTaskCommand, RenameSpaceCommand,
    RestoreSpaceCommand, RestoreTaskCommand, SetCurrentSpaceCommand, UpdateTaskStatusCommand,
};
use crate::application::error::AppError;
use crate::application::queries::{ListSpacesQuery, ListTasksQuery, ShowSpaceQuery, ShowTaskQuery};
use crate::cli::output;
use crate::cli::parser::{
    RootCommand, SpaceCommand, TaskCommand, TaskLogCommand, TaskStatusCommand, TodoCli,
};
use crate::tui::{self, LaunchOptions};

pub fn dispatch(context: &AppContext, cli: TodoCli) -> Result<(), AppError> {
    match cli.command {
        RootCommand::Doctor => {
            let report = context.maintenance_service.doctor()?;
            println!("{}", output::render_doctor_report(&report));
            Ok(())
        }
        RootCommand::Tui(args) => {
            let space_id = match args.space.as_deref() {
                Some(reference) => Some(context.space_service.resolve_space(reference, false)?.id),
                None => None,
            };

            tui::run_with_options(
                context,
                LaunchOptions {
                    space_id,
                    view: args.view.map(Into::into),
                    sort: args.sort.map(Into::into),
                },
            )
        }
        RootCommand::Space(space_args) => match space_args.command {
            SpaceCommand::Add(args) => {
                let space = context
                    .space_service
                    .create_space(CreateSpaceCommand { name: args.name })?;
                println!("{}", output::format_created_space(&space));
                launch_tui_if_needed(context, args.tui, Some(space.id), None, None)
            }
            SpaceCommand::List(args) => {
                let spaces = context.space_service.list_spaces(ListSpacesQuery {
                    include_archived: args.include_archived,
                })?;
                println!("{}", output::render_spaces(&spaces));
                Ok(())
            }
            SpaceCommand::Show(args) => {
                let details = context.space_service.show_space(ShowSpaceQuery {
                    space_ref: args.space_ref,
                })?;
                println!("{}", output::render_space(&details));
                Ok(())
            }
            SpaceCommand::Use(args) => {
                let space = context.space_service.use_space(SetCurrentSpaceCommand {
                    space_ref: args.space_ref,
                })?;
                println!("{}", output::format_used_space(&space));
                launch_tui_if_needed(context, args.tui, Some(space.id), None, None)
            }
            SpaceCommand::Rename(args) => {
                let space = context.space_service.rename_space(RenameSpaceCommand {
                    space_ref: args.space_ref,
                    new_name: args.new_name,
                })?;
                println!("{}", output::format_renamed_space(&space));
                launch_tui_if_needed(context, args.tui, Some(space.id), None, None)
            }
            SpaceCommand::Archive(args) => {
                let outcome = context.space_service.archive_space(ArchiveSpaceCommand {
                    space_ref: args.space_ref,
                })?;
                let space = outcome.root_space.expect("space archive returns space");
                println!("{}", output::format_archived_space(&space));
                launch_tui_if_needed(context, args.tui, Some(space.id), None, None)
            }
            SpaceCommand::Restore(args) => {
                let outcome = context.space_service.restore_space(RestoreSpaceCommand {
                    space_ref: args.space_ref,
                })?;
                let space = outcome.root_space.expect("space restore returns space");
                println!("{}", output::format_restored_space(&space));
                launch_tui_if_needed(context, args.tui, Some(space.id), None, None)
            }
            SpaceCommand::Purge(args) => {
                let _ = args.force;
                let outcome = context.space_service.purge_space(PurgeSpaceCommand {
                    space_ref: args.space_ref,
                })?;
                let space = outcome.root_space.expect("space purge returns prior space");
                println!("{}", output::format_purged_space(&space));
                Ok(())
            }
        },
        RootCommand::Task(task_args) => match task_args.command {
            TaskCommand::Add(args) => {
                let task = context.task_service.create_task(CreateTaskCommand {
                    title: args.title,
                    space_ref: args.space,
                    description: args.description,
                    parent_ref: args.parent,
                    status: args
                        .status
                        .map(Into::into)
                        .unwrap_or(crate::domain::TaskStatus::Todo),
                })?;
                let space = context.space_service.load_space(&task.space_id)?;
                println!("{}", output::format_created_task(&task, &space));
                launch_tui_if_needed(context, args.tui, Some(task.space_id), None, None)
            }
            TaskCommand::List(args) => {
                let result = context.task_service.list_tasks(ListTasksQuery {
                    space_ref: args.space,
                    view: args.view.map(Into::into),
                    sort: args.sort.map(Into::into),
                    allow_archived_space: false,
                })?;
                println!("{}", output::render_task_list(&result));
                Ok(())
            }
            TaskCommand::Show(args) => {
                let details = context.task_service.show_task(ShowTaskQuery {
                    task_ref: args.task_ref,
                })?;
                println!("{}", output::render_task(&details));
                Ok(())
            }
            TaskCommand::Edit(args) => {
                let task = context.task_service.edit_task(EditTaskCommand {
                    task_ref: args.task_ref,
                    title: args.title,
                    description: args.description,
                    clear_description: args.clear_desc,
                    status: args.status.map(Into::into),
                    parent_ref: args.parent,
                    clear_parent: args.clear_parent,
                    space_ref: args.space,
                })?;
                println!("{}", output::format_updated_task(&task));
                launch_tui_if_needed(context, args.tui, Some(task.space_id), None, None)
            }
            TaskCommand::Status(status_args) => match status_args.command {
                TaskStatusCommand::Set(args) => {
                    let task = context
                        .task_service
                        .set_task_status(UpdateTaskStatusCommand {
                            task_ref: args.task_ref,
                            status: args.status.into(),
                        })?;
                    println!("{}", output::format_task_status(&task));
                    launch_tui_if_needed(context, args.tui, Some(task.space_id), None, None)
                }
            },
            TaskCommand::Done(args) => {
                let task = context
                    .task_service
                    .set_task_status(UpdateTaskStatusCommand {
                        task_ref: args.task_ref,
                        status: crate::domain::TaskStatus::Done,
                    })?;
                println!("{}", output::format_task_status(&task));
                launch_tui_if_needed(context, args.tui, Some(task.space_id), None, None)
            }
            TaskCommand::Archive(args) => {
                let outcome = context.task_service.archive_task(ArchiveTaskCommand {
                    task_ref: args.task_ref,
                })?;
                let task = outcome.root_task.expect("task archive returns task");
                println!(
                    "{}",
                    output::format_archived_task(&task, outcome.affected_count)
                );
                launch_tui_if_needed(context, args.tui, Some(task.space_id), None, None)
            }
            TaskCommand::Restore(args) => {
                let outcome = context.task_service.restore_task(RestoreTaskCommand {
                    task_ref: args.task_ref,
                    status: args
                        .status
                        .map(Into::into)
                        .unwrap_or(crate::domain::TaskStatus::Todo),
                })?;
                let task = outcome.root_task.expect("task restore returns task");
                println!(
                    "{}",
                    output::format_restored_task(&task, outcome.affected_count)
                );
                launch_tui_if_needed(context, args.tui, Some(task.space_id), None, None)
            }
            TaskCommand::Log(log_args) => match log_args.command {
                TaskLogCommand::Add(args) => {
                    let task = context.task_service.add_task_log(AddTaskLogCommand {
                        task_ref: args.task_ref,
                        message: args.text,
                    })?;
                    println!("{}", output::format_logged_task(&task));
                    launch_tui_if_needed(context, args.tui, Some(task.space_id), None, None)
                }
            },
            TaskCommand::Purge(args) => {
                let _ = args.force;
                let outcome = context.task_service.purge_task(PurgeTaskCommand {
                    task_ref: args.task_ref,
                    recursive: args.recursive,
                })?;
                let task = outcome.root_task.expect("task purge returns prior task");
                println!(
                    "{}",
                    output::format_purged_task(&task, outcome.affected_count)
                );
                Ok(())
            }
        },
    }
}

fn launch_tui_if_needed(
    context: &AppContext,
    enabled: bool,
    space_id: Option<crate::domain::SpaceId>,
    view: Option<crate::domain::ViewMode>,
    sort: Option<crate::domain::SortMode>,
) -> Result<(), AppError> {
    if enabled {
        tui::run_with_options(
            context,
            LaunchOptions {
                space_id,
                view,
                sort,
            },
        )
    } else {
        Ok(())
    }
}
