use crate::domain::{FocusArea, SpaceListMode, TaskStatus, ViewMode};
use crate::tui::app::{
    ConfirmModal, FormModal, Mode, MouseTarget, SpaceFormMode, TaskFormField, TaskFormMode, TuiApp,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

const ACCENT: Color = Color::Rgb(138, 211, 208);
const SCREEN_BG: Color = Color::Rgb(22, 25, 29);
const HEADER_BG: Color = Color::Rgb(36, 41, 46);
const PANEL_BG: Color = Color::Rgb(30, 34, 39);
const SUBTLE_BG: Color = Color::Rgb(44, 49, 55);
const HOVER_BG: Color = Color::Rgb(58, 66, 74);
const BORDER: Color = Color::Rgb(118, 129, 140);
const TEXT: Color = Color::Rgb(236, 239, 242);
const MUTED: Color = Color::Rgb(172, 180, 188);
const DANGER: Color = Color::Rgb(224, 108, 108);

pub fn render(frame: &mut Frame, app: &mut TuiApp) {
    app.begin_frame();
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(SCREEN_BG)),
        frame.area(),
    );
    let area = inset_rect(frame.area(), 1, 1);
    if area.width == 0 || area.height == 0 {
        return;
    }
    app.set_frame_area(area);
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]);
    let [status_area, body_area, footer_area] = area.layout(&layout);

    render_status_bar(frame, status_area, app);
    if app.is_narrow(area.width) {
        render_narrow_body(frame, body_area, app);
    } else {
        render_wide_body(frame, body_area, app);
    }
    render_footer(frame, footer_area, app);

    match app.mode.clone() {
        Mode::SpaceManager(manager) => render_space_manager(frame, area, app, &manager),
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
    let block = header_block();
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let content_layout = Layout::horizontal([Constraint::Min(24), Constraint::Length(21)]);
    let [meta_area, tabs_area] = inner.layout(&content_layout);
    render_header_links(frame, meta_area, app);
    render_view_tabs(frame, tabs_area, app);
}

fn render_header_links(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let items = vec![
        (
            app.space_button_label(),
            MouseTarget::OpenSpaceManager,
            matches!(&app.mode, Mode::SpaceManager(_)),
        ),
        (
            format!("Sort: {}", sort_label(app.current_sort)),
            MouseTarget::CycleSort,
            false,
        ),
        (
            app.filter_label(),
            MouseTarget::OpenFilter,
            !app.task_filter.trim().is_empty(),
        ),
    ];
    let mut spans = Vec::new();
    let mut x = area.x;

    for (index, (label, target, active)) in items.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("   ", Style::default().fg(MUTED)));
            x = x.saturating_add(3);
        }

        let hovered = app.is_hovered(&target);
        let style = if active {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else if hovered {
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
        } else if index == 0 {
            Style::default().fg(TEXT)
        } else {
            Style::default().fg(MUTED)
        };
        let width = label.chars().count() as u16;
        if width == 0 || x >= area.right() {
            break;
        }
        let rect = Rect::new(x, area.y, width.min(area.right().saturating_sub(x)), 1);
        spans.push(Span::styled(label, style));
        app.register_hitbox(rect, target);
        x = x.saturating_add(width);
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(HEADER_BG)),
        area,
    );
}

fn render_view_tabs(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let items = [
        (
            "Todo",
            MouseTarget::SwitchView(ViewMode::Todo),
            app.current_view == ViewMode::Todo,
        ),
        (
            "Archive",
            MouseTarget::SwitchView(ViewMode::Archive),
            app.current_view == ViewMode::Archive,
        ),
        (
            "All",
            MouseTarget::SwitchView(ViewMode::All),
            app.current_view == ViewMode::All,
        ),
    ];
    let separator = " \u{2506} ";
    let separator_width = separator.chars().count();
    let total_width = items
        .iter()
        .map(|(label, _, _)| label.chars().count())
        .sum::<usize>()
        + separator_width * items.len().saturating_sub(1);
    let start_x = area
        .right()
        .saturating_sub(total_width.min(area.width as usize) as u16);
    let draw_area = Rect::new(start_x, area.y, area.right().saturating_sub(start_x), 1);
    let mut spans = Vec::new();
    let mut x = draw_area.x;

    for (index, (label, target, selected)) in items.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(separator, Style::default().fg(BORDER)));
            x = x.saturating_add(separator_width as u16);
        }

        let hovered = app.is_hovered(&target);
        let style = if selected {
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
        } else if hovered {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(MUTED)
        };
        let width = label.chars().count() as u16;
        if width == 0 || x >= draw_area.right() {
            break;
        }
        spans.push(Span::styled(label, style));
        app.register_hitbox(
            Rect::new(
                x,
                draw_area.y,
                width.min(draw_area.right().saturating_sub(x)),
                1,
            ),
            target,
        );
        x = x.saturating_add(width);
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Right)
            .style(Style::default().bg(HEADER_BG)),
        draw_area,
    );
}

