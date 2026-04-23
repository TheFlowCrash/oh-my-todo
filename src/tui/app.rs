use crate::application::bootstrap::AppContext;
use crate::application::commands::{
    AddTaskLogCommand, CreateSpaceCommand, CreateTaskCommand, EditTaskCommand, PurgeTaskCommand,
    RenameSpaceCommand, RestoreTaskCommand, SetCurrentSpaceCommand, UpdateTaskStatusCommand,
};
use crate::application::error::AppError;
use crate::application::queries::{
    ListSpacesQuery, ListTasksQuery, SpaceSummary, TaskDetails, TaskListResult,
};
use crate::domain::{
    FocusArea, SortMode, SpaceId, SpaceViewMemory, Task, TaskId, TaskStatus, TuiMemory, ViewMode,
};
use crate::tui::LaunchOptions;
use crate::tui::input::TextInput;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct VisibleTaskEntry {
    pub task: Task,
    pub depth: usize,
    pub child_count: usize,
    pub is_expanded: bool,
}

#[derive(Debug, Clone)]
pub enum FormModal {
    Space(SpaceFormState),
    Task(TaskFormState),
    Log(LogFormState),
}

#[derive(Debug, Clone)]
pub enum ConfirmModal {
    PurgeTask(PurgeTaskConfirmState),
}

#[derive(Debug, Clone)]
pub enum Mode {
    Browse,
    Form(FormModal),
    Confirm(ConfirmModal),
}

#[derive(Debug, Clone)]
pub enum SpaceFormMode {
    Create,
    Rename { space_id: SpaceId },
}

#[derive(Debug, Clone)]
pub struct SpaceFormState {
    pub mode: SpaceFormMode,
    pub name: TextInput,
}

