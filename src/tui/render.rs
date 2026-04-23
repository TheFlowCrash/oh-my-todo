use crate::domain::{FocusArea, SpaceListMode, TaskStatus, ViewMode};
use crate::tui::app::{
    ConfirmModal, FormModal, Mode, MouseTarget, SpaceFormMode, TaskFormField, TaskFormMode, TuiApp,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

const ACCENT: Color = Color::Cyan;
const PANEL_BG: Color = Color::Rgb(20, 24, 28);
const SUBTLE_BG: Color = Color::Rgb(32, 38, 44);
const HOVER_BG: Color = Color::Rgb(48, 58, 68);
const BORDER: Color = Color::Rgb(85, 98, 110);
const TEXT: Color = Color::Rgb(224, 228, 232);
const MUTED: Color = Color::Rgb(145, 155, 165);
const DANGER: Color = Color::Rgb(220, 90, 90);

pub fn render(frame: &mut Frame, app: &mut TuiApp) {
    app.begin_frame();
    let area = frame.area();
    app.set_frame_area(area);
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]);
    let [status_area, spaces_area, body_area, footer_area] = area.layout(&layout);

    render_status_bar(frame, status_area, app);
    render_spaces(frame, spaces_area, app);
    if app.is_narrow(area.width) {
        render_narrow_body(frame, body_area, app);
    } else {
        render_wide_body(frame, body_area, app);
    }
    render_footer(frame, footer_area, app);

    match app.mode.clone() {
        Mode::Form(FormModal::Space(form)) => render_space_form(frame, area, app, &form),
        Mode::Form(FormModal::Task(form)) => render_task_form(frame, area, app, &form),
        Mode::Form(FormModal::Log(form)) => render_log_form(frame, area, app, &form),
        Mode::Confirm(ConfirmModal::PurgeTask(confirm)) => {
            render_purge_confirm(frame, area, app, &confirm)
        }
        Mode::Confirm(ConfirmModal::PurgeSpace(confirm)) => {
            render_space_purge_confirm(frame, area, app, &confirm)
        }
        Mode::Filter(filter) => render_filter_form(frame, area, app, &filter),
        Mode::Help => render_help_overlay(frame, area, app),
        Mode::Browse => {}
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(PANEL_BG)),
        area,
    );

    let layout = Layout::horizontal([
        Constraint::Length(14),
        Constraint::Length(1),
        Constraint::Length(22),
        Constraint::Length(16),
        Constraint::Length(18),
        Constraint::Length(34),
        Constraint::Min(10),
    ]);
    let [
        title_area,
        spacer_area,
        view_area,
        sort_area,
        filter_area,
        space_area,
        message_area,
    ] = area.layout(&layout);

    frame.render_widget(
        Paragraph::new(" oh-my-todo ")
            .style(
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Left),
        title_area,
    );

    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(PANEL_BG)),
        spacer_area,
    );

    render_inline_buttons(
        frame,
        app,
        view_area,
        &[
            (
                "Todo",
                MouseTarget::SwitchView(ViewMode::Todo),
                app.current_view == ViewMode::Todo,
                true,
                false,
            ),
            (
                "Archive",
                MouseTarget::SwitchView(ViewMode::Archive),
                app.current_view == ViewMode::Archive,
                true,
                false,
            ),
            (
                "All",
                MouseTarget::SwitchView(ViewMode::All),
                app.current_view == ViewMode::All,
                true,
                false,
            ),
        ],
    );

    let sort_button = button_rect(sort_area.x, sort_area.y, sort_area.width.min(16), 1);
    render_button(
        frame,
        sort_button,
        &format!("Sort: {}", sort_label(app.current_sort)),
        false,
        true,
        false,
        app.is_hovered(&MouseTarget::CycleSort),
    );
    app.register_hitbox(sort_button, MouseTarget::CycleSort);

    let filter_target = MouseTarget::OpenFilter;
    let filter_button = button_rect(filter_area.x, filter_area.y, filter_area.width.min(18), 1);
    render_button(
        frame,
        filter_button,
        &app.filter_label(),
        !app.task_filter.trim().is_empty(),
        true,
        false,
        app.is_hovered(&filter_target),
    );
    app.register_hitbox(filter_button, filter_target);

    frame.render_widget(
        Paragraph::new(app.space_context_label()).style(
            Style::default()
                .fg(TEXT)
                .bg(PANEL_BG)
                .add_modifier(Modifier::BOLD),
        ),
        space_area,
    );

    let message = app
        .status_message
        .clone()
        .unwrap_or_else(|| format!("active: {}", active_area_label(app.focus_area)));
    frame.render_widget(
        Paragraph::new(message)
            .alignment(Alignment::Right)
            .style(Style::default().fg(MUTED).bg(PANEL_BG)),
        message_area,
    );
}