fn render_space_manager(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    _manager: &crate::tui::app::SpaceManagerState,
) {
    let popup = centered_rect(area, 82, 76);
    frame.render_widget(Clear, popup);
    let block = panel_block("Space Manager", true);
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let selected_space = app.selected_space_summary().cloned();
    let can_open = selected_space.is_some();
    let can_rename = selected_space.is_some();
    let can_archive = selected_space
        .as_ref()
        .is_some_and(|space| space.space.state.is_active());
    let can_restore = selected_space
        .as_ref()
        .is_some_and(|space| space.space.state.is_archived());
    let can_purge = can_restore;
    let lifecycle_label = if can_restore { "Restore" } else { "Archive" };
    let lifecycle_target = if can_restore {
        MouseTarget::RestoreSpace
    } else {
        MouseTarget::ArchiveSpace
    };

    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]);
    let [
        context_area,
        utility_area,
        action_area,
        body_area,
        hint_area,
    ] = inner.layout(&layout);

    frame.render_widget(
        Paragraph::new(space_manager_context_text(app))
            .style(Style::default().fg(MUTED).bg(PANEL_BG)),
        context_area,
    );

    render_inline_buttons(
        frame,
        app,
        utility_area,
        &[
            (
                "Active",
                MouseTarget::SetSpaceListMode(SpaceListMode::Active),
                app.space_list_mode == SpaceListMode::Active,
                true,
                false,
            ),
            (
                "All",
                MouseTarget::SetSpaceListMode(SpaceListMode::All),
                app.space_list_mode == SpaceListMode::All,
                true,
                false,
            ),
            ("+ New", MouseTarget::OpenSpaceCreate, false, true, false),
            ("Close", MouseTarget::CloseSpaceManager, false, true, false),
        ],
    );

    render_inline_buttons(
        frame,
        app,
        action_area,
        &[
            (
                "Open",
                MouseTarget::OpenSelectedSpace,
                false,
                can_open,
                false,
            ),
            (
                "Rename",
                MouseTarget::OpenSpaceRename,
                false,
                can_rename,
                false,
            ),
            (
                lifecycle_label,
                lifecycle_target,
                false,
                can_archive || can_restore,
                false,
            ),
            ("Purge", MouseTarget::OpenPurgeSpace, false, can_purge, true),
        ],
    );

    let body_layout = Layout::horizontal([Constraint::Percentage(46), Constraint::Percentage(54)]);
    let [list_area, summary_area] = body_area.layout(&body_layout);
    render_space_manager_list(frame, list_area, app);
    render_space_manager_summary(frame, summary_area, app);

    frame.render_widget(
        Paragraph::new(
            "Markers: * current, > viewed. Click a row to select it. Open changes the main context, while archive, restore, and purge act on the selected space.",
        )
        .style(Style::default().fg(MUTED)),
        hint_area,
    );
}

