use crate::application::bootstrap::AppContext;
use crate::application::commands::{
    AddTaskLogCommand, ArchiveSpaceCommand, CreateSpaceCommand, CreateTaskCommand, EditTaskCommand,
    MoveTaskCommand, MoveTaskDirection, PurgeSpaceCommand, PurgeTaskCommand, RenameSpaceCommand,
    RestoreSpaceCommand, RestoreTaskCommand, SetCurrentSpaceCommand, UpdateTaskStatusCommand,
};
use crate::application::error::AppError;
use crate::application::queries::{
    ListSpacesQuery, ListTasksQuery, SpaceSummary, TaskDetails, TaskListResult,
};
use crate::domain::{
    FocusArea, SortMode, SpaceId, SpaceListMode, SpaceState, SpaceViewMemory, Task, TaskId,
    TaskStatus, TuiMemory, ViewMode,
};
use crate::tui::LaunchOptions;
use crate::tui::input::TextInput;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;
use std::collections::{HashMap, HashSet};

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
    PurgeSpace(PurgeSpaceConfirmState),
}

#[derive(Debug, Clone)]
pub enum Mode {
    Browse,
    SpaceManager(SpaceManagerState),
    Form(FormModal),
    Confirm(ConfirmModal),
    Filter(FilterState),
    Help,
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

#[derive(Debug, Clone)]
pub struct PurgeSpaceConfirmState {
    pub space_id: SpaceId,
    pub space_name: String,
    pub task_count: usize,
    pub phrase: TextInput,
}

#[derive(Debug, Clone)]
pub struct FilterState {
    pub input: TextInput,
}

#[derive(Debug, Clone)]
pub struct SpaceManagerState {
    pub scroll: usize,
    pub origin_focus: FocusArea,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseTarget {
    SwitchView(ViewMode),
    CycleSort,
    OpenFilter,
    OpenSpaceManager,
    OpenHelp,
    CloseHelp,
    CloseSpaceManager,
    SelectManagedSpace(usize),
    OpenSelectedSpace,
    SetSpaceListMode(SpaceListMode),
    OpenSpaceCreate,
    OpenSpaceRename,
    ArchiveSpace,
    RestoreSpace,
    OpenPurgeSpace,
    SelectTask(usize),
    ToggleTask(TaskId),
    CreateTask,
    CreateSubtask,
    CloseDetails,
    EditTask,
    SetTaskStatus(TaskStatus),
    AddLog,
    RestoreTask,
    OpenPurgeTask,
    MoveTask(MoveTaskDirection),
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
    FilterInput,
    FilterApply,
    FilterClear,
    FilterCancel,
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
    pub space_manager_viewport: Option<Rect>,
    pub task_tree_viewport: Option<Rect>,
    pub details_viewport: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct TuiApp {
    pub should_quit: bool,
    pub mode: Mode,
    pub background_mode: Option<Mode>,
    pub focus_area: FocusArea,
    pub return_focus: FocusArea,
    pub current_space_id: Option<SpaceId>,
    pub viewed_space_id: Option<SpaceId>,
    pub current_view: ViewMode,
    pub current_sort: SortMode,
    pub space_list_mode: SpaceListMode,
    pub task_filter: String,
    pub spaces: Vec<SpaceSummary>,
    pub space_index: usize,
    pub task_result: Option<TaskListResult>,
    pub visible_tasks: Vec<VisibleTaskEntry>,
    pub task_list_state: ListState,
    pub details: Option<TaskDetails>,
    pub details_scroll: usize,
    pub tui_memory: TuiMemory,
    pub status_message: Option<String>,
    pub hovered_target: Option<MouseTarget>,
    pub ui: UiState,
}

impl TuiApp {
    pub fn new(context: &AppContext, options: LaunchOptions) -> Result<Self, AppError> {
        let mut state = context.app_state_service.load()?;

        if let Some(space_id) = options.space_id {
            state.tui_memory.selected_space_id = Some(space_id);
        }
        if let Some(view) = options.view {
            state.current_view = view;
        }
        if let Some(sort) = options.sort {
            state.current_sort = sort;
        }

        let viewed_space_id = state
            .tui_memory
            .selected_space_id
            .clone()
            .or_else(|| state.current_space_id.clone());

        let mut app = Self {
            should_quit: false,
            mode: Mode::Browse,
            background_mode: None,
            focus_area: state.tui_memory.focus_area,
            return_focus: state.tui_memory.focus_area,
            current_space_id: state.current_space_id.clone(),
            viewed_space_id,
            current_view: state.current_view,
            current_sort: state.current_sort,
            space_list_mode: state.tui_memory.space_list_mode,
            task_filter: state.tui_memory.task_filter.clone(),
            spaces: Vec::new(),
            space_index: state.tui_memory.spaces_cursor,
            task_result: None,
            visible_tasks: Vec::new(),
            task_list_state: ListState::default(),
            details: None,
            details_scroll: 0,
            tui_memory: state.tui_memory,
            status_message: None,
            hovered_target: None,
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
            Mode::Browse => self.handle_browse_key(context, key),
            Mode::SpaceManager(manager) => self.handle_space_manager_key(context, key, manager),
            Mode::Form(form) => self.handle_form_key(context, key, form),
            Mode::Confirm(confirm) => self.handle_confirm_key(context, key, confirm),
            Mode::Filter(filter) => self.handle_filter_key(key, filter),
            Mode::Help => self.handle_help_key(key),
        };

        Ok(self.handle_interaction_result(result))
    }

    pub fn handle_mouse(
        &mut self,
        context: &AppContext,
        mouse: MouseEvent,
    ) -> Result<bool, AppError> {
        let position = Position::new(mouse.column, mouse.row);
        let result = match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_click(context, position),
            MouseEventKind::ScrollDown => self.handle_scroll(context, position, 3),
            MouseEventKind::ScrollUp => self.handle_scroll(context, position, -3),
            MouseEventKind::Moved => Ok(self.update_hover(position)),
            _ => Ok(false),
        };

        Ok(self.handle_interaction_result(result))
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
        self.viewed_space_id
            .as_ref()
            .and_then(|space_id| self.spaces.iter().find(|space| &space.space.id == space_id))
    }

    pub fn current_active_space(&self) -> Option<&SpaceSummary> {
        self.current_space_id
            .as_ref()
            .and_then(|space_id| self.spaces.iter().find(|space| &space.space.id == space_id))
    }

    pub fn selected_space_summary(&self) -> Option<&SpaceSummary> {
        self.spaces.get(self.space_index)
    }

    pub fn space_button_label(&self) -> String {
        self.current_space()
            .map(|space| format!("Space: {}", space.space.name))
            .unwrap_or_else(|| "Space: none".to_owned())
    }

    pub fn task_tree_empty_message(&self) -> &'static str {
        if self.spaces.is_empty() {
            "No spaces yet. Click the Space button to open the manager and create your first space."
        } else if self.task_filter.trim().is_empty() {
            "No tasks in this view. Click + Task to create one."
        } else {
            "No tasks match the current filter. Click Filter to adjust it."
        }
    }