fn render_spaces(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let block = panel_block("Spaces", app.focus_area == FocusArea::Spaces);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let selected_space = app.current_space().cloned();
    let rename_enabled = selected_space.is_some();
    let archive_enabled = selected_space
        .as_ref()
        .is_some_and(|space| space.space.state.is_active());
    let restore_enabled = selected_space
        .as_ref()
        .is_some_and(|space| space.space.state.is_archived());
    let purge_enabled = restore_enabled;

    let mut right_edge = inner.right();
    let purge_area = allocate_right_button(&mut right_edge, inner.y, "Purge");
    let lifecycle_label = if restore_enabled {
        "Restore"
    } else {
        "Archive"
    };
    let lifecycle_area = allocate_right_button(&mut right_edge, inner.y, lifecycle_label);
    let rename_area = allocate_right_button(&mut right_edge, inner.y, "Rename");
    let new_area = allocate_right_button(&mut right_edge, inner.y, "+ New");
    let all_area = allocate_right_button(&mut right_edge, inner.y, "All");
    let active_area = allocate_right_button(&mut right_edge, inner.y, "Active");

    render_button(
        frame,
        active_area,
        "Active",
        app.space_list_mode == SpaceListMode::Active,
        true,
        false,
        app.is_hovered(&MouseTarget::SetSpaceListMode(SpaceListMode::Active)),
    );
    app.register_hitbox(
        active_area,
        MouseTarget::SetSpaceListMode(SpaceListMode::Active),
    );

    render_button(
        frame,
        all_area,
        "All",
        app.space_list_mode == SpaceListMode::All,
        true,
        false,
        app.is_hovered(&MouseTarget::SetSpaceListMode(SpaceListMode::All)),
    );
    app.register_hitbox(all_area, MouseTarget::SetSpaceListMode(SpaceListMode::All));

    render_button(
        frame,
        new_area,
        "+ New",
        false,
        true,
        false,
        app.is_hovered(&MouseTarget::OpenSpaceCreate),
    );
    app.register_hitbox(new_area, MouseTarget::OpenSpaceCreate);

    render_button(
        frame,
        rename_area,
        "Rename",
        false,
        rename_enabled,
        false,
        app.is_hovered(&MouseTarget::OpenSpaceRename),
    );
    if rename_enabled {
        app.register_hitbox(rename_area, MouseTarget::OpenSpaceRename);
    }

    let lifecycle_target = if restore_enabled {
        MouseTarget::RestoreSpace
    } else {
        MouseTarget::ArchiveSpace
    };
    render_button(
        frame,
        lifecycle_area,
        lifecycle_label,
        false,
        archive_enabled || restore_enabled,
        false,
        app.is_hovered(&lifecycle_target),
    );
    if archive_enabled || restore_enabled {
        app.register_hitbox(lifecycle_area, lifecycle_target);
    }

    render_button(
        frame,
        purge_area,
        "Purge",
        false,
        purge_enabled,
        true,
        app.is_hovered(&MouseTarget::OpenPurgeSpace),
    );
    if purge_enabled {
        app.register_hitbox(purge_area, MouseTarget::OpenPurgeSpace);
    }

    let tabs_area = Rect::new(inner.x, inner.y, right_edge.saturating_sub(inner.x), 1);

    if app.spaces.is_empty() {
        frame.render_widget(
            Paragraph::new("No spaces yet. Click + New to create your first space.")
                .style(Style::default().fg(MUTED)),
            tabs_area,
        );
        return;
    }

    let mut cursor_x = tabs_area.x;
    for index in 0..app.spaces.len() {
        let label = if app.spaces[index].space.state.is_archived() {
            format!("[a] {}", app.spaces[index].space.name)
        } else {
            app.spaces[index].space.name.clone()
        };
        let width = (label.len() as u16 + 2).min(tabs_area.right().saturating_sub(cursor_x));
        if width == 0 {
            break;
        }

        let rect = Rect::new(cursor_x, tabs_area.y, width, 1);
        let is_active = index == app.space_index;
        let target = MouseTarget::SwitchSpace(index);
        render_button(
            frame,
            rect,
            &label,
            is_active,
            true,
            false,
            app.is_hovered(&target),
        );
        app.register_hitbox(rect, target);
        cursor_x = rect.right().saturating_add(1);
        if cursor_x >= tabs_area.right() {
            break;
        }
    }
}