fn render_space_manager_list(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let block = panel_block("Spaces", true);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    app.set_space_manager_viewport(inner);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if app.spaces.is_empty() {
        frame.render_widget(
            Paragraph::new("No spaces yet. Click + New to create the first one.")
                .style(Style::default().fg(MUTED))
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    app.ensure_space_manager_selection_visible();
    let max_offset = app.spaces.len().saturating_sub(inner.height as usize);
    let offset = app.space_manager_scroll().min(max_offset);
    if offset != app.space_manager_scroll() {
        app.set_space_manager_scroll(offset);
    }
    let visible_count = inner.height as usize;

    for row in 0..visible_count {
        let index = offset + row;
        let Some(space) = app.spaces.get(index) else {
            break;
        };

        let row_rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        let selected = index == app.space_index;
        let target = MouseTarget::SelectManagedSpace(index);
        let hovered = app.is_hovered(&target);
        let row_style = if selected {
            Style::default().bg(SUBTLE_BG).fg(TEXT)
        } else if hovered {
            Style::default().bg(HOVER_BG).fg(TEXT)
        } else {
            Style::default().bg(PANEL_BG).fg(TEXT)
        };

        let current_marker = if app.current_active_space().map(|current| &current.space.id)
            == Some(&space.space.id)
        {
            "*"
        } else {
            " "
        };
        let viewed_marker =
            if app.current_space().map(|viewed| &viewed.space.id) == Some(&space.space.id) {
                ">"
            } else {
                " "
            };
        let status = if space.space.state.is_archived() {
            "archived"
        } else {
            "active"
        };
        let counts = format!(
            "{}/{}",
            space.counts.todo_tasks, space.counts.archived_tasks
        );
        let line = format!(
            "{}{} {:<16.16} [{:<8}] {:>5}",
            current_marker, viewed_marker, space.space.name, status, counts
        );

        frame.render_widget(Paragraph::new(line).style(row_style), row_rect);
        app.register_hitbox(row_rect, target);
    }
}

fn render_space_manager_summary(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let block = panel_block("Selection", true);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let text = if let Some(space) = app.selected_space_summary() {
        let role = match (
            app.current_active_space().map(|current| &current.space.id) == Some(&space.space.id),
            app.current_space().map(|viewed| &viewed.space.id) == Some(&space.space.id),
        ) {
            (true, true) => "current + viewed",
            (true, false) => "current",
            (false, true) => "viewed",
            (false, false) => "listed",
        };

        Text::from(vec![
            kv_line("Name", &space.space.name),
            kv_line("Slug", &space.space.slug),
            kv_line(
                "State",
                if space.space.state.is_archived() {
                    "archived"
                } else {
                    "active"
                },
            ),
            kv_line("Role", role),
            kv_line(
                "Tasks",
                &format!(
                    "{} active / {} archived",
                    space.counts.todo_tasks, space.counts.archived_tasks
                ),
            ),
            Line::from(""),
            Line::from(Span::styled(
                "How it works",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            )),
            Line::from("- Open switches the main working context to the selected space."),
            Line::from("- Archived spaces stay browseable but remain read-only until restored."),
            Line::from("- Purge permanently removes the whole space after confirmation."),
        ])
    } else {
        Text::from(vec![
            Line::from(Span::styled(
                "Select a space to inspect it.",
                Style::default().fg(MUTED),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Use Active or All to change which spaces appear in this manager.",
                Style::default().fg(MUTED),
            )),
        ])
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(TEXT))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn space_manager_context_text(app: &TuiApp) -> String {
    match (app.current_space(), app.current_active_space()) {
        (Some(viewed), Some(current)) if viewed.space.id != current.space.id => format!(
            "Viewed: {} [{}] | Current: {}",
            viewed.space.name,
            if viewed.space.state.is_archived() {
                "archived"
            } else {
                "active"
            },
            current.space.name,
        ),
        (Some(viewed), _) => format!(
            "Viewed: {} [{}]",
            viewed.space.name,
            if viewed.space.state.is_archived() {
                "archived"
            } else {
                "active"
            },
        ),
        (None, Some(current)) => format!("Current: {}", current.space.name),
        (None, None) => "No space selected yet.".to_owned(),
    }
}

fn render_wide_body(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let block = shell_block();
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let detail_height = detail_panel_height(inner.height);
    let layout = Layout::vertical([
        Constraint::Min(inner.height.saturating_sub(detail_height)),
        Constraint::Length(0),
        Constraint::Length(detail_height),
    ]);
    let [top_area, _, detail_area] = inner.layout(&layout);
    let row_layout = Layout::horizontal([
        Constraint::Fill(4),
        Constraint::Length(1),
        Constraint::Fill(3),
    ]);
    let [todo_area, _, inspector_area] = top_area.layout(&row_layout);

    render_task_tree(frame, todo_area, app);
    render_inspector(
        frame,
        inspector_area,
        app,
        app.focus_area == FocusArea::Details,
        false,
    );
    render_detail_panel(
        frame,
        detail_area,
        app,
        app.focus_area == FocusArea::Details,
    );
}

fn render_narrow_body(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let block = shell_block();
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    render_task_tree(frame, inner, app);
    if app.focus_area == FocusArea::Details {
        let overlay = centered_rect(inner, 96, 90);
        frame.render_widget(Clear, overlay);
        let shell = shell_block();
        frame.render_widget(shell.clone(), overlay);
        let overlay_inner = shell.inner(overlay);
        if overlay_inner.width == 0 || overlay_inner.height == 0 {
            return;
        }
        let detail_height = detail_panel_height(overlay_inner.height);
        let layout = Layout::vertical([
            Constraint::Min(overlay_inner.height.saturating_sub(detail_height)),
            Constraint::Length(0),
            Constraint::Length(detail_height),
        ]);
        let [inspector_area, _, detail_area] = overlay_inner.layout(&layout);
        render_inspector(frame, inspector_area, app, true, true);
        render_detail_panel(frame, detail_area, app, true);
    }
}

fn render_task_tree(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let block = panel_block("TODO", app.focus_area == FocusArea::TaskTree);
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

fn render_inspector(frame: &mut Frame, area: Rect, app: &mut TuiApp, focused: bool, narrow: bool) {
    let block = panel_block("Inspector", focused);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let content = inner;
    if content.width == 0 || content.height < 7 {
        return;
    }

    render_details_toolbar(frame, content, app, narrow);
}

fn render_detail_panel(frame: &mut Frame, area: Rect, app: &mut TuiApp, focused: bool) {
    let block = panel_block("Detail", focused);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    app.set_details_viewport(inner);
    let lines = if let Some(details) = app.details.as_ref() {
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
        let description = details
            .task
            .description
            .clone()
            .unwrap_or_else(|| "-".to_owned());

        let mut lines = vec![
            kv_line("Title", &details.task.title),
            Line::from(""),
            Line::from(Span::styled(
                "Description",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            )),
        ];
        lines.extend(description.lines().map(|line| Line::from(line.to_owned())));
        lines.extend([
            Line::from(""),
            Line::from(Span::styled(
                "Recent Logs",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            )),
        ]);
        lines.extend(logs);
        lines
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
        lines
    };

    let viewport_height = inner.height as usize;
    let max_scroll = lines.len().saturating_sub(viewport_height);
    app.details_scroll = app.details_scroll.min(max_scroll);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .scroll((app.details_scroll as u16, 0))
            .style(Style::default().fg(TEXT))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_details_toolbar(frame: &mut Frame, area: Rect, app: &mut TuiApp, narrow: bool) {
    let selected = app.details.as_ref().map(|details| &details.task);
    let selected_status = selected.map(|task| task.status);
    let selected_archived = selected.is_some_and(|task| task.archived);
    let can_mutate = app.can_mutate_viewed_space();
    let can_create = app.current_space().is_some() && can_mutate;
    let can_edit_task = selected.is_some() && can_mutate && !selected_archived;
    let can_restore = selected_archived && can_mutate;
    let can_purge = selected_archived && can_mutate;
    let can_status = selected.is_some() && can_mutate && !selected_archived;
    let show_arrange = app.current_sort == crate::domain::SortMode::Manual;
    let can_reorder = can_edit_task && show_arrange;

    if narrow {
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(if show_arrange { 1 } else { 0 }),
            Constraint::Length(if show_arrange { 1 } else { 0 }),
            Constraint::Min(0),
        ]);
        let [
            back_row,
            actions_header,
            actions_row,
            actions_gap,
            workflow_header,
            workflow_row,
            workflow_gap,
            arrange_header,
            arrange_row,
            _,
        ] = area.layout(&rows);

        render_link_buttons(
            frame,
            app,
            back_row,
            &[("Back", MouseTarget::CloseDetails, false, true, false)],
        );
        render_section_rule(frame, actions_header, "Actions");
        render_link_buttons(
            frame,
            app,
            actions_row,
            &[
                ("+ Task", MouseTarget::CreateTask, false, can_create, false),
                (
                    "+ Subtask",
                    MouseTarget::CreateSubtask,
                    false,
                    can_edit_task,
                    false,
                ),
                ("Edit", MouseTarget::EditTask, false, can_edit_task, false),
                ("Log", MouseTarget::AddLog, false, can_edit_task, false),
            ],
        );
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(PANEL_BG)),
            actions_gap,
        );

        render_section_rule(frame, workflow_header, "Workflow");
        render_workflow_buttons(
            frame,
            app,
            workflow_row,
            selected_archived,
            selected_status,
            can_restore,
            can_purge,
            can_status,
        );
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(PANEL_BG)),
            workflow_gap,
        );

        if show_arrange {
            render_section_rule(frame, arrange_header, "Arrange");
            render_arrange_buttons(frame, app, arrange_row, can_reorder);
        }
    } else {
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(if show_arrange { 1 } else { 0 }),
            Constraint::Length(if show_arrange { 1 } else { 0 }),
            Constraint::Min(0),
        ]);
        let [
            actions_header,
            actions_row,
            actions_gap,
            workflow_header,
            workflow_row,
            workflow_gap,
            arrange_header,
            arrange_row,
            _,
        ] = area.layout(&rows);

        render_section_rule(frame, actions_header, "Actions");
        render_link_buttons(
            frame,
            app,
            actions_row,
            &[
                ("+ Task", MouseTarget::CreateTask, false, can_create, false),
                (
                    "+ Subtask",
                    MouseTarget::CreateSubtask,
                    false,
                    can_edit_task,
                    false,
                ),
                ("Edit", MouseTarget::EditTask, false, can_edit_task, false),
                ("Log", MouseTarget::AddLog, false, can_edit_task, false),
            ],
        );
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(PANEL_BG)),
            actions_gap,
        );

        render_section_rule(frame, workflow_header, "Workflow");
        render_workflow_buttons(
            frame,
            app,
            workflow_row,
            selected_archived,
            selected_status,
            can_restore,
            can_purge,
            can_status,
        );
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(PANEL_BG)),
            workflow_gap,
        );

        if show_arrange {
            render_section_rule(frame, arrange_header, "Arrange");
            render_arrange_buttons(frame, app, arrange_row, can_reorder);
        }
    }
}