    pub fn help_text(&self) -> String {
        match &self.mode {
            Mode::Help => "[Scroll] [Close] [Ctrl+C Quit]".to_owned(),
            Mode::SpaceManager(_) => {
                "[Select Space] [Open] [Manage] [Esc Close] [Ctrl+C Quit]".to_owned()
            }
            Mode::Filter(_) => "[Type Filter] [Apply] [Clear] [Cancel] [Ctrl+C Quit]".to_owned(),
            Mode::Form(FormModal::Space(_)) => {
                "[Type Name] [Save] [Cancel] [Ctrl+C Quit]".to_owned()
            }
            Mode::Form(FormModal::Task(_)) => {
                "[Type] [Pick Status] [Save] [Cancel] [Ctrl+C Quit]".to_owned()
            }
            Mode::Form(FormModal::Log(_)) => "[Type Log] [Save] [Cancel] [Ctrl+C Quit]".to_owned(),
            Mode::Confirm(_) => "[Type purge] [Confirm] [Cancel] [Ctrl+C Quit]".to_owned(),
            Mode::Browse => match self.focus_area {
                FocusArea::Spaces => "[Spaces] [Select] [Open] [Action] [Ctrl+C Quit]".to_owned(),
                FocusArea::TaskTree => {
                    "[Spaces] [Scroll] [Select] [Toggle] [Ctrl+C Quit]".to_owned()
                }
                FocusArea::Details => {
                    "[Spaces] [Scroll] [Select] [Action] [Ctrl+C Quit]".to_owned()
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
        if key.code == KeyCode::Esc {
            let _ = context;
            self.close_modal(false);
            return Ok(true);
        }

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

    fn handle_browse_key(
        &mut self,
        _context: &AppContext,
        key: KeyEvent,
    ) -> Result<bool, AppError> {
        match key.code {
            KeyCode::Char('?') => {
                self.open_help();
                Ok(true)
            }
            KeyCode::Char('/') => {
                self.open_filter();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle_space_manager_key(
        &mut self,
        _context: &AppContext,
        key: KeyEvent,
        manager: SpaceManagerState,
    ) -> Result<bool, AppError> {
        match key.code {
            KeyCode::Esc => {
                self.close_modal(false);
                Ok(true)
            }
            _ => {
                self.mode = Mode::SpaceManager(manager);
                Ok(false)
            }
        }
    }

    fn handle_confirm_key(
        &mut self,
        context: &AppContext,
        key: KeyEvent,
        confirm: ConfirmModal,
    ) -> Result<bool, AppError> {
        if key.code == KeyCode::Esc {
            let _ = context;
            self.close_modal(false);
            return Ok(true);
        }

        match confirm {
            ConfirmModal::PurgeTask(mut confirm) => {
                let _ = context;
                if confirm.requires_phrase {
                    confirm.phrase.handle_key(key);
                }
                self.mode = Mode::Confirm(ConfirmModal::PurgeTask(confirm));
                Ok(true)
            }
            ConfirmModal::PurgeSpace(mut confirm) => {
                let _ = context;
                confirm.phrase.handle_key(key);
                self.mode = Mode::Confirm(ConfirmModal::PurgeSpace(confirm));
                Ok(true)
            }
        }
    }

    fn handle_filter_key(
        &mut self,
        key: KeyEvent,
        mut filter: FilterState,
    ) -> Result<bool, AppError> {
        match key.code {
            KeyCode::Esc => {
                self.close_modal(false);
                Ok(true)
            }
            _ => {
                filter.input.handle_key(key);
                self.mode = Mode::Filter(filter);
                Ok(true)
            }
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                self.close_modal(false);
                Ok(true)
            }
            _ => Ok(false),
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

    pub fn set_space_manager_viewport(&mut self, rect: Rect) {
        self.ui.space_manager_viewport = Some(rect);
    }

    pub fn set_task_tree_viewport(&mut self, rect: Rect) {
        self.ui.task_tree_viewport = Some(rect);
    }

    pub fn set_details_viewport(&mut self, rect: Rect) {
        self.ui.details_viewport = Some(rect);
    }

    pub fn is_hovered(&self, target: &MouseTarget) -> bool {
        self.hovered_target.as_ref() == Some(target)
    }

    pub fn filter_label(&self) -> String {
        if self.task_filter.trim().is_empty() {
            "Filter: all".to_owned()
        } else {
            format!("Filter: {}", self.task_filter.trim())
        }
    }

    fn handle_interaction_result(&mut self, result: Result<bool, AppError>) -> bool {
        match result {
            Ok(changed) => changed,
            Err(error) => {
                self.status_message = Some(match error.hint() {
                    Some(hint) => format!("{error} | {hint}"),
                    None => error.to_string(),
                });
                true
            }
        }
    }

    fn handle_click(&mut self, context: &AppContext, position: Position) -> Result<bool, AppError> {
        let Some(hitbox) = self.hitbox_at(position) else {
            return Ok(false);
        };

        self.hovered_target = Some(hitbox.target.clone());
        self.apply_mouse_target(context, hitbox, position)
    }

    fn update_hover(&mut self, position: Position) -> bool {
        let next = self.hitbox_at(position).map(|hitbox| hitbox.target);
        if self.hovered_target == next {
            false
        } else {
            self.hovered_target = next;
            true
        }
    }

    fn handle_scroll(
        &mut self,
        context: &AppContext,
        position: Position,
        delta: isize,
    ) -> Result<bool, AppError> {
        if matches!(self.mode, Mode::SpaceManager(_)) {
            if let Some(viewport) = self.ui.space_manager_viewport {
                if viewport.contains(position) {
                    self.focus_area = FocusArea::Spaces;
                    self.scroll_space_manager(delta);
                    return Ok(true);
                }
            }

            return Ok(false);
        }

        if !matches!(self.mode, Mode::Browse) {
            return Ok(false);
        }

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
            MouseTarget::OpenFilter => {
                self.focus_area = FocusArea::TaskTree;
                self.open_filter();
                Ok(true)
            }
            MouseTarget::OpenSpaceManager => {
                self.open_space_manager();
                Ok(true)
            }
            MouseTarget::OpenHelp => {
                self.open_help();
                Ok(true)
            }
            MouseTarget::CloseHelp => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::CloseSpaceManager => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::SelectManagedSpace(index) => {
                self.focus_area = FocusArea::Spaces;
                self.select_managed_space(index)
            }
            MouseTarget::OpenSelectedSpace => {
                self.focus_area = FocusArea::Spaces;
                self.open_selected_space(context)
            }
            MouseTarget::SetSpaceListMode(mode) => {
                self.focus_area = FocusArea::Spaces;
                self.set_space_list_mode(context, mode)
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
            MouseTarget::ArchiveSpace => {
                self.focus_area = FocusArea::Spaces;
                self.archive_selected_space(context)
            }
            MouseTarget::RestoreSpace => {
                self.focus_area = FocusArea::Spaces;
                self.restore_selected_space(context)
            }
            MouseTarget::OpenPurgeSpace => {
                self.focus_area = FocusArea::Spaces;
                self.open_purge_space_confirm();
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
            MouseTarget::RestoreTask => {
                self.focus_area = FocusArea::Details;
                self.restore_selected_task(context)
            }
            MouseTarget::OpenPurgeTask => {
                self.focus_area = FocusArea::Details;
                self.open_purge_confirm(context)?;
                Ok(true)
            }
            MouseTarget::MoveTask(direction) => {
                self.focus_area = FocusArea::Details;
                self.move_selected_task(context, direction)
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
            MouseTarget::FilterInput => {
                if let Mode::Filter(mut filter) = self.mode.clone() {
                    set_single_line_cursor(&mut filter.input, hitbox.rect, position);
                    self.mode = Mode::Filter(filter);
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::FilterApply => {
                if let Mode::Filter(filter) = self.mode.clone() {
                    self.submit_filter(context, filter)?;
                    return Ok(true);
                }
                Ok(false)
            }
            MouseTarget::FilterClear => {
                self.clear_filter(context)?;
                Ok(true)
            }
            MouseTarget::FilterCancel => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::ConfirmPhraseInput => {
                match self.mode.clone() {
                    Mode::Confirm(ConfirmModal::PurgeTask(mut confirm)) => {
                        if confirm.requires_phrase {
                            set_single_line_cursor(&mut confirm.phrase, hitbox.rect, position);
                            self.mode = Mode::Confirm(ConfirmModal::PurgeTask(confirm));
                            return Ok(true);
                        }
                    }
                    Mode::Confirm(ConfirmModal::PurgeSpace(mut confirm)) => {
                        set_single_line_cursor(&mut confirm.phrase, hitbox.rect, position);
                        self.mode = Mode::Confirm(ConfirmModal::PurgeSpace(confirm));
                        return Ok(true);
                    }
                    _ => {}
                }
                Ok(false)
            }
            MouseTarget::ConfirmCancel => {
                self.close_modal(false);
                Ok(true)
            }
            MouseTarget::ConfirmPurge => {
                match self.mode.clone() {
                    Mode::Confirm(ConfirmModal::PurgeTask(confirm)) => {
                        self.submit_purge_confirm(context, confirm)?;
                        return Ok(true);
                    }
                    Mode::Confirm(ConfirmModal::PurgeSpace(confirm)) => {
                        self.submit_purge_space_confirm(context, confirm)?;
                        return Ok(true);
                    }
                    _ => {}
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

    fn base_focus_area(&self) -> FocusArea {
        match &self.mode {
            Mode::SpaceManager(state) => state.origin_focus,
            _ => match &self.background_mode {
                Some(Mode::SpaceManager(state)) => state.origin_focus,
                _ => self.return_focus,
            },
        }
    }

    fn enter_root_overlay(&mut self, mode: Mode) {
        self.return_focus = if matches!(self.mode, Mode::Browse) {
            self.focus_area
        } else {
            self.base_focus_area()
        };
        self.background_mode = Some(Mode::Browse);
        self.mode = mode;
    }

    fn enter_child_overlay(&mut self, mode: Mode) {
        self.return_focus = self.focus_area;
        self.background_mode = Some(self.mode.clone());
        self.mode = mode;
    }

    fn open_space_manager(&mut self) {
        let origin_focus = if matches!(self.mode, Mode::Browse) {
            self.focus_area
        } else {
            self.base_focus_area()
        };
        self.enter_root_overlay(Mode::SpaceManager(SpaceManagerState {
            scroll: 0,
            origin_focus,
        }));
        self.focus_area = FocusArea::Spaces;
    }

    pub fn space_manager_scroll(&self) -> usize {
        match &self.mode {
            Mode::SpaceManager(state) => state.scroll,
            _ => 0,
        }
    }

    pub fn set_space_manager_scroll(&mut self, scroll: usize) {
        if let Mode::SpaceManager(state) = &mut self.mode {
            state.scroll = scroll;
        }
    }

    fn select_managed_space(&mut self, index: usize) -> Result<bool, AppError> {
        if index >= self.spaces.len() {
            return Ok(false);
        }

        self.space_index = index;
        self.ensure_space_manager_selection_visible();
        Ok(true)
    }

    fn open_selected_space(&mut self, context: &AppContext) -> Result<bool, AppError> {
        let Some(space) = self.selected_space_summary().cloned() else {
            self.status_message = Some("Select a space first.".to_owned());
            return Ok(true);
        };

        if space.space.state.is_active() {
            context.space_service.use_space(SetCurrentSpaceCommand {
                space_ref: space.space.id.as_str().to_owned(),
            })?;
            self.current_space_id = Some(space.space.id.clone());
            self.status_message = Some(format!("Switched to space {}.", space.space.name));
        } else {
            self.status_message = Some(format!("Viewing archived space {}.", space.space.name));
        }

        self.viewed_space_id = Some(space.space.id.clone());
        self.close_modal(false);
        self.reload(context)?;
        Ok(true)
    }

    fn open_space_form_create(&mut self) {
        self.enter_child_overlay(Mode::Form(FormModal::Space(SpaceFormState {
            mode: SpaceFormMode::Create,
            name: TextInput::single_line(""),
        })));
    }

    fn open_filter(&mut self) {
        self.enter_root_overlay(Mode::Filter(FilterState {
            input: TextInput::single_line(&self.task_filter),
        }));
    }

    fn open_help(&mut self) {
        self.enter_root_overlay(Mode::Help);
    }

    fn open_space_form_rename(&mut self) {
        if let Some(space) = self
            .selected_space_summary()
            .map(|space| (space.space.id.clone(), space.space.name.clone()))
        {
            self.enter_child_overlay(Mode::Form(FormModal::Space(SpaceFormState {
                mode: SpaceFormMode::Rename { space_id: space.0 },
                name: TextInput::single_line(space.1),
            })));
        } else {
            self.status_message = Some("No space selected to rename.".to_owned());
        }
    }

    fn open_purge_space_confirm(&mut self) {
        if let Some(space) = self.selected_space_summary().cloned() {
            if !space.space.state.is_archived() {
                self.status_message =
                    Some("Only archived spaces can be purged. Archive the space first.".to_owned());
                return;
            }

            self.enter_child_overlay(Mode::Confirm(ConfirmModal::PurgeSpace(
                PurgeSpaceConfirmState {
                    space_id: space.space.id.clone(),
                    space_name: space.space.name.clone(),
                    task_count: space.counts.todo_tasks + space.counts.archived_tasks,
                    phrase: TextInput::single_line(""),
                },
            )));
        } else {
            self.status_message = Some("Select a space first to purge it.".to_owned());
        }
    }

    fn open_task_form_create_root(&mut self) {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before creating new tasks inside it.".to_owned());
            return;
        }
        self.enter_root_overlay(Mode::Form(FormModal::Task(TaskFormState {
            mode: TaskFormMode::CreateRoot,
            focus: TaskFormField::Title,
            title: TextInput::single_line(""),
            description: TextInput::multiline(""),
            status: TaskStatus::Todo,
        })));
    }

    fn open_task_form_create_child(&mut self) {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before creating new tasks inside it.".to_owned());
            return;
        }
        if let Some(task_id) = self.selected_task_id() {
            if let Some(memory) = self.space_memory_mut() {
                if !memory.expanded_task_ids.contains(&task_id) {
                    memory.expanded_task_ids.push(task_id.clone());
                }
            }
            self.enter_root_overlay(Mode::Form(FormModal::Task(TaskFormState {
                mode: TaskFormMode::CreateChild { parent_id: task_id },
                focus: TaskFormField::Title,
                title: TextInput::single_line(""),
                description: TextInput::multiline(""),
                status: TaskStatus::Todo,
            })));
        } else {
            self.status_message = Some("Select a task first to create a subtask.".to_owned());
        }
    }

    fn open_task_form_edit(&mut self) {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before editing tasks inside it.".to_owned());
            return;
        }
        if let Some(details) = self.details.as_ref() {
            self.enter_root_overlay(Mode::Form(FormModal::Task(TaskFormState {
                mode: TaskFormMode::Edit {
                    task_id: details.task.id.clone(),
                },
                focus: TaskFormField::Title,
                title: TextInput::single_line(&details.task.title),
                description: TextInput::multiline(
                    details.task.description.clone().unwrap_or_default(),
                ),
                status: details.task.status,
            })));
        } else {
            self.status_message = Some("Select a task first to edit it.".to_owned());
        }
    }

    fn open_log_form(&mut self) {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before adding logs inside it.".to_owned());
            return;
        }
        if let Some(details) = self.details.as_ref() {
            self.enter_root_overlay(Mode::Form(FormModal::Log(LogFormState {
                task_id: details.task.id.clone(),
                task_title: details.task.title.clone(),
                input: TextInput::multiline(""),
            })));
        } else {
            self.status_message = Some("Select a task first to add a log.".to_owned());
        }
    }

    fn open_purge_confirm(&mut self, _context: &AppContext) -> Result<(), AppError> {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before purging tasks inside it.".to_owned());
            return Ok(());
        }

        if let Some((task_id, task_title, archived)) = self.selected_task().map(|selected| {
            (
                selected.task.id.clone(),
                selected.task.title.clone(),
                selected.task.archived,
            )
        }) {
            if !archived {
                self.status_message =
                    Some("Only archived tasks can be purged. Archive the task first.".to_owned());
                return Ok(());
            }
            let affected_count = self.subtree_count(&task_id);
            self.enter_root_overlay(Mode::Confirm(ConfirmModal::PurgeTask(
                PurgeTaskConfirmState {
                    task_id,
                    task_title,
                    affected_count,
                    requires_phrase: affected_count > 1,
                    phrase: TextInput::single_line(""),
                },
            )));
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
                self.viewed_space_id = Some(created.id.clone());
                self.space_index = usize::MAX;
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
                    space_ref: self
                        .current_space()
                        .map(|space| space.space.id.as_str().to_owned()),
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

    fn submit_purge_space_confirm(
        &mut self,
        context: &AppContext,
        confirm: PurgeSpaceConfirmState,
    ) -> Result<(), AppError> {
        if confirm.phrase.value().trim() != "purge" {
            self.status_message = Some("Type `purge` to confirm this deletion.".to_owned());
            self.mode = Mode::Confirm(ConfirmModal::PurgeSpace(confirm));
            return Ok(());
        }

        context.space_service.purge_space(PurgeSpaceCommand {
            space_ref: confirm.space_id.as_str().to_owned(),
        })?;
        self.status_message = Some(format!("Purged space {}.", confirm.space_name));
        self.close_modal(true);
        self.reload(context)?;
        Ok(())
    }

    fn submit_filter(&mut self, context: &AppContext, filter: FilterState) -> Result<(), AppError> {
        self.task_filter = filter.input.value().trim().to_owned();
        self.status_message = if self.task_filter.is_empty() {
            Some("Cleared task filter.".to_owned())
        } else {
            Some(format!("Applied filter: {}.", self.task_filter))
        };
        self.close_modal(false);
        self.reload(context)?;
        Ok(())
    }

    fn clear_filter(&mut self, context: &AppContext) -> Result<(), AppError> {
        self.task_filter.clear();
        self.status_message = Some("Cleared task filter.".to_owned());
        self.close_modal(false);
        self.reload(context)?;
        Ok(())
    }

    fn close_modal(&mut self, clear_message: bool) {
        let next_mode = self.background_mode.take().unwrap_or(Mode::Browse);
        self.mode = next_mode.clone();
        self.focus_area = self.return_focus;
        if let Mode::SpaceManager(state) = next_mode {
            self.focus_area = FocusArea::Spaces;
            self.return_focus = state.origin_focus;
        }
        self.hovered_target = None;
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

    fn set_space_list_mode(
        &mut self,
        context: &AppContext,
        mode: SpaceListMode,
    ) -> Result<bool, AppError> {
        if self.space_list_mode == mode {
            return Ok(false);
        }

        self.space_list_mode = mode;
        self.reload(context)?;
        Ok(true)
    }

    fn set_selected_task_status(
        &mut self,
        context: &AppContext,
        status: TaskStatus,
    ) -> Result<bool, AppError> {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before changing task status inside it.".to_owned());
            return Ok(true);
        }

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
        self.status_message = Some(if updated.archived && status.is_finished() {
            format!("Archived {} as {}.", updated.title, status_label(status))
        } else {
            format!("Set {} to {}.", updated.title, status_label(status))
        });
        self.reload(context)?;
        Ok(true)
    }

    fn restore_selected_task(&mut self, context: &AppContext) -> Result<bool, AppError> {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before restoring tasks inside it.".to_owned());
            return Ok(true);
        }

        let Some(task_id) = self.selected_task_id() else {
            self.status_message = Some("Select a task first.".to_owned());
            return Ok(true);
        };

        let outcome = context.task_service.restore_task(RestoreTaskCommand {
            task_ref: task_id.as_str().to_owned(),
        })?;
        if let Some(task) = outcome.root_task {
            self.select_task_after_action(task.id);
            self.status_message = Some(format!("Restored {} task(s).", outcome.affected_count));
        }
        self.reload(context)?;
        Ok(true)
    }

    fn archive_selected_space(&mut self, context: &AppContext) -> Result<bool, AppError> {
        let Some(space) = self.selected_space_summary().cloned() else {
            self.status_message = Some("Select a space first.".to_owned());
            return Ok(true);
        };

        if space.space.state.is_archived() {
            self.status_message = Some("That space is already archived.".to_owned());
            return Ok(true);
        }

        context.space_service.archive_space(ArchiveSpaceCommand {
            space_ref: space.space.id.as_str().to_owned(),
        })?;
        self.status_message = Some(format!("Archived space {}.", space.space.name));
        self.reload(context)?;
        Ok(true)
    }

    fn restore_selected_space(&mut self, context: &AppContext) -> Result<bool, AppError> {
        let Some(space) = self.selected_space_summary().cloned() else {
            self.status_message = Some("Select a space first.".to_owned());
            return Ok(true);
        };

        if space.space.state.is_active() {
            self.status_message = Some("That space is already active.".to_owned());
            return Ok(true);
        }

        context.space_service.restore_space(RestoreSpaceCommand {
            space_ref: space.space.id.as_str().to_owned(),
        })?;
        self.status_message = Some(format!("Restored space {}.", space.space.name));
        self.reload(context)?;
        Ok(true)
    }

    fn move_selected_task(
        &mut self,
        context: &AppContext,
        direction: MoveTaskDirection,
    ) -> Result<bool, AppError> {
        if !self.can_mutate_viewed_space() {
            self.status_message =
                Some("Restore this space before reordering tasks inside it.".to_owned());
            return Ok(true);
        }
        if self.current_sort != SortMode::Manual {
            self.status_message = Some("Switch to manual sort before reordering tasks.".to_owned());
            return Ok(true);
        }

        let Some(task_id) = self.selected_task_id() else {
            self.status_message = Some("Select a task first.".to_owned());
            return Ok(true);
        };

        let moved = context.task_service.move_task(MoveTaskCommand {
            task_ref: task_id.as_str().to_owned(),
            direction,
        })?;
        self.select_task_after_action(moved.id.clone());
        self.status_message = Some(format!(
            "Moved {} {}.",
            moved.title,
            match direction {
                MoveTaskDirection::Up => "up",
                MoveTaskDirection::Down => "down",
            }
        ));
        self.reload(context)?;
        Ok(true)
    }

    fn reload(&mut self, context: &AppContext) -> Result<(), AppError> {
        let previous_selected_task_id = self.selected_task_id();
        let previous_selected_index = self.task_list_state.selected();
        let previous_selected_space_id = self
            .selected_space_summary()
            .map(|space| space.space.id.clone());
        self.current_space_id = context.app_state_service.load()?.current_space_id;

        self.spaces = context.space_service.list_spaces(ListSpacesQuery {
            include_archived: self.space_list_mode.includes_archived(),
        })?;

        if self.viewed_space_id.is_none() {
            self.viewed_space_id = self
                .current_space_id
                .clone()
                .or_else(|| self.spaces.first().map(|space| space.space.id.clone()));
        }

        if let Some(space_id) = self.viewed_space_id.as_ref() {
            if !self.spaces.iter().any(|space| &space.space.id == space_id) {
                self.viewed_space_id = self
                    .current_space_id
                    .clone()
                    .filter(|candidate| {
                        self.spaces.iter().any(|space| &space.space.id == candidate)
                    })
                    .or_else(|| self.spaces.first().map(|space| space.space.id.clone()));
            }
        }

        self.space_index = previous_selected_space_id
            .as_ref()
            .and_then(|space_id| {
                self.spaces
                    .iter()
                    .position(|space| &space.space.id == space_id)
            })
            .or_else(|| {
                self.viewed_space_id.as_ref().and_then(|space_id| {
                    self.spaces
                        .iter()
                        .position(|space| &space.space.id == space_id)
                })
            })
            .unwrap_or(0);

        if let Some(space_id) = self.viewed_space_id.clone() {
            let result = context.task_service.list_tasks(ListTasksQuery {
                space_ref: Some(space_id.as_str().to_owned()),
                view: Some(self.current_view),
                sort: Some(self.current_sort),
                allow_archived_space: true,
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
        let filtered_entries = filter_task_entries(&task_result.entries, &self.task_filter);
        let expanded_ids = if self.task_filter.trim().is_empty() {
            expanded_ids
        } else {
            filtered_entries
                .iter()
                .filter(|entry| entry.child_count > 0)
                .map(|entry| entry.task.id.clone())
                .collect()
        };

        self.visible_tasks = build_visible_tasks(&filtered_entries, &expanded_ids);

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

    fn scroll_space_manager(&mut self, delta: isize) {
        let viewport_height = self
            .ui
            .space_manager_viewport
            .map(|rect| rect.height as usize)
            .unwrap_or(1)
            .max(1);
        let max_offset = self.spaces.len().saturating_sub(viewport_height);
        let current = self.space_manager_scroll() as isize;
        let next = (current + delta).clamp(0, max_offset as isize) as usize;
        self.set_space_manager_scroll(next);
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

    pub fn ensure_space_manager_selection_visible(&mut self) {
        let viewport_height = self
            .ui
            .space_manager_viewport
            .map(|rect| rect.height as usize)
            .unwrap_or(1)
            .max(1);
        let offset = self.space_manager_scroll();
        if self.space_index < offset {
            self.set_space_manager_scroll(self.space_index);
        } else if self.space_index >= offset.saturating_add(viewport_height) {
            self.set_space_manager_scroll(self.space_index.saturating_sub(viewport_height - 1));
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
        self.tui_memory.selected_space_id = self.viewed_space_id.clone();
        self.tui_memory.space_list_mode = self.space_list_mode;
        self.tui_memory.task_filter = self.task_filter.clone();
        if let Some(space_id) = self.viewed_space_id.clone() {
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
        self.viewed_space_id
            .as_ref()
            .and_then(|space_id| self.tui_memory.spaces.get(space_id))
    }

    fn space_memory_mut(&mut self) -> Option<&mut SpaceViewMemory> {
        let space_id = self.viewed_space_id.clone()?;
        Some(self.tui_memory.spaces.entry(space_id).or_default())
    }

    pub fn can_mutate_viewed_space(&self) -> bool {
        self.current_space()
            .is_some_and(|space| matches!(space.space.state, SpaceState::Active))
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

fn filter_task_entries(
    entries: &[crate::application::queries::TaskListEntry],
    query: &str,
) -> Vec<crate::application::queries::TaskListEntry> {
    let normalized_query = query.trim().to_lowercase();
    if normalized_query.is_empty() {
        return entries.to_vec();
    }

    let tasks = entries
        .iter()
        .map(|entry| entry.task.clone())
        .collect::<Vec<_>>();
    let tasks_by_id = entries
        .iter()
        .map(|entry| (entry.task.id.clone(), entry.task.clone()))
        .collect::<HashMap<_, _>>();
    let mut included_ids = HashSet::new();

    for entry in entries {
        if !task_matches_filter(&entry.task, &normalized_query) {
            continue;
        }

        let mut ancestor_id = Some(entry.task.id.clone());
        while let Some(task_id) = ancestor_id {
            if !included_ids.insert(task_id.clone()) {
                break;
            }
            ancestor_id = tasks_by_id
                .get(&task_id)
                .and_then(|task| task.parent_id.clone());
        }

        included_ids.extend(collect_subtree_ids(&tasks, &entry.task.id));
    }

    entries
        .iter()
        .filter(|entry| included_ids.contains(&entry.task.id))
        .cloned()
        .collect()
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

fn task_matches_filter(task: &Task, normalized_query: &str) -> bool {
    task.title.to_lowercase().contains(normalized_query)
        || task.id.as_str().to_lowercase().contains(normalized_query)
        || task.id.short_id().to_lowercase().contains(normalized_query)
        || status_label(task.status).contains(normalized_query)
        || task
            .description
            .as_deref()
            .is_some_and(|description| description.to_lowercase().contains(normalized_query))
        || task
            .logs
            .iter()
            .any(|log| log.message.to_lowercase().contains(normalized_query))
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
        TaskStatus::Close => "close",
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
    use super::{
        ConfirmModal, FormModal, Mode, MouseTarget, PurgeTaskConfirmState, SpaceFormMode,
        SpaceFormState, TuiApp, build_visible_tasks, filter_task_entries, is_global_quit_shortcut,
    };
    use crate::application::bootstrap::{BootstrapOptions, bootstrap};
    use crate::application::commands::{
        CreateSpaceCommand, CreateTaskCommand, SetCurrentSpaceCommand,
    };
    use crate::application::queries::{ListTasksQuery, TaskListEntry};
    use crate::domain::{FocusArea, SpaceId, Task, TaskId, TaskStatus};
    use crate::tui::LaunchOptions;
    use crate::tui::input::TextInput;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ratatui::layout::Rect;
    use std::collections::HashSet;
    use tempfile::tempdir;

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

    #[test]
    fn filter_keeps_matching_subtree_and_ancestors() {
        let space_id = SpaceId::new();
        let parent = Task::new("Weekly Review", space_id.clone(), 0);
        let mut child = Task::new("Collect notes", space_id.clone(), 1);
        child.parent_id = Some(parent.id.clone());
        let mut grandchild = Task::new("Draft summary", space_id, 2);
        grandchild.parent_id = Some(child.id.clone());

        let entries = vec![
            TaskListEntry {
                task: parent.clone(),
                depth: 0,
                child_count: 1,
            },
            TaskListEntry {
                task: child.clone(),
                depth: 1,
                child_count: 1,
            },
            TaskListEntry {
                task: grandchild.clone(),
                depth: 2,
                child_count: 0,
            },
        ];

        let filtered = filter_task_entries(&entries, "summary");
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].task.id, parent.id);
        assert_eq!(filtered[1].task.id, child.id);
        assert_eq!(filtered[2].task.id, grandchild.id);
    }

    #[test]
    fn esc_closes_form_and_confirm_popups() {
        let temp_dir = tempdir().unwrap();
        let context = bootstrap(BootstrapOptions {
            data_root: Some(temp_dir.path().join("app_data")),
        })
        .unwrap();
        let mut app = TuiApp::new(&context, LaunchOptions::default()).unwrap();

        app.return_focus = FocusArea::Details;
        app.mode = Mode::Form(FormModal::Space(SpaceFormState {
            mode: SpaceFormMode::Create,
            name: TextInput::single_line("draft"),
        }));

        let changed = app
            .handle_key(&context, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(changed);
        assert!(matches!(app.mode, Mode::Browse));
        assert_eq!(app.focus_area, FocusArea::Details);

        app.return_focus = FocusArea::Spaces;
        app.mode = Mode::Confirm(ConfirmModal::PurgeTask(PurgeTaskConfirmState {
            task_id: TaskId::new(),
            task_title: "draft".to_owned(),
            affected_count: 1,
            requires_phrase: false,
            phrase: TextInput::single_line(""),
        }));

        let changed = app
            .handle_key(&context, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(changed);
        assert!(matches!(app.mode, Mode::Browse));
        assert_eq!(app.focus_area, FocusArea::Spaces);
    }

    #[test]
    fn space_manager_restores_previous_focus_after_nested_dialog() {
        let temp_dir = tempdir().unwrap();
        let context = bootstrap(BootstrapOptions {
            data_root: Some(temp_dir.path().join("app_data")),
        })
        .unwrap();
        let mut app = TuiApp::new(&context, LaunchOptions::default()).unwrap();
        app.focus_area = FocusArea::TaskTree;

        app.open_space_manager();
        assert!(matches!(app.mode, Mode::SpaceManager(_)));
        assert_eq!(app.focus_area, FocusArea::Spaces);

        app.open_space_form_create();
        assert!(matches!(app.mode, Mode::Form(FormModal::Space(_))));

        let changed = app
            .handle_key(&context, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(changed);
        assert!(matches!(app.mode, Mode::SpaceManager(_)));
        assert_eq!(app.focus_area, FocusArea::Spaces);

        let changed = app
            .handle_key(&context, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(changed);
        assert!(matches!(app.mode, Mode::Browse));
        assert_eq!(app.focus_area, FocusArea::TaskTree);
    }

    #[test]
    fn root_popup_replaces_existing_popup_instead_of_nesting() {
        let temp_dir = tempdir().unwrap();
        let context = bootstrap(BootstrapOptions {
            data_root: Some(temp_dir.path().join("app_data")),
        })
        .unwrap();
        let mut app = TuiApp::new(&context, LaunchOptions::default()).unwrap();
        app.focus_area = FocusArea::TaskTree;

        app.open_space_manager();
        assert!(matches!(app.mode, Mode::SpaceManager(_)));

        app.open_help();
        assert!(matches!(app.mode, Mode::Help));

        let changed = app
            .handle_key(&context, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(changed);
        assert!(matches!(app.mode, Mode::Browse));
        assert_eq!(app.focus_area, FocusArea::TaskTree);
    }

    #[test]
    fn mouse_validation_errors_become_status_message_instead_of_exiting() {
        let temp_dir = tempdir().unwrap();
        let context = bootstrap(BootstrapOptions {
            data_root: Some(temp_dir.path().join("app_data")),
        })
        .unwrap();

        let space = context
            .space_service
            .create_space(CreateSpaceCommand {
                name: "Personal".to_owned(),
            })
            .unwrap();
        context
            .space_service
            .use_space(SetCurrentSpaceCommand {
                space_ref: space.id.as_str().to_owned(),
            })
            .unwrap();

        let parent = context
            .task_service
            .create_task(CreateTaskCommand {
                title: "Parent".to_owned(),
                space_ref: None,
                description: None,
                parent_ref: None,
                status: TaskStatus::Todo,
            })
            .unwrap();
        context
            .task_service
            .create_task(CreateTaskCommand {
                title: "Child".to_owned(),
                space_ref: None,
                description: None,
                parent_ref: Some(parent.id.as_str().to_owned()),
                status: TaskStatus::Todo,
            })
            .unwrap();

        let mut app = TuiApp::new(&context, LaunchOptions::default()).unwrap();
        let selected_index = app
            .visible_tasks
            .iter()
            .position(|entry| entry.task.id == parent.id)
            .expect("parent should be visible");
        app.task_list_state.select(Some(selected_index));
        app.refresh_details(&context).unwrap();
        app.register_hitbox(
            Rect::new(0, 0, 8, 1),
            MouseTarget::SetTaskStatus(TaskStatus::Done),
        );

        let changed = app
            .handle_mouse(
                &context,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                },
            )
            .unwrap();

        let reloaded = context.task_service.load_task(&parent.id).unwrap();
        let listed = context
            .task_service
            .list_tasks(ListTasksQuery {
                space_ref: Some(space.id.as_str().to_owned()),
                view: Some(crate::domain::ViewMode::Todo),
                sort: None,
                allow_archived_space: false,
            })
            .unwrap();

        assert!(changed);
        assert!(!app.should_quit);
        assert!(!reloaded.archived);
        assert!(
            listed
                .entries
                .iter()
                .any(|entry| entry.task.id == parent.id)
        );
        assert!(app.status_message.as_deref().is_some_and(|message| {
            message.contains("cannot be completed")
                && message.contains("finish or close every subtask first")
        }));
    }
}