fn render_wide_body(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let layout = Layout::horizontal([Constraint::Percentage(46), Constraint::Percentage(54)]);
    let [tree_area, details_area] = area.layout(&layout);
    render_task_tree(frame, tree_area, app);
    render_details(
        frame,
        details_area,
        app,
        app.focus_area == FocusArea::Details,
    );
}

fn render_narrow_body(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    render_task_tree(frame, area, app);
    if app.focus_area == FocusArea::Details {
        let overlay = centered_rect(area, 94, 84);
        frame.render_widget(Clear, overlay);
        render_details(frame, overlay, app, true);
    }
}

fn render_task_tree(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let block = panel_block("Task Tree", app.focus_area == FocusArea::TaskTree);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    app.set_task_tree_viewport(inner);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if app.visible_tasks.is_empty() {
        frame.render_widget(
            Paragraph::new(app.task_tree_empty_message())
                .style(Style::default().fg(MUTED))
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let max_offset = app
        .visible_tasks
        .len()
        .saturating_sub(inner.height as usize);
    let offset = app.task_list_state.offset().min(max_offset);
    *app.task_list_state.offset_mut() = offset;

    let visible_count = inner.height as usize;
    for row in 0..visible_count {
        let item_index = offset + row;
        let Some(entry) = app.visible_tasks.get(item_index).cloned() else {
            break;
        };
        let row_rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        let selected = app.task_list_state.selected() == Some(item_index);
        let hovered = app.is_hovered(&MouseTarget::SelectTask(item_index));
        let row_style = if selected {
            Style::default().bg(SUBTLE_BG).fg(TEXT)
        } else if hovered {
            Style::default().bg(HOVER_BG).fg(TEXT)
        } else {
            Style::default().bg(PANEL_BG).fg(TEXT)
        };
        let branch = match (entry.child_count > 0, entry.is_expanded) {
            (true, true) => "v",
            (true, false) => ">",
            (false, _) => "-",
        };
        let line = Line::from(vec![
            Span::raw("  ".repeat(entry.depth)),
            Span::styled(branch, Style::default().fg(MUTED)),
            Span::raw(" "),
            Span::styled(
                status_marker(entry.task.status),
                Style::default().fg(status_color(entry.task.status)),
            ),
            Span::raw(" "),
            Span::styled(entry.task.title.clone(), Style::default().fg(TEXT)),
        ]);

        frame.render_widget(Paragraph::new(line).style(row_style), row_rect);
        app.register_hitbox(row_rect, MouseTarget::SelectTask(item_index));

        if entry.child_count > 0 {
            let toggle_rect = Rect::new(
                inner.x.saturating_add((entry.depth as u16) * 2),
                row_rect.y,
                1,
                1,
            );
            app.register_hitbox(toggle_rect, MouseTarget::ToggleTask(entry.task.id.clone()));
        }
    }
}

fn render_details(frame: &mut Frame, area: Rect, app: &mut TuiApp, focused: bool) {
    let block = panel_block("Details", focused);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let layout = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]);
    let [toolbar_area, content_area] = inner.layout(&layout);
    render_details_toolbar(frame, toolbar_area, app, area.width < 100);
    app.set_details_viewport(content_area);

    let text = if let Some(details) = app.details.as_ref() {
        let logs = if details.logs.is_empty() {
            vec![Line::from(Span::styled("-", Style::default().fg(MUTED)))]
        } else {
            details
                .logs
                .iter()
                .map(|log| {
                    Line::from(vec![
                        Span::styled(format_timestamp(log.at), Style::default().fg(ACCENT)),
                        Span::raw(" "),
                        Span::styled(log.message.clone(), Style::default().fg(TEXT)),
                    ])
                })
                .collect::<Vec<_>>()
        };

        let mut lines = vec![
            kv_line("Title", &details.task.title),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(MUTED)),
                Span::styled(
                    status_label(details.task.status),
                    Style::default().fg(status_color(details.task.status)),
                ),
            ]),
            kv_line("Space", &details.space.slug),
            kv_line(
                "Parent",
                &details
                    .parent
                    .as_ref()
                    .map(|task| task.title.clone())
                    .unwrap_or_else(|| "-".to_owned()),
            ),
            Line::from(""),
            Line::from(Span::styled(
                "Description",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            )),
            Line::from(
                details
                    .task
                    .description
                    .clone()
                    .unwrap_or_else(|| "-".to_owned()),
            ),
            Line::from(""),
            Line::from(Span::styled(
                "Recent Logs",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            )),
        ];
        lines.extend(logs);
        Text::from(lines)
    } else {
        let mut lines = vec![Line::from(Span::styled(
            "Select a task to view details.",
            Style::default().fg(MUTED),
        ))];
        lines.push(Line::from(""));
        if app.current_space().is_none() {
            lines.push(Line::from(Span::styled(
                "Create or select a space to begin.",
                Style::default().fg(MUTED),
            )));
        } else if app.can_mutate_viewed_space() {
            lines.push(Line::from(Span::styled(
                "Click + Task to create the first task in this space.",
                Style::default().fg(MUTED),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "This archived space is read-only here. Restore it to edit tasks.",
                Style::default().fg(MUTED),
            )));
        }
        Text::from(lines)
    };

    frame.render_widget(
        Paragraph::new(text)
            .scroll((app.details_scroll.min(10_000) as u16, 0))
            .style(Style::default().fg(TEXT))
            .wrap(Wrap { trim: false }),
        content_area,
    );
}