fn render_workflow_buttons(
    frame: &mut Frame,
    app: &mut TuiApp,
    area: Rect,
    selected_archived: bool,
    selected_status: Option<TaskStatus>,
    can_restore: bool,
    can_purge: bool,
    can_status: bool,
) {
    if selected_archived {
        render_link_buttons(
            frame,
            app,
            area,
            &[
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
    } else {
        render_link_buttons(
            frame,
            app,
            area,
            &[
                (
                    "Todo",
                    MouseTarget::SetTaskStatus(TaskStatus::Todo),
                    matches!(selected_status, Some(TaskStatus::Todo)),
                    can_status,
                    false,
                ),
                (
                    "Doing",
                    MouseTarget::SetTaskStatus(TaskStatus::InProgress),
                    matches!(selected_status, Some(TaskStatus::InProgress)),
                    can_status,
                    false,
                ),
                (
                    "Done",
                    MouseTarget::SetTaskStatus(TaskStatus::Done),
                    matches!(selected_status, Some(TaskStatus::Done)),
                    can_status,
                    false,
                ),
                (
                    "Close",
                    MouseTarget::SetTaskStatus(TaskStatus::Close),
                    matches!(selected_status, Some(TaskStatus::Close)),
                    can_status,
                    false,
                ),
            ],
        );
    }
}

fn render_arrange_buttons(frame: &mut Frame, app: &mut TuiApp, area: Rect, can_reorder: bool) {
    render_link_buttons(
        frame,
        app,
        area,
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
    let footer_text = app
        .status_message
        .clone()
        .unwrap_or_else(|| app.help_text());
    let prefix = "╰─ ";
    let help_label = "[Help]";
    let prefix_width = prefix.chars().count();
    let help_width = help_label.chars().count();
    let reserved_width = prefix_width + 1 + help_width + 2;
    let body_width = area.width as usize;
    let message = truncate_text(&footer_text, body_width.saturating_sub(reserved_width));
    let message_width = message.chars().count();
    let tail_width = body_width.saturating_sub(prefix_width + message_width + 1 + help_width);
    let tail = match tail_width {
        0 => String::new(),
        1 => "╯".to_owned(),
        _ => format!(" {}╯", "─".repeat(tail_width - 2)),
    };
    let help_target = MouseTarget::OpenHelp;
    let help_style = if matches!(&app.mode, Mode::Help) {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else if app.is_hovered(&help_target) {
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };
    let message_style = if app.status_message.is_some() {
        Style::default().fg(TEXT)
    } else {
        Style::default().fg(MUTED)
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(prefix, Style::default().fg(BORDER)),
            Span::styled(message.clone(), message_style),
            Span::raw(" "),
            Span::styled(help_label, help_style),
            Span::styled(tail, Style::default().fg(BORDER)),
        ]))
        .style(Style::default().bg(SCREEN_BG)),
        area,
    );

    let help_x = area.x + (prefix_width + message_width + 1) as u16;
    if help_x < area.right() {
        app.register_hitbox(
            Rect::new(
                help_x,
                area.y,
                help_width.min(area.width as usize) as u16,
                1,
            ),
            help_target,
        );
    }
}

fn render_space_form(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    form: &crate::tui::app::SpaceFormState,
) {
    let popup = centered_rect(area, 58, 28);
    frame.render_widget(Clear, popup);
    let block = rounded_block()
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
        Paragraph::new("Type in the field, then click Save or Cancel. Esc closes this dialog. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );
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
    let block = rounded_block()
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
        Paragraph::new("Click fields to edit, click a status chip, then click Save or Cancel. Esc closes this dialog. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );
}

fn render_log_form(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    form: &crate::tui::app::LogFormState,
) {
    let popup = centered_rect(area, 76, 66);
    frame.render_widget(Clear, popup);
    let block = rounded_block()
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
        Paragraph::new("Type in the message box, then click Save or Cancel. Esc closes this dialog. Ctrl+C quits the app.")
            .style(Style::default().fg(MUTED)),
        hint_area,
    );
}

fn render_purge_confirm(
    frame: &mut Frame,
    area: Rect,
    app: &mut TuiApp,
    confirm: &crate::tui::app::PurgeTaskConfirmState,
) {
    let popup = centered_rect(area, 62, 38);
    frame.render_widget(Clear, popup);
    let block = rounded_block()
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
        Paragraph::new("Click Cancel or Purge. Esc closes this dialog. Ctrl+C quits the app.")
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
    let block = rounded_block()
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
        Paragraph::new("Click Cancel or Purge. Esc closes this dialog. Ctrl+C quits the app.")
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
    let block = rounded_block()
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
            "Matches task title, description, logs, and ids. Click Apply, Clear, or Cancel. Esc closes this dialog.",
        )
        .style(Style::default().fg(MUTED)),
        hint_area,
    );
}