#[derive(Debug, Clone)]
pub enum TaskFormMode {
    CreateRoot,
    CreateChild { parent_id: TaskId },
    Edit { task_id: TaskId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskFormField {
    Title,
    Status,
    Description,
}

#[derive(Debug, Clone)]
pub struct TaskFormState {
    pub mode: TaskFormMode,
    pub focus: TaskFormField,
    pub title: TextInput,
    pub description: TextInput,
    pub status: TaskStatus,
}

#[derive(Debug, Clone)]
pub struct LogFormState {
    pub task_id: TaskId,
    pub task_title: String,
    pub input: TextInput,
}

#[derive(Debug, Clone)]
pub struct PurgeTaskConfirmState {
    pub task_id: TaskId,
    pub task_title: String,
    pub affected_count: usize,
    pub requires_phrase: bool,
    pub phrase: TextInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseTarget {
    SwitchView(ViewMode),
    CycleSort,
    SwitchSpace(usize),
    OpenSpaceCreate,
    OpenSpaceRename,
    SelectTask(usize),
    ToggleTask(TaskId),
    CreateTask,
    CreateSubtask,
    CloseDetails,
    EditTask,
    SetTaskStatus(TaskStatus),
    AddLog,
    ArchiveTask,
    RestoreTask,
    OpenPurgeTask,
    SpaceFormInput,
    SpaceFormSave,
    SpaceFormCancel,
    TaskFormTitle,
    TaskFormDescription,
    TaskFormStatus(TaskStatus),
    TaskFormSave,
    TaskFormCancel,
    LogFormInput,
    LogFormSave,
    LogFormCancel,
    ConfirmPhraseInput,
    ConfirmCancel,
    ConfirmPurge,
}

#[derive(Debug, Clone)]
pub struct Hitbox {
    pub rect: Rect,
    pub target: MouseTarget,
}

#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub hitboxes: Vec<Hitbox>,
    pub frame_area: Option<Rect>,
    pub task_tree_viewport: Option<Rect>,
    pub details_viewport: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct TuiApp {
    pub should_quit: bool,
    pub mode: Mode,
    pub focus_area: FocusArea,
    pub return_focus: FocusArea,
    pub current_space_id: Option<SpaceId>,
    pub current_view: ViewMode,
    pub current_sort: SortMode,
    pub spaces: Vec<SpaceSummary>,
    pub space_index: usize,
    pub task_result: Option<TaskListResult>,
    pub visible_tasks: Vec<VisibleTaskEntry>,
    pub task_list_state: ListState,
    pub details: Option<TaskDetails>,
    pub details_scroll: usize,
    pub tui_memory: TuiMemory,
    pub status_message: Option<String>,
    pub ui: UiState,
}

impl TuiApp {
    pub fn new(context: &AppContext, options: LaunchOptions) -> Result<Self, AppError> {
        let mut state = context.app_state_service.load()?;

        if let Some(space_id) = options.space_id {
            state.current_space_id = Some(space_id);
        }
        if let Some(view) = options.view {
            state.current_view = view;
        }
        if let Some(sort) = options.sort {
            state.current_sort = sort;
        }

        let mut app = Self {
            should_quit: false,
            mode: Mode::Browse,
            focus_area: state.tui_memory.focus_area,
            return_focus: state.tui_memory.focus_area,
            current_space_id: state.current_space_id,
            current_view: state.current_view,
            current_sort: state.current_sort,
            spaces: Vec::new(),
            space_index: state.tui_memory.spaces_cursor,
            task_result: None,
            visible_tasks: Vec::new(),
            task_list_state: ListState::default(),
            details: None,
            details_scroll: 0,
            tui_memory: state.tui_memory,
            status_message: None,
            ui: UiState::default(),
        };

        app.reload(context)?;
        app.persist(context)?;
        Ok(app)
    }

    pub fn handle_key(&mut self, context: &AppContext, key: KeyEvent) -> Result<bool, AppError> {
        if is_global_quit_shortcut(key) {
            self.should_quit = true;
            return Ok(true);
        }

        let result = match self.mode.clone() {
            Mode::Browse => Ok(false),
            Mode::Form(form) => self.handle_form_key(context, key, form),
            Mode::Confirm(confirm) => self.handle_confirm_key(context, key, confirm),
        };

        match result {
            Ok(changed) => Ok(changed),
            Err(error) => {
                self.status_message = Some(match error.hint() {
                    Some(hint) => format!("{error} | {hint}"),
                    None => error.to_string(),
                });
                Ok(true)
            }
        }
    }

    pub fn handle_mouse(
        &mut self,
        context: &AppContext,
        mouse: MouseEvent,
    ) -> Result<bool, AppError> {
        let position = Position::new(mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_click(context, position),
            MouseEventKind::ScrollDown => self.handle_scroll(context, position, 3),
            MouseEventKind::ScrollUp => self.handle_scroll(context, position, -3),
            _ => Ok(false),
        }
    }

    pub fn persist(&mut self, context: &AppContext) -> Result<(), AppError> {
        self.sync_memory();
        context.app_state_service.update(|state| {
            state.current_space_id = self.current_space_id.clone();
            state.current_view = self.current_view;
            state.current_sort = self.current_sort;
            state.tui_memory = self.tui_memory.clone();
        })?;
        Ok(())
    }

    pub fn is_narrow(&self, width: u16) -> bool {
        width < 100
    }

    pub fn selected_task_id(&self) -> Option<TaskId> {
        self.task_list_state
            .selected()
            .and_then(|index| self.visible_tasks.get(index))
            .map(|entry| entry.task.id.clone())
    }

    pub fn selected_task(&self) -> Option<&VisibleTaskEntry> {
        self.task_list_state
            .selected()
            .and_then(|index| self.visible_tasks.get(index))
    }

    pub fn current_space(&self) -> Option<&SpaceSummary> {
        self.current_space_id
            .as_ref()
            .and_then(|space_id| self.spaces.iter().find(|space| &space.space.id == space_id))
    }

    pub fn task_tree_empty_message(&self) -> &'static str {
        if self.spaces.is_empty() {
            "No spaces yet. Click + New to create your first space."
        } else {
            "No tasks in this view. Click + Task to create one."
        }
    }

    pub fn help_text(&self) -> String {
        match &self.mode {
            Mode::Form(FormModal::Space(_)) => {
                "Click field to edit | Click Save or Cancel | Ctrl+C quit".to_owned()
            }
            Mode::Form(FormModal::Task(_)) => {
                "Click fields to edit | Click status chips | Click Save or Cancel | Ctrl+C quit"
                    .to_owned()
            }
            Mode::Form(FormModal::Log(_)) => {
                "Type in the message box | Click Save or Cancel | Ctrl+C quit".to_owned()
            }
            Mode::Confirm(_) => {
                "Type purge if required | Click Cancel or Purge | Ctrl+C quit".to_owned()
            }
            Mode::Browse => match self.focus_area {
                FocusArea::Spaces => {
                    "Click a space to switch | Click + New or Rename | Ctrl+C quit".to_owned()
                }
                FocusArea::TaskTree => {
                    "Click task rows to select | Click arrows to fold | Scroll to browse | Ctrl+C quit"
                        .to_owned()
                }
                FocusArea::Details => {
                    "Click action buttons | Scroll details | Ctrl+C quit".to_owned()
                }
            },
        }
    }

    fn handle_form_key(
        &mut self,
        context: &AppContext,
        key: KeyEvent,
        form: FormModal,
    ) -> Result<bool, AppError> {
        match form {
            FormModal::Space(mut form) => {
                let _ = context;
                form.name.handle_key(key);
                self.mode = Mode::Form(FormModal::Space(form));
                Ok(true)
            }
            FormModal::Task(mut form) => {
                let _ = context;
                match form.focus {
                    TaskFormField::Title => form.title.handle_key(key),
                    TaskFormField::Description => form.description.handle_key(key),
                    TaskFormField::Status => {}
                }
                self.mode = Mode::Form(FormModal::Task(form));
                Ok(true)
            }
            FormModal::Log(mut form) => {
                let _ = context;
                form.input.handle_key(key);
                self.mode = Mode::Form(FormModal::Log(form));
                Ok(true)
            }
        }
    }

    fn handle_confirm_key(
        &mut self,
        context: &AppContext,
        key: KeyEvent,
        confirm: ConfirmModal,
    ) -> Result<bool, AppError> {
        match confirm {
            ConfirmModal::PurgeTask(mut confirm) => {
                let _ = context;
                if confirm.requires_phrase {
                    confirm.phrase.handle_key(key);
                }
                self.mode = Mode::Confirm(ConfirmModal::PurgeTask(confirm));
                Ok(true)
            }
        }
    }

    pub fn begin_frame(&mut self) {
        self.ui = UiState::default();
    }

    pub fn set_frame_area(&mut self, rect: Rect) {
        self.ui.frame_area = Some(rect);
    }

    pub fn register_hitbox(&mut self, rect: Rect, target: MouseTarget) {
        self.ui.hitboxes.push(Hitbox { rect, target });
    }

    pub fn set_task_tree_viewport(&mut self, rect: Rect) {
        self.ui.task_tree_viewport = Some(rect);
    }

    pub fn set_details_viewport(&mut self, rect: Rect) {
        self.ui.details_viewport = Some(rect);
    }

    fn handle_click(&mut self, context: &AppContext, position: Position) -> Result<bool, AppError> {
        let Some(hitbox) = self.hitbox_at(position) else {
            return Ok(false);
        };

        self.apply_mouse_target(context, hitbox, position)
    }

    fn handle_scroll(
        &mut self,
        context: &AppContext,
        position: Position,
        delta: isize,
    ) -> Result<bool, AppError> {
        if let Some(viewport) = self.ui.task_tree_viewport {
            if viewport.contains(position) {
                self.focus_area = FocusArea::TaskTree;
                self.scroll_task_tree(delta);
                self.refresh_details(context)?;
                return Ok(true);
            }
        }

        if let Some(viewport) = self.ui.details_viewport {
            if viewport.contains(position) {
                self.focus_area = FocusArea::Details;
                self.scroll_details(delta);
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn apply_mouse_target(
        &mut self,
        context: &AppContext,
        hitbox: Hitbox,
        position: Position,
    ) -> Result<bool, AppError> {
        match hitbox.target {
            MouseTarget::SwitchView(view) => {
                self.focus_area = FocusArea::TaskTree;
                self.set_view(context, view)
            }
            MouseTarget::CycleSort => {
                self.focus_area = FocusArea::TaskTree;
                self.cycle_sort(context)
            }
            MouseTarget::SwitchSpace(index) => {
                self.focus_area = FocusArea::Spaces;
                self.switch_space_to_index(context, index)
            }
            MouseTarget::OpenSpaceCreate => {
                self.focus_area = FocusArea::Spaces;
                self.open_space_form_create();
                Ok(true)
            }
            MouseTarget::OpenSpaceRename => {
                self.focus_area = FocusArea::Spaces;
                self.open_space_form_rename();
                Ok(true)
            }
            MouseTarget::SelectTask(index) => {
                let width = self
                    .ui
                    .frame_area
                    .map(|rect| rect.width)
                    .unwrap_or_default();
                self.focus_area = if self.is_narrow(width) {
                    FocusArea::Details
                } else {
                    FocusArea::TaskTree
                };
                self.task_list_state.select(Some(index));
                self.refresh_details(context)?;
                Ok(true)
            }
            MouseTarget::ToggleTask(task_id) => {
                self.focus_area = FocusArea::TaskTree;
                if self
                    .space_memory()
                    .is_some_and(|memory| memory.expanded_task_ids.contains(&task_id))
                {
                    self.collapse_task(task_id);
                } else {
                    self.expand_task(task_id);
                }
                self.reload(context)?;
                Ok(true)
            }
            MouseTarget::CreateTask => {
                self.focus_area = FocusArea::Details;
                self.open_task_form_create_root();
                Ok(true)
            }
            MouseTarget::CreateSubtask => {
                self.focus_area = FocusArea::Details;
                self.open_task_form_create_child();
                Ok(true)
            }
            MouseTarget::EditTask => {
                self.focus_area = FocusArea::Details;
                self.open_task_form_edit();
                Ok(true)
            }
            MouseTarget::SetTaskStatus(status) => {
                self.focus_area = FocusArea::Details;
                self.set_selected_task_status(context, status)
            }
            MouseTarget::AddLog => {
                self.focus_area = FocusArea::Details;
                self.open_log_form();
                Ok(true)
            }
            MouseTarget::ArchiveTask => {
                self.focus_area = FocusArea::Details;
                self.archive_selected_task(context)
            }
            MouseTarget::RestoreTask => {
                self.focus_area = FocusArea::Details;
                self.restore_selected_task(context)
            }
            MouseTarget::OpenPurgeTask => {
                self.focus_area = FocusArea::Details;
                self.open_purge_confirm(context)?;
                Ok(true)
            }
            MouseTarget::CloseDetails => {
                self.focus_area = FocusArea::TaskTree;
                Ok(true)
            }
            MouseTarget::SpaceFormInput => {
                if let Mode::Form(FormModal::Space(mut form)) = self.mode.clone() {
                    set_single_line_cursor(&mut form.name, hitbox.rect, position);
                    self.mode = Mode::Form(FormModal::Space(form));
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::SpaceFormSave => {
                if let Mode::Form(FormModal::Space(form)) = self.mode.clone() {
                    self.submit_space_form(context, form)?;
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::SpaceFormCancel => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::TaskFormTitle => {
                if let Mode::Form(FormModal::Task(mut form)) = self.mode.clone() {
                    form.focus = TaskFormField::Title;
                    set_single_line_cursor(&mut form.title, hitbox.rect, position);
                    self.mode = Mode::Form(FormModal::Task(form));
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::TaskFormDescription => {
                if let Mode::Form(FormModal::Task(mut form)) = self.mode.clone() {
                    form.focus = TaskFormField::Description;
                    set_multiline_cursor(&mut form.description, hitbox.rect, position);
                    self.mode = Mode::Form(FormModal::Task(form));
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::TaskFormStatus(status) => {
                if let Mode::Form(FormModal::Task(mut form)) = self.mode.clone() {
                    form.focus = TaskFormField::Status;
                    form.status = status;
                    self.mode = Mode::Form(FormModal::Task(form));
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::TaskFormSave => {
                if let Mode::Form(FormModal::Task(form)) = self.mode.clone() {
                    self.submit_task_form(context, form)?;
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::TaskFormCancel => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::LogFormInput => {
                if let Mode::Form(FormModal::Log(mut form)) = self.mode.clone() {
                    set_multiline_cursor(&mut form.input, hitbox.rect, position);
                    self.mode = Mode::Form(FormModal::Log(form));
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::LogFormSave => {
                if let Mode::Form(FormModal::Log(form)) = self.mode.clone() {
                    self.submit_log_form(context, form)?;
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::LogFormCancel => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::ConfirmPhraseInput => {
                if let Mode::Confirm(ConfirmModal::PurgeTask(mut confirm)) = self.mode.clone() {
                    if confirm.requires_phrase {
                        set_single_line_cursor(&mut confirm.phrase, hitbox.rect, position);
                        self.mode = Mode::Confirm(ConfirmModal::PurgeTask(confirm));
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            MouseTarget::ConfirmCancel => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::ConfirmPurge => {
                if let Mode::Confirm(ConfirmModal::PurgeTask(confirm)) = self.mode.clone() {
                    self.submit_purge_confirm(context, confirm)?;
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    fn hitbox_at(&self, position: Position) -> Option<Hitbox> {
        self.ui
            .hitboxes
            .iter()
            .rev()
            .find(|hitbox| hitbox.rect.contains(position))
            .cloned()
    }

    fn open_space_form_create(&mut self) {
        self.return_focus = self.focus_area;
        self.mode = Mode::Form(FormModal::Space(SpaceFormState {
            mode: SpaceFormMode::Create,
            name: TextInput::single_line(""),
        }));
    }

    fn open_space_form_rename(&mut self) {
        if let Some(space) = self
            .current_space()
            .map(|space| (space.space.id.clone(), space.space.name.clone()))
        {
            self.return_focus = self.focus_area;
            self.mode = Mode::Form(FormModal::Space(SpaceFormState {
                mode: SpaceFormMode::Rename { space_id: space.0 },
                name: TextInput::single_line(space.1),
            }));
        } else {
            self.status_message = Some("No active space to rename.".to_owned());
        }
    }

    fn open_task_form_create_root(&mut self) {
        self.return_focus = self.focus_area;
        self.mode = Mode::Form(FormModal::Task(TaskFormState {
            mode: TaskFormMode::CreateRoot,
            focus: TaskFormField::Title,
            title: TextInput::single_line(""),
            description: TextInput::multiline(""),
            status: TaskStatus::Todo,
        }));
    }

    fn open_task_form_create_child(&mut self) {
        if let Some(task_id) = self.selected_task_id() {
            if let Some(memory) = self.space_memory_mut() {
                if !memory.expanded_task_ids.contains(&task_id) {
                    memory.expanded_task_ids.push(task_id.clone());
                }
            }
            self.return_focus = self.focus_area;
            self.mode = Mode::Form(FormModal::Task(TaskFormState {
                mode: TaskFormMode::CreateChild { parent_id: task_id },
                focus: TaskFormField::Title,
                title: TextInput::single_line(""),
                description: TextInput::multiline(""),
                status: TaskStatus::Todo,
            }));
        } else {
            self.status_message = Some("Select a task first to create a subtask.".to_owned());
        }
    }

    fn open_task_form_edit(&mut self) {
        if let Some(details) = self.details.as_ref() {
            self.return_focus = self.focus_area;
            self.mode = Mode::Form(FormModal::Task(TaskFormState {
                mode: TaskFormMode::Edit {
                    task_id: details.task.id.clone(),
                },
                focus: TaskFormField::Title,
                title: TextInput::single_line(&details.task.title),
                description: TextInput::multiline(
                    details.task.description.clone().unwrap_or_default(),
                ),
                status: details.task.status,
            }));
        } else {
            self.status_message = Some("Select a task first to edit it.".to_owned());
        }
    }

    fn open_log_form(&mut self) {
        if let Some(details) = self.details.as_ref() {
            self.return_focus = self.focus_area;
            self.mode = Mode::Form(FormModal::Log(LogFormState {
                task_id: details.task.id.clone(),
                task_title: details.task.title.clone(),
                input: TextInput::multiline(""),
            }));
        } else {
            self.status_message = Some("Select a task first to add a log.".to_owned());
        }
    }

    fn open_purge_confirm(&mut self, _context: &AppContext) -> Result<(), AppError> {
        if let Some((task_id, task_title, task_status)) = self.selected_task().map(|selected| {
            (
                selected.task.id.clone(),
                selected.task.title.clone(),
                selected.task.status,
            )
        }) {
            if !matches!(task_status, TaskStatus::Archived) {
                self.status_message =
                    Some("Only archived tasks can be purged. Archive the task first.".to_owned());
                return Ok(());
            }
            let affected_count = self.subtree_count(&task_id);
            self.return_focus = self.focus_area;
            self.mode = Mode::Confirm(ConfirmModal::PurgeTask(PurgeTaskConfirmState {
                task_id,
                task_title,
                affected_count,
                requires_phrase: affected_count > 1,
                phrase: TextInput::single_line(""),
            }));
        } else {
            self.status_message = Some("Select a task first to purge it.".to_owned());
        }
        Ok(())
    }

    fn submit_space_form(
        &mut self,
        context: &AppContext,
        form: SpaceFormState,
    ) -> Result<(), AppError> {
        match form.mode {
            SpaceFormMode::Create => {
                let created = context.space_service.create_space(CreateSpaceCommand {
                    name: form.name.value(),
                })?;
                context.space_service.use_space(SetCurrentSpaceCommand {
                    space_ref: created.id.as_str().to_owned(),
                })?;
                self.current_space_id = Some(created.id.clone());
                self.status_message = Some(format!("Created space {}.", created.name));
            }
            SpaceFormMode::Rename { space_id } => {
                let renamed = context.space_service.rename_space(RenameSpaceCommand {
                    space_ref: space_id.as_str().to_owned(),
                    new_name: form.name.value(),
                })?;
                self.status_message = Some(format!("Renamed space to {}.", renamed.name));
            }
        }

        self.close_modal(true);
        self.reload(context)?;
        Ok(())
    }

    fn submit_task_form(
        &mut self,
        context: &AppContext,
        form: TaskFormState,
    ) -> Result<(), AppError> {
        match form.mode {
            TaskFormMode::CreateRoot => {
                let created = context.task_service.create_task(CreateTaskCommand {
                    title: form.title.value(),
                    space_ref: None,
                    description: description_option(&form.description),
                    parent_ref: None,
                    status: form.status,
                })?;
                self.select_task_after_action(created.id.clone());
                self.status_message = Some(format!("Created task {}.", created.title));
            }
            TaskFormMode::CreateChild { parent_id } => {
                let created = context.task_service.create_task(CreateTaskCommand {
                    title: form.title.value(),
                    space_ref: None,
                    description: description_option(&form.description),
                    parent_ref: Some(parent_id.as_str().to_owned()),
                    status: form.status,
                })?;
                self.expand_task(parent_id);
                self.select_task_after_action(created.id.clone());
                self.status_message = Some(format!("Created task {}.", created.title));
            }
            TaskFormMode::Edit { task_id } => {
                let updated = context.task_service.edit_task(EditTaskCommand {
                    task_ref: task_id.as_str().to_owned(),
                    title: Some(form.title.value()),
                    description: description_option(&form.description),
                    clear_description: form.description.is_blank(),
                    status: Some(form.status),
                    parent_ref: None,
                    clear_parent: false,
                    space_ref: None,
                })?;
                self.select_task_after_action(updated.id.clone());
                self.status_message = Some(format!("Updated task {}.", updated.title));
            }
        }

        self.close_modal(true);
        self.reload(context)?;
        Ok(())
    }

    fn submit_log_form(
        &mut self,
        context: &AppContext,
        form: LogFormState,
    ) -> Result<(), AppError> {
        let updated = context.task_service.add_task_log(AddTaskLogCommand {
            task_ref: form.task_id.as_str().to_owned(),
            message: form.input.value(),
        })?;
        self.select_task_after_action(updated.id.clone());
        self.status_message = Some(format!("Added log to {}.", updated.title));
        self.close_modal(true);
        self.reload(context)?;
        Ok(())
    }

    fn submit_purge_confirm(
        &mut self,
        context: &AppContext,
        confirm: PurgeTaskConfirmState,
    ) -> Result<(), AppError> {
        if confirm.requires_phrase && confirm.phrase.value().trim() != "purge" {
            self.status_message = Some("Type `purge` to confirm this deletion.".to_owned());
            self.mode = Mode::Confirm(ConfirmModal::PurgeTask(confirm));
            return Ok(());
        }

        let purged = context.task_service.purge_task(PurgeTaskCommand {
            task_ref: confirm.task_id.as_str().to_owned(),
            recursive: confirm.affected_count > 1,
        })?;
        self.status_message = Some(format!("Purged {} task(s).", purged.affected_count));
        self.close_modal(true);
        self.reload(context)?;
        Ok(())
    }

    fn close_modal(&mut self, clear_message: bool) {
        self.mode = Mode::Browse;
        self.focus_area = self.return_focus;
        if clear_message && self.status_message.is_none() {
            self.status_message = None;
        }
    }

    fn set_view(&mut self, context: &AppContext, view: ViewMode) -> Result<bool, AppError> {
        self.current_view = view;
        self.reload(context)?;
        Ok(true)
    }

    fn cycle_sort(&mut self, context: &AppContext) -> Result<bool, AppError> {
        self.current_sort = match self.current_sort {
            SortMode::Created => SortMode::Updated,
            SortMode::Updated => SortMode::Status,
            SortMode::Status => SortMode::Manual,
            SortMode::Manual => SortMode::Created,
        };
        self.reload(context)?;
        Ok(true)
    }

    fn switch_space_to_index(
        &mut self,
        context: &AppContext,
        index: usize,
    ) -> Result<bool, AppError> {
        if let Some(space) = self.spaces.get(index) {
            context.space_service.use_space(SetCurrentSpaceCommand {
                space_ref: space.space.id.as_str().to_owned(),
            })?;
            self.current_space_id = Some(space.space.id.clone());
            self.space_index = index;
            self.reload(context)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn set_selected_task_status(
        &mut self,
        context: &AppContext,
        status: TaskStatus,
    ) -> Result<bool, AppError> {
        let Some(task_id) = self.selected_task_id() else {
            self.status_message = Some("Select a task first.".to_owned());
            return Ok(true);
        };

        let updated = context
            .task_service
            .set_task_status(UpdateTaskStatusCommand {
                task_ref: task_id.as_str().to_owned(),
                status,
            })?;
        self.select_task_after_action(updated.id.clone());
        self.status_message = Some(format!(
            "Set {} to {}.",
            updated.title,
            status_label(status)
        ));
        self.reload(context)?;
        Ok(true)
    }

    fn archive_selected_task(&mut self, context: &AppContext) -> Result<bool, AppError> {
        let Some(task_id) = self.selected_task_id() else {
            self.status_message = Some("Select a task first.".to_owned());
            return Ok(true);
        };

        let outcome = context.task_service.archive_task(
            crate::application::commands::ArchiveTaskCommand {
                task_ref: task_id.as_str().to_owned(),
            },
        )?;
        if let Some(task) = outcome.root_task {
            self.select_task_after_action(task.id);
            self.status_message = Some(format!("Archived {} task(s).", outcome.affected_count));
        }
        self.reload(context)?;
        Ok(true)
    }

    fn restore_selected_task(&mut self, context: &AppContext) -> Result<bool, AppError> {
        let Some(task_id) = self.selected_task_id() else {
            self.status_message = Some("Select a task first.".to_owned());
            return Ok(true);
        };

        let outcome = context.task_service.restore_task(RestoreTaskCommand {
            task_ref: task_id.as_str().to_owned(),
            status: TaskStatus::Todo,
        })?;
        if let Some(task) = outcome.root_task {
            self.select_task_after_action(task.id);
            self.status_message = Some(format!("Restored {} task(s).", outcome.affected_count));
        }
        self.reload(context)?;
        Ok(true)
    }

    fn reload(&mut self, context: &AppContext) -> Result<(), AppError> {
        let previous_selected_task_id = self.selected_task_id();
        let previous_selected_index = self.task_list_state.selected();

        self.spaces = context.space_service.list_spaces(ListSpacesQuery {
            include_archived: false,
        })?;

        if self.current_space_id.is_none() {
            self.current_space_id = self.spaces.first().map(|space| space.space.id.clone());
        }

        if let Some(space_id) = self.current_space_id.as_ref() {
            if !self.spaces.iter().any(|space| &space.space.id == space_id) {
                self.current_space_id = self.spaces.first().map(|space| space.space.id.clone());
            }
        }

        self.space_index = self
            .current_space_id
            .as_ref()
            .and_then(|space_id| {
                self.spaces
                    .iter()
                    .position(|space| &space.space.id == space_id)
            })
            .unwrap_or(0);

        if let Some(space_id) = self.current_space_id.clone() {
            let result = context.task_service.list_tasks(ListTasksQuery {
                space_ref: Some(space_id.as_str().to_owned()),
                view: Some(self.current_view),
                sort: Some(self.current_sort),
            })?;
            self.task_result = Some(result);
            self.details_scroll = self
                .space_memory()
                .map_or(0, |memory| memory.details_scroll);
            self.rebuild_visible_tasks(previous_selected_task_id, previous_selected_index);
            self.refresh_details(context)?;
        } else {
            self.task_result = None;
            self.visible_tasks.clear();
            self.task_list_state = ListState::default();
            self.details = None;
            self.details_scroll = 0;
        }

        Ok(())
    }

    fn refresh_details(&mut self, context: &AppContext) -> Result<(), AppError> {
        self.details = match self.selected_task_id() {
            Some(task_id) => Some(context.task_service.show_task(
                crate::application::queries::ShowTaskQuery {
                    task_ref: task_id.as_str().to_owned(),
                },
            )?),
            None => None,
        };
        Ok(())
    }

    fn rebuild_visible_tasks(
        &mut self,
        preferred_task_id: Option<TaskId>,
        previous_selected_index: Option<usize>,
    ) {
        let Some(task_result) = self.task_result.as_ref() else {
            self.visible_tasks.clear();
            self.task_list_state = ListState::default();
            return;
        };

        let expanded_ids = self
            .space_memory()
            .map(|memory| {
                memory
                    .expanded_task_ids
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        self.visible_tasks = build_visible_tasks(&task_result.entries, &expanded_ids);

        let memory_selected = self
            .space_memory()
            .and_then(|memory| memory.selected_task_id.clone());
        let selected_index = preferred_task_id
            .as_ref()
            .and_then(|task_id| index_of_task(&self.visible_tasks, task_id))
            .or_else(|| {
                memory_selected
                    .as_ref()
                    .and_then(|task_id| index_of_task(&self.visible_tasks, task_id))
            })
            .or_else(|| previous_selected_index.filter(|index| *index < self.visible_tasks.len()))
            .or_else(|| (!self.visible_tasks.is_empty()).then_some(0));

        let offset = self
            .space_memory()
            .map_or(0, |memory| memory.task_tree_scroll);
        self.task_list_state = ListState::default().with_offset(offset);
        self.task_list_state.select(selected_index);
        self.ensure_selected_task_visible();
    }

    fn scroll_task_tree(&mut self, delta: isize) {
        let viewport_height = self
            .ui
            .task_tree_viewport
            .map(|rect| rect.height.saturating_sub(2) as usize)
            .unwrap_or(1)
            .max(1);
        let max_offset = self.visible_tasks.len().saturating_sub(viewport_height);
        let current = self.task_list_state.offset() as isize;
        let next = (current + delta).clamp(0, max_offset as isize) as usize;
        *self.task_list_state.offset_mut() = next;
    }

    fn scroll_details(&mut self, delta: isize) {
        if delta.is_negative() {
            self.details_scroll = self.details_scroll.saturating_sub(delta.unsigned_abs());
        } else {
            self.details_scroll = self.details_scroll.saturating_add(delta as usize);
        }
    }

    fn ensure_selected_task_visible(&mut self) {
        let Some(selected) = self.task_list_state.selected() else {
            return;
        };
        let viewport_height = self
            .ui
            .task_tree_viewport
            .map(|rect| rect.height.saturating_sub(2) as usize)
            .unwrap_or(1)
            .max(1);
        let offset = self.task_list_state.offset();
        if selected < offset {
            *self.task_list_state.offset_mut() = selected;
        } else if selected >= offset.saturating_add(viewport_height) {
            *self.task_list_state.offset_mut() = selected.saturating_sub(viewport_height - 1);
        }
    }

    fn select_task_after_action(&mut self, task_id: TaskId) {
        if let Some(memory) = self.space_memory_mut() {
            memory.selected_task_id = Some(task_id);
        }
    }

    fn expand_task(&mut self, task_id: TaskId) {
        if let Some(memory) = self.space_memory_mut() {
            if !memory.expanded_task_ids.contains(&task_id) {
                memory.expanded_task_ids.push(task_id);
            }
        }
    }

    fn collapse_task(&mut self, task_id: TaskId) {
        if let Some(memory) = self.space_memory_mut() {
            memory
                .expanded_task_ids
                .retain(|expanded| expanded != &task_id);
        }
    }

    fn subtree_count(&self, task_id: &TaskId) -> usize {
        self.task_result
            .as_ref()
            .map(|result| {
                let tasks = result
                    .entries
                    .iter()
                    .map(|entry| entry.task.clone())
                    .collect::<Vec<_>>();
                collect_subtree_ids(&tasks, task_id).len()
            })
            .unwrap_or(1)
    }

    fn sync_memory(&mut self) {
        self.tui_memory.focus_area = self.focus_area;
        self.tui_memory.spaces_cursor = self.space_index;
        if let Some(space_id) = self.current_space_id.clone() {
            let selected_task_id = self.selected_task_id();
            let task_tree_scroll = self.task_list_state.offset();
            let details_scroll = self.details_scroll;
            let memory = self
                .tui_memory
                .spaces
                .entry(space_id)
                .or_insert_with(SpaceViewMemory::default);
            memory.selected_task_id = selected_task_id;
            memory.task_tree_scroll = task_tree_scroll;
            memory.details_scroll = details_scroll;
        }
    }

    fn space_memory(&self) -> Option<&SpaceViewMemory> {
        self.current_space_id
            .as_ref()
            .and_then(|space_id| self.tui_memory.spaces.get(space_id))
    }

    fn space_memory_mut(&mut self) -> Option<&mut SpaceViewMemory> {
        let space_id = self.current_space_id.clone()?;
        Some(self.tui_memory.spaces.entry(space_id).or_default())
    }
}

fn description_option(input: &TextInput) -> Option<String> {
    let value = input.value();
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn build_visible_tasks(
    entries: &[crate::application::queries::TaskListEntry],
    expanded_ids: &HashSet<TaskId>,
) -> Vec<VisibleTaskEntry> {
    let mut visible = Vec::new();
    let mut hidden_depth: Option<usize> = None;

    for entry in entries {
        if let Some(depth) = hidden_depth {
            if entry.depth > depth {
                continue;
            }
            hidden_depth = None;
        }

        let is_expanded = entry.child_count == 0 || expanded_ids.contains(&entry.task.id);
        visible.push(VisibleTaskEntry {
            task: entry.task.clone(),
            depth: entry.depth,
            child_count: entry.child_count,
            is_expanded,
        });

        if entry.child_count > 0 && !is_expanded {
            hidden_depth = Some(entry.depth);
        }
    }

    visible
}

fn index_of_task(entries: &[VisibleTaskEntry], task_id: &TaskId) -> Option<usize> {
    entries.iter().position(|entry| &entry.task.id == task_id)
}

fn is_global_quit_shortcut(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn set_single_line_cursor(input: &mut TextInput, rect: Rect, position: Position) {
    let inner_x = rect.x.saturating_add(1);
    let col = position.x.saturating_sub(inner_x) as usize;
    input.set_cursor(0, col);
}

fn set_multiline_cursor(input: &mut TextInput, rect: Rect, position: Position) {
    let inner_x = rect.x.saturating_add(1);
    let inner_y = rect.y.saturating_add(1);
    let row = position.y.saturating_sub(inner_y) as usize;
    let col = position.x.saturating_sub(inner_x) as usize;
    input.set_cursor(row, col);
}

fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Archived => "archived",
    }
}

pub(crate) fn collect_subtree_ids(tasks: &[Task], root_id: &TaskId) -> HashSet<TaskId> {
    let mut stack = vec![root_id.clone()];
    let mut seen = HashSet::new();

    while let Some(current_id) = stack.pop() {
        if !seen.insert(current_id.clone()) {
            continue;
        }

        for child in tasks
            .iter()
            .filter(|task| task.parent_id.as_ref() == Some(&current_id))
        {
            stack.push(child.id.clone());
        }
    }

    seen
}

#[cfg(test)]
mod tests {
    use super::{build_visible_tasks, is_global_quit_shortcut};
    use crate::application::queries::TaskListEntry;
    use crate::domain::{SpaceId, Task};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::HashSet;

    #[test]
    fn collapsed_parent_hides_descendants() {
        let space_id = SpaceId::new();
        let parent = Task::new("parent", space_id.clone(), 0);
        let mut child = Task::new("child", space_id, 1);
        child.parent_id = Some(parent.id.clone());
        let entries = vec![
            TaskListEntry {
                task: parent.clone(),
                depth: 0,
                child_count: 1,
            },
            TaskListEntry {
                task: child,
                depth: 1,
                child_count: 0,
            },
        ];

        let visible = build_visible_tasks(&entries, &HashSet::new());
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].task.id, parent.id);
    }

    #[test]
    fn ctrl_c_is_reserved_as_global_quit() {
        assert!(is_global_quit_shortcut(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(is_global_quit_shortcut(KeyEvent::new(
            KeyCode::Char('C'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        )));
    }
}