fn render_details_toolbar(frame: &mut Frame, area: Rect, app: &mut TuiApp, narrow: bool) {
    let selected = app.details.as_ref().map(|details| details.task.status);
    let can_mutate = app.can_mutate_viewed_space();
    let can_create = app.current_space().is_some() && can_mutate;
    let can_act = selected.is_some() && can_mutate;
    let can_restore = matches!(selected, Some(TaskStatus::Archived)) && can_mutate;
    let can_archive = matches!(
        selected,
        Some(TaskStatus::Todo | TaskStatus::InProgress | TaskStatus::Done)
    ) && can_mutate;
    let can_purge = matches!(selected, Some(TaskStatus::Archived)) && can_mutate;
    let can_status =
        selected.is_some() && !matches!(selected, Some(TaskStatus::Archived)) && can_mutate;
    let can_reorder = can_act && app.current_sort == crate::domain::SortMode::Manual;

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let [top_row, middle_row, bottom_row] = area.layout(&rows);
    if narrow {
        render_inline_buttons(
            frame,
            app,
            top_row,
            &[
                ("Back", MouseTarget::CloseDetails, false, true, false),
                ("+ Task", MouseTarget::CreateTask, false, can_create, false),
                (
                    "+ Subtask",
                    MouseTarget::CreateSubtask,
                    false,
                    can_act,
                    false,
                ),
                ("Edit", MouseTarget::EditTask, false, can_act, false),
                ("Log", MouseTarget::AddLog, false, can_act, false),
            ],
        );
    } else {
        render_inline_buttons(
            frame,
            app,
            top_row,
            &[
                ("+ Task", MouseTarget::CreateTask, false, can_create, false),
                (
                    "+ Subtask",
                    MouseTarget::CreateSubtask,
                    false,
                    can_act,
                    false,
                ),
                ("Edit", MouseTarget::EditTask, false, can_act, false),
                ("Log", MouseTarget::AddLog, false, can_act, false),
            ],
        );
    }
    render_inline_buttons(
        frame,
        app,
        middle_row,
        &[
            (
                "Todo",
                MouseTarget::SetTaskStatus(TaskStatus::Todo),
                matches!(selected, Some(TaskStatus::Todo)),
                can_status,
                false,
            ),
            (
                "Doing",
                MouseTarget::SetTaskStatus(TaskStatus::InProgress),
                matches!(selected, Some(TaskStatus::InProgress)),
                can_status,
                false,
            ),
            (
                "Done",
                MouseTarget::SetTaskStatus(TaskStatus::Done),
                matches!(selected, Some(TaskStatus::Done)),
                can_status,
                false,
            ),
            (
                "Archive",
                MouseTarget::ArchiveTask,
                false,
                can_archive,
                false,
            ),
            (
                "Restore",
                MouseTarget::RestoreTask,
                false,
                can_restore,
                false,
            ),
            ("Purge", MouseTarget::OpenPurgeTask, false, can_purge, true),
        ],
    );
    render_inline_buttons(
        frame,
        app,
        bottom_row,
        &[
            (
                "Move Up",
                MouseTarget::MoveTask(crate::application::commands::MoveTaskDirection::Up),
                false,
                can_reorder,
                false,
            ),
            (
                "Move Down",
                MouseTarget::MoveTask(crate::application::commands::MoveTaskDirection::Down),
                false,
                can_reorder,
                false,
            ),
        ],
    );
}