fn render_help_overlay(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let popup = centered_rect(area, 74, 64);
    frame.render_widget(Clear, popup);
    let block = rounded_block()
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
        Line::from("- Click the Space button in the top bar to open the manager popup."),
        Line::from(
            "- Click Filter to narrow the current task tree by title, description, logs, or ids.",
        ),
        Line::from("- Open from the manager switches context; archived spaces appear in All mode."),
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
    let block = rounded_block()
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
            (
                "Close",
                MouseTarget::TaskFormStatus(TaskStatus::Close),
                status == TaskStatus::Close,
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

fn render_link_buttons(
    frame: &mut Frame,
    app: &mut TuiApp,
    area: Rect,
    buttons: &[(&str, MouseTarget, bool, bool, bool)],
) {
    let mut spans = Vec::new();
    let mut x = area.x;

    for (index, (label, target, selected, enabled, danger)) in buttons
        .iter()
        .map(|(a, b, c, d, e)| (*a, b.clone(), *c, *d, *e))
        .enumerate()
    {
        if index > 0 {
            spans.push(Span::raw(" "));
            x = x.saturating_add(1);
        }

        let token = format!("[{label}]");
        let width = token.chars().count() as u16;
        if width == 0 || x.saturating_add(width) > area.right() {
            break;
        }

        let style = if !enabled {
            Style::default().fg(MUTED)
        } else if danger {
            Style::default().fg(DANGER).add_modifier(Modifier::BOLD)
        } else if selected {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else if app.is_hovered(&target) {
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        };

        spans.push(Span::styled(token, style));
        if enabled {
            app.register_hitbox(
                Rect::new(x, area.y, width.min(area.right().saturating_sub(x)), 1),
                target,
            );
        }
        x = x.saturating_add(width);
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(PANEL_BG)),
        area,
    );
}

fn render_section_rule(frame: &mut Frame, area: Rect, title: &str) {
    let title_text = format!(" {title} ");
    let title_width = title_text.chars().count();
    let total_width = area.width as usize;
    if total_width == 0 {
        return;
    }

    let remaining = total_width.saturating_sub(title_width);
    let left = remaining / 2;
    let right = remaining.saturating_sub(left);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("─".repeat(left), Style::default().fg(BORDER)),
            Span::styled(
                title_text,
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ),
            Span::styled("─".repeat(right), Style::default().fg(BORDER)),
        ]))
        .style(Style::default().bg(PANEL_BG)),
        area,
    );
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
    label: &'static str,
    input: &crate::tui::input::TextInput,
    focused: bool,
) {
    let block = rounded_block()
        .title(label)
        .border_style(border_style(focused))
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    let mut widget = input.widget(block, Style::default().bg(PANEL_BG).fg(TEXT));
    widget.set_cursor_line_style(if focused {
        Style::default().bg(SUBTLE_BG)
    } else {
        Style::default()
    });
    widget.set_cursor_style(if focused {
        Style::default().fg(Color::Black).bg(ACCENT)
    } else {
        Style::default().fg(TEXT).bg(PANEL_BG)
    });
    frame.render_widget(&widget, area);
}

fn render_text_area(
    frame: &mut Frame,
    area: Rect,
    label: &'static str,
    input: &crate::tui::input::TextInput,
    focused: bool,
) {
    let block = rounded_block()
        .title(label)
        .border_style(border_style(focused))
        .style(Style::default().bg(PANEL_BG).fg(TEXT));
    let mut widget = input.widget(block, Style::default().bg(PANEL_BG).fg(TEXT));
    widget.set_cursor_line_style(if focused {
        Style::default().bg(SUBTLE_BG)
    } else {
        Style::default()
    });
    widget.set_cursor_style(if focused {
        Style::default().fg(Color::Black).bg(ACCENT)
    } else {
        Style::default().fg(TEXT).bg(PANEL_BG)
    });
    frame.render_widget(&widget, area);
}

fn shell_block<'a>() -> Block<'a> {
    rounded_block()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(PANEL_BG).fg(TEXT))
}

fn header_block<'a>() -> Block<'a> {
    rounded_block()
        .title(Span::styled(
            " oh-my-todo ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(HEADER_BG).fg(TEXT))
}

fn detail_panel_height(total_height: u16) -> u16 {
    if total_height >= 26 {
        11
    } else if total_height >= 21 {
        9
    } else if total_height >= 17 {
        7
    } else {
        total_height.saturating_sub(5).max(5)
    }
}

fn truncate_text(value: &str, max_width: usize) -> String {
    if value.chars().count() <= max_width {
        return value.to_owned();
    }
    if max_width <= 3 {
        return value.chars().take(max_width).collect();
    }
    let mut shortened = value.chars().take(max_width - 3).collect::<String>();
    shortened.push_str("...");
    shortened
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    rounded_block()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused))
        .style(Style::default().bg(PANEL_BG).fg(TEXT))
}

fn rounded_block<'a>() -> Block<'a> {
    Block::bordered().border_type(BorderType::Rounded)
}

fn inset_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    let horizontal = horizontal.min(area.width / 2);
    let vertical = vertical.min(area.height / 2);
    Rect::new(
        area.x.saturating_add(horizontal),
        area.y.saturating_add(vertical),
        area.width.saturating_sub(horizontal.saturating_mul(2)),
        area.height.saturating_sub(vertical.saturating_mul(2)),
    )
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
        TaskStatus::Close => "[c]",
    }
}

fn status_color(status: TaskStatus) -> Color {
    match status {
        TaskStatus::Todo => TEXT,
        TaskStatus::InProgress => Color::Yellow,
        TaskStatus::Done => Color::Green,
        TaskStatus::Close => Color::Cyan,
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

fn format_timestamp(value: time::OffsetDateTime) -> String {
    value
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| value.to_string())
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