fn render_footer(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let layout = Layout::horizontal([Constraint::Fill(1), Constraint::Length(8)]);
    let [text_area, help_area] = area.layout(&layout);
    frame.render_widget(
        Paragraph::new(app.help_text())
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL_BG)),
        text_area,
    );

    let help_target = MouseTarget::OpenHelp;
    render_button(
        frame,
        help_area,
        "Help",
        matches!(&app.mode, Mode::Help),
        true,
        false,
        app.is_hovered(&help_target),
    );
    app.register_hitbox(help_area, help_target);
}

fn render_space_form(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    form: &crate::tui::app::SpaceFormState,
) {
    let popup = centered_rect(area, 58, 28);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title(match form.mode {
            SpaceFormMode::Create => "New Space",
            SpaceFormMode::Rename { .. } => "Rename Space",
        })
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let [input_area, button_area, hint_area] = inner.layout(&layout);
    render_input_field(frame, input_area, "Name", &form.name, true);
    app.register_hitbox(input_area, MouseTarget::SpaceFormInput);
    render_button_row(
        frame,
        app,
        button_area,
        &[
            ("Save", MouseTarget::SpaceFormSave, true, false),
            ("Cancel", MouseTarget::SpaceFormCancel, true, false),
        ],
    );
    frame.render_widget(
        Paragraph::new("Type in the field, then click Save or Cancel. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );
    set_input_cursor(frame, input_area, &form.name);
}

fn render_task_form(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    form: &crate::tui::app::TaskFormState,
) {
    let popup = centered_rect(area, 76, 74);
    frame.render_widget(Clear, popup);
    let title = match form.mode {
        TaskFormMode::CreateRoot => "New Task",
        TaskFormMode::CreateChild { .. } => "New Subtask",
        TaskFormMode::Edit { .. } => "Edit Task",
    };
    let block = Block::bordered()
        .title(title)
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let [
        title_area,
        status_area,
        description_area,
        button_area,
        hint_area,
    ] = inner.layout(&layout);

    render_input_field(
        frame,
        title_area,
        "Title",
        &form.title,
        form.focus == TaskFormField::Title,
    );
    app.register_hitbox(title_area, MouseTarget::TaskFormTitle);

    render_status_picker(frame, status_area, app, form.status);

    render_text_area(
        frame,
        description_area,
        "Description",
        &form.description,
        form.focus == TaskFormField::Description,
    );
    app.register_hitbox(description_area, MouseTarget::TaskFormDescription);

    render_button_row(
        frame,
        app,
        button_area,
        &[
            ("Save", MouseTarget::TaskFormSave, true, false),
            ("Cancel", MouseTarget::TaskFormCancel, true, false),
        ],
    );
    frame.render_widget(
        Paragraph::new("Click fields to edit, click a status chip, then click Save or Cancel. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );

    match form.focus {
        TaskFormField::Title => set_input_cursor(frame, title_area, &form.title),
        TaskFormField::Description => {
            set_text_area_cursor(frame, description_area, &form.description)
        }
        TaskFormField::Status => {}
    }
}

fn render_log_form(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    form: &crate::tui::app::LogFormState,
) {
    let popup = centered_rect(area, 76, 66);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title(format!("Add Log: {}", form.task_title))
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    let layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let [input_area, button_area, hint_area] = inner.layout(&layout);
    render_text_area(frame, input_area, "Message", &form.input, true);
    app.register_hitbox(input_area, MouseTarget::LogFormInput);
    render_button_row(
        frame,
        app,
        button_area,
        &[
            ("Save", MouseTarget::LogFormSave, true, false),
            ("Cancel", MouseTarget::LogFormCancel, true, false),
        ],
    );
    frame.render_widget(
        Paragraph::new("Type in the message box, then click Save or Cancel. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );
    set_text_area_cursor(frame, input_area, &form.input);
}

fn render_purge_confirm(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    confirm: &crate::tui::app::PurgeTaskConfirmState,
) {
    let popup = centered_rect(area, 62, 38);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title("Purge Task")
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(if confirm.requires_phrase { 3 } else { 0 }),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let [message_area, info_area, phrase_area, button_area, hint_area] = inner.layout(&layout);

    frame.render_widget(
        Paragraph::new(format!(
            "{} will permanently remove {} task(s).",
            confirm.task_title, confirm.affected_count
        )),
        message_area,
    );
    frame.render_widget(
        Paragraph::new(if confirm.requires_phrase {
            "Type `purge` to continue."
        } else {
            "Click Purge to continue."
        })
        .style(Style::default().fg(MUTED)),
        info_area,
    );

    if confirm.requires_phrase {
        render_input_field(frame, phrase_area, "Type purge", &confirm.phrase, true);
        app.register_hitbox(phrase_area, MouseTarget::ConfirmPhraseInput);
        set_input_cursor(frame, phrase_area, &confirm.phrase);
    }

    render_button_row(
        frame,
        app,
        button_area,
        &[
            ("Cancel", MouseTarget::ConfirmCancel, true, false),
            ("Purge", MouseTarget::ConfirmPurge, true, true),
        ],
    );
    frame.render_widget(
        Paragraph::new("Click Cancel or Purge. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );
}

fn render_space_purge_confirm(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    confirm: &crate::tui::app::PurgeSpaceConfirmState,
) {
    let popup = centered_rect(area, 66, 42);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title("Purge Space")
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let [message_area, info_area, phrase_area, button_area, hint_area] = inner.layout(&layout);

    frame.render_widget(
        Paragraph::new(format!(
            "{} will permanently remove {} task record(s).",
            confirm.space_name, confirm.task_count
        )),
        message_area,
    );
    frame.render_widget(
        Paragraph::new("Type `purge` to continue.").style(Style::default().fg(MUTED)),
        info_area,
    );
    render_input_field(frame, phrase_area, "Type purge", &confirm.phrase, true);
    app.register_hitbox(phrase_area, MouseTarget::ConfirmPhraseInput);
    set_input_cursor(frame, phrase_area, &confirm.phrase);
    render_button_row(
        frame,
        app,
        button_area,
        &[
            ("Cancel", MouseTarget::ConfirmCancel, true, false),
            ("Purge", MouseTarget::ConfirmPurge, true, true),
        ],
    );
    frame.render_widget(
        Paragraph::new("Click Cancel or Purge. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );
}

fn render_filter_form(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    filter: &crate::tui::app::FilterState,
) {
    let popup = centered_rect(area, 62, 30);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title("Filter Tasks")
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let [input_area, button_area, hint_area] = inner.layout(&layout);
    render_input_field(frame, input_area, "Query", &filter.input, true);
    app.register_hitbox(input_area, MouseTarget::FilterInput);
    render_button_row(
        frame,
        app,
        button_area,
        &[
            ("Apply", MouseTarget::FilterApply, true, false),
            ("Clear", MouseTarget::FilterClear, true, false),
            ("Cancel", MouseTarget::FilterCancel, true, false),
        ],
    );
    frame.render_widget(
        Paragraph::new(
            "Matches task title, description, logs, and ids. Click Apply, Clear, or Cancel.",
        )
        .style(Style::default().fg(MUTED)),
        hint_area,
    );
    set_input_cursor(frame, input_area, &filter.input);
}

fn render_help_overlay(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let popup = centered_rect(area, 74, 64);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title("Help")
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]);
    let [content_area, button_area] = inner.layout(&layout);
    let help_lines = vec![
        Line::from(Span::styled(
            "Mouse-first workflow",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from("- Click spaces to switch context; archived spaces appear in All mode."),
        Line::from(
            "- Click Filter to narrow the current task tree by title, description, logs, or ids.",
        ),
        Line::from("- Use manual sort plus Move Up/Move Down for sibling reordering."),
        Line::from("- Archived spaces are read-only until restored."),
        Line::from("- Purge always requires typing `purge` in a confirm dialog."),
        Line::from(""),
        Line::from(Span::styled(
            "Optional keyboard helpers",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from("- `?` opens or closes this help overlay."),
        Line::from("- `/` opens the filter dialog."),
        Line::from("- `Esc` closes help and filter overlays."),
        Line::from("- `Ctrl+C` always exits safely."),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(help_lines))
            .style(Style::default().fg(TEXT))
            .wrap(Wrap { trim: false }),
        content_area,
    );
    render_button_row(
        frame,
        app,
        button_area,
        &[("Close", MouseTarget::CloseHelp, true, false)],
    );
}

fn render_status_picker(frame: &mut Frame, area: Rect, app: &mut TuiApp, status: TaskStatus) {
    let block = Block::bordered()
        .title("Status")
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    render_inline_buttons(
        frame,
        app,
        inner,
        &[
            (
                "Todo",
                MouseTarget::TaskFormStatus(TaskStatus::Todo),
                status == TaskStatus::Todo,
                true,
                false,
            ),
            (
                "Doing",
                MouseTarget::TaskFormStatus(TaskStatus::InProgress),
                status == TaskStatus::InProgress,
                true,
                false,
            ),
            (
                "Done",
                MouseTarget::TaskFormStatus(TaskStatus::Done),
                status == TaskStatus::Done,
                true,
                false,
            ),
        ],
    );
}

fn render_button_row(
    frame: &mut Frame,
    app: &mut TuiApp,
    area: Rect,
    buttons: &[(&str, MouseTarget, bool, bool)],
) {
    let mut x = area.x;
    for (label, target, enabled, danger) in
        buttons.iter().map(|(a, b, c, d)| (*a, b.clone(), *c, *d))
    {
        let width = label.len() as u16 + 2;
        if x.saturating_add(width) > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        render_button(
            frame,
            rect,
            label,
            false,
            enabled,
            danger,
            app.is_hovered(&target),
        );
        if enabled {
            app.register_hitbox(rect, target.clone());
        }
        x = rect.right().saturating_add(2);
    }
}

fn render_inline_buttons(
    frame: &mut Frame,
    app: &mut TuiApp,
    area: Rect,
    buttons: &[(&str, MouseTarget, bool, bool, bool)],
) {
    let mut x = area.x;
    for (label, target, selected, enabled, danger) in buttons
        .iter()
        .map(|(a, b, c, d, e)| (*a, b.clone(), *c, *d, *e))
    {
        let width = label.len() as u16 + 2;
        if x.saturating_add(width) > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        render_button(
            frame,
            rect,
            label,
            selected,
            enabled,
            danger,
            app.is_hovered(&target),
        );
        if enabled {
            app.register_hitbox(rect, target.clone());
        }
        x = rect.right().saturating_add(1);
    }
}

fn render_button(
    frame: &mut Frame,
    rect: Rect,
    label: &str,
    selected: bool,
    enabled: bool,
    danger: bool,
    hovered: bool,
) {
    let style = if !enabled {
        Style::default().fg(MUTED).bg(PANEL_BG)
    } else if danger {
        Style::default()
            .fg(Color::White)
            .bg(if hovered {
                Color::Rgb(240, 110, 110)
            } else {
                DANGER
            })
            .add_modifier(Modifier::BOLD)
    } else if selected {
        Style::default()
            .fg(Color::Black)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD)
    } else if hovered {
        Style::default().fg(TEXT).bg(HOVER_BG)
    } else {
        Style::default().fg(TEXT).bg(SUBTLE_BG)
    };
    frame.render_widget(
        Paragraph::new(format!(" {label} "))
            .style(style)
            .alignment(Alignment::Center),
        rect,
    );
}

fn render_input_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    input: &crate::tui::input::TextInput,
    focused: bool,
) {
    let block = Block::bordered()
        .title(label)
        .border_style(border_style(focused))
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    frame.render_widget(Paragraph::new(input.value()).block(block), area);
}

fn render_text_area(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    input: &crate::tui::input::TextInput,
    focused: bool,
) {
    let block = Block::bordered()
        .title(label)
        .border_style(border_style(focused))
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    let text = if input.value().is_empty() {
        Text::from(vec![Line::from("")])
    } else {
        Text::from(
            input
                .lines()
                .iter()
                .cloned()
                .map(Line::from)
                .collect::<Vec<_>>(),
        )
    };
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn set_input_cursor(frame: &mut Frame, area: Rect, input: &crate::tui::input::TextInput) {
    let (_, col) = input.cursor();
    frame.set_cursor_position(Position::new(area.x + 1 + col as u16, area.y + 1));
}

fn set_text_area_cursor(frame: &mut Frame, area: Rect, input: &crate::tui::input::TextInput) {
    let (row, col) = input.cursor();
    frame.set_cursor_position(Position::new(
        area.x + 1 + col as u16,
        area.y + 1 + row as u16,
    ));
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    Block::bordered()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused))
        .style(Style::default().bg(PANEL_BG).fg(TEXT))
}

fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(BORDER)
    }
}

fn kv_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(MUTED)),
        Span::styled(value.to_owned(), Style::default().fg(TEXT)),
    ])
}

fn status_marker(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "[ ]",
        TaskStatus::InProgress => "[~]",
        TaskStatus::Done => "[x]",
        TaskStatus::Archived => "[a]",
    }
}

fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Archived => "archived",
    }
}

fn status_color(status: TaskStatus) -> Color {
    match status {
        TaskStatus::Todo => TEXT,
        TaskStatus::InProgress => Color::Yellow,
        TaskStatus::Done => Color::Green,
        TaskStatus::Archived => MUTED,
    }
}

fn sort_label(sort: crate::domain::SortMode) -> &'static str {
    match sort {
        crate::domain::SortMode::Created => "created",
        crate::domain::SortMode::Updated => "updated",
        crate::domain::SortMode::Status => "status",
        crate::domain::SortMode::Manual => "manual",
    }
}

fn active_area_label(area: FocusArea) -> &'static str {
    match area {
        FocusArea::Spaces => "spaces",
        FocusArea::TaskTree => "task-tree",
        FocusArea::Details => "details",
    }
}

fn format_timestamp(value: time::OffsetDateTime) -> String {
    value
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| value.to_string())
}

fn allocate_right_button(right_edge: &mut u16, y: u16, label: &str) -> Rect {
    let width = label.len() as u16 + 2;
    let x = right_edge.saturating_sub(width);
    let rect = Rect::new(x, y, width.max(1), 1);
    *right_edge = x.saturating_sub(1);
    rect
}

fn button_rect(x: u16, y: u16, width: u16, height: u16) -> Rect {
    Rect::new(x, y, width.max(1), height.max(1))
}

fn centered_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - height_pct) / 2),
        Constraint::Percentage(height_pct),
        Constraint::Percentage((100 - height_pct) / 2),
    ]);
    let [_, middle, _] = area.layout(&vertical);
    let horizontal = Layout::horizontal([
        Constraint::Percentage((100 - width_pct) / 2),
        Constraint::Percentage(width_pct),
        Constraint::Percentage((100 - width_pct) / 2),
    ]);
    let [_, center, _] = middle.layout(&horizontal);
    center
}
