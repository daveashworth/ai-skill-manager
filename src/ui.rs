use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, DeleteTarget, Focus, GroupNameMode, GroupStatus, ImportConfirm, Screen};

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Import => draw_import(f, app),
        Screen::Main => draw_main(f, app),
    }
}

fn draw_import(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Center the dialog
    let dialog = centered_rect(70, 70, area);

    // Clear background
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(dialog);

    // Title
    let title = Paragraph::new("  Unmanaged Skills Found")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Skill Manager "),
        );
    f.render_widget(title, chunks[0]);

    // Skill list
    let items: Vec<ListItem> = app
        .unmanaged
        .iter()
        .map(|s| {
            let source = if s.is_symlink {
                format!(
                    " (symlink → {})",
                    s.symlink_target
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                )
            } else {
                format!(" (dir: {})", s.source_path.display())
            };
            let desc = if s.meta.description.is_empty() {
                String::new()
            } else {
                let d = &s.meta.description;
                let truncated = if d.len() > 60 {
                    format!("{}…", &d[..60])
                } else {
                    d.clone()
                };
                format!("  {}", truncated)
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        &s.name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(source, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(Span::styled(desc, Style::default().fg(Color::Gray))),
            ])
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
        " {} skill(s) will be imported to central store ",
        app.unmanaged.len()
    )));
    f.render_widget(list, chunks[1]);

    // Description of what will happen
    let action_text =
        vec![
        Line::from(vec![
            Span::styled("  Skills will be copied to ", Style::default().fg(Color::Gray)),
            Span::styled("~/.config/skillmanager/skills/", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(Span::styled(
            "  Original files will be replaced with symlinks. All skills will be set to active.",
            Style::default().fg(Color::Gray),
        )),
    ];
    let action = Paragraph::new(action_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" What happens next "),
    );
    f.render_widget(action, chunks[2]);

    // Confirm buttons
    let yes_style = if app.import_confirm == ImportConfirm::Yes {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(Color::Gray)
    };
    let no_style = if app.import_confirm == ImportConfirm::No {
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(Color::Gray)
    };

    let confirm = Paragraph::new(Line::from(vec![
        Span::styled(
            "  Import and manage these skills?   ",
            Style::default().fg(Color::White),
        ),
        Span::styled(" Yes ", yes_style),
        Span::raw("  "),
        Span::styled(" No ", no_style),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(confirm, chunks[3]);
}

fn draw_main(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    // Title bar
    let total = app.skills.len();
    let active_count = app.skills.iter().filter(|s| s.active).count();
    let group_count = app.config.groups.len();
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " Skill Manager",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  {} skills ({} active, {} inactive)  {} groups",
                total,
                active_count,
                total - active_count,
                group_count
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(title, outer[0]);

    // Main content: skill list + details
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(outer[1]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(content[1]);

    draw_skill_list(f, app, content[0]);
    draw_detail(f, app, right[0]);
    draw_group_list(f, app, right[1]);

    if app.group_editor.is_some() {
        draw_group_editor_dialog(f, app, area);
    } else if app.group_name_input.is_some() {
        draw_group_name_dialog(f, app, area);
    } else if app.delete_confirm.is_some() {
        draw_delete_dialog(f, app, area);
    }

    // Footer
    let footer = if app.searching {
        Paragraph::new(Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Yellow)),
            Span::styled(&app.search_query, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
            Span::styled(
                "  [Enter] done  [Esc] cancel",
                Style::default().fg(Color::DarkGray),
            ),
        ]))
    } else {
        let mut spans = vec![
            Span::styled(" [Space]", Style::default().fg(Color::Yellow)),
            Span::styled(" Toggle  ", Style::default().fg(Color::Gray)),
            Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
            Span::styled(" Focus  ", Style::default().fg(Color::Gray)),
            Span::styled("[/]", Style::default().fg(Color::Yellow)),
            Span::styled(" Search  ", Style::default().fg(Color::Gray)),
            Span::styled("[a]", Style::default().fg(Color::Yellow)),
            Span::styled(" Activate all  ", Style::default().fg(Color::Gray)),
            Span::styled("[d]", Style::default().fg(Color::Yellow)),
            Span::styled(" Deactivate all  ", Style::default().fg(Color::Gray)),
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::styled(" Quit", Style::default().fg(Color::Gray)),
        ];
        if app.focus == Focus::Groups {
            spans.push(Span::styled("  [n]", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(" New  ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled("[e]", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(" Edit  ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled("[r]", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(" Rename  ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled("[x]", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(
                " Delete group",
                Style::default().fg(Color::Gray),
            ));
        } else {
            spans.push(Span::styled("  [x]", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(
                " Delete skill",
                Style::default().fg(Color::Gray),
            ));
        }
        if !app.search_query.is_empty() {
            spans.push(Span::styled("  [Esc]", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(
                " Clear filter",
                Style::default().fg(Color::Gray),
            ));
        }
        Paragraph::new(Line::from(spans))
    };
    f.render_widget(footer, outer[2]);
}

fn draw_skill_list(f: &mut Frame, app: &mut App, area: Rect) {
    let selected = app.selected;
    let border_style = if app.focus == Focus::Skills {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Rgb(60, 60, 80))
    };

    let items: Vec<ListItem> = app
        .filtered_skills()
        .iter()
        .enumerate()
        .map(|(i, (_, skill))| {
            let marker = if skill.active { "●" } else { "○" };
            let marker_color = if skill.active {
                Color::Green
            } else {
                Color::DarkGray
            };

            let name_style = if i == selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if skill.active {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let version = if skill.meta.version.is_empty() {
                String::new()
            } else {
                format!(" {}", skill.meta.version)
            };

            let groups = app.groups_for_skill(&skill.key);

            let key_style = if i == selected {
                Style::default().fg(Color::Rgb(170, 170, 210))
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let mut title_spans = vec![
                Span::styled(format!(" {} ", marker), Style::default().fg(marker_color)),
                Span::styled(skill.meta.name.clone(), name_style),
                Span::styled(version, Style::default().fg(Color::DarkGray)),
            ];
            for group in groups {
                title_spans.push(Span::raw(" "));
                title_spans.push(Span::styled(
                    format!("[{}]", group),
                    Style::default()
                        .fg(color_for_group(group))
                        .add_modifier(Modifier::BOLD),
                ));
            }

            let lines = vec![
                Line::from(title_spans),
                Line::from(vec![
                    Span::styled("   key: ", key_style),
                    Span::styled(skill.key.clone(), key_style),
                ]),
            ];

            if i == selected {
                ListItem::new(lines).style(Style::default().bg(Color::Rgb(30, 30, 50)))
            } else {
                ListItem::new(lines)
            }
        })
        .collect();

    let title = if app.search_query.is_empty() {
        " Skills ".to_string()
    } else {
        format!(" Skills (filter: \"{}\") ", app.search_query)
    };

    let list = List::new(items).highlight_symbol("").block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    if app.focus == Focus::Groups {
        draw_group_detail(f, app, area);
    } else {
        draw_skill_detail(f, app, area);
    }
}

fn draw_skill_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(skill) = app.selected_skill() else {
        let empty = Paragraph::new("  No skill selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
                    .title(" Details "),
            );
        f.render_widget(empty, area);
        return;
    };

    let status = if skill.active { "Active" } else { "Inactive" };
    let status_color = if skill.active {
        Color::Green
    } else {
        Color::DarkGray
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Status:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                status,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &skill.meta.name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Key:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(&skill.key, Style::default().fg(Color::Rgb(170, 170, 210))),
        ]),
    ];

    if !skill.meta.version.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Version: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&skill.meta.version, Style::default().fg(Color::White)),
        ]));
    }

    if !skill.meta.author.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Author:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&skill.meta.author, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));

    if !skill.meta.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Description:",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Word-wrap the description
        let max_width = area.width.saturating_sub(6) as usize;
        for wrapped_line in word_wrap(&skill.meta.description, max_width) {
            lines.push(Line::from(Span::styled(
                format!("  {}", wrapped_line),
                Style::default().fg(Color::White),
            )));
        }
    }

    lines.push(Line::from(""));

    let groups = app.groups_for_skill(&skill.key);
    if !groups.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Groups:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(groups.join(", "), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("  Path:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            skill.store_path.display().to_string(),
            Style::default().fg(Color::Rgb(100, 100, 140)),
        ),
    ]));

    let detail = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
            .title(" Details "),
    );
    f.render_widget(detail, area);
}

fn draw_group_list(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.focus == Focus::Groups {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Rgb(60, 60, 80))
    };

    if app.config.groups.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(Span::styled(
                "  No groups configured",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [n] to create a group here.",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "  Membership is stored with canonical skill keys.",
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Groups "),
        );
        f.render_widget(empty, area);
        return;
    }

    let selected = app.group_selected;
    let items: Vec<ListItem> = app
        .group_entries()
        .iter()
        .enumerate()
        .map(|(i, (name, members))| {
            let (marker, marker_color) = match app.group_status(name) {
                GroupStatus::Active => ("●", Color::Green),
                GroupStatus::Inactive => ("○", Color::DarkGray),
                GroupStatus::Mixed => ("◐", Color::Yellow),
                GroupStatus::Empty => ("◌", Color::DarkGray),
            };
            let (active_count, managed_count, configured_count) = app.group_counts(name);
            let detail = if managed_count == configured_count {
                format!("   {} active / {} skills", active_count, managed_count)
            } else {
                format!(
                    "   {} active / {} managed / {} configured",
                    active_count, managed_count, configured_count
                )
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", marker), Style::default().fg(marker_color)),
                    Span::styled(
                        (*name).clone(),
                        if i == selected {
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        },
                    ),
                ]),
                Line::from(Span::styled(
                    detail,
                    if i == selected {
                        Style::default().fg(Color::Rgb(170, 170, 210))
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                )),
                Line::from(Span::styled(
                    format!("   keys: {}", members.join(", ")),
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            if i == selected {
                ListItem::new(lines).style(Style::default().bg(Color::Rgb(30, 30, 50)))
            } else {
                ListItem::new(lines)
            }
        })
        .collect();

    let list = List::new(items).highlight_symbol("").block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Groups "),
    );
    f.render_stateful_widget(list, area, &mut app.group_list_state);
}

fn draw_group_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some((group_name, member_keys)) = app.selected_group() else {
        let empty = Paragraph::new("  No group selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Group Details "),
            );
        f.render_widget(empty, area);
        return;
    };

    let (status, status_color) = match app.group_status(group_name) {
        GroupStatus::Active => ("Active", Color::Green),
        GroupStatus::Inactive => ("Inactive", Color::DarkGray),
        GroupStatus::Mixed => ("Mixed", Color::Yellow),
        GroupStatus::Empty => ("Empty", Color::DarkGray),
    };
    let managed_members = app.group_member_skills(group_name);
    let missing_count = member_keys.len().saturating_sub(managed_members.len());

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Status:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                status,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Group:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                group_name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Keys:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                member_keys.join(", "),
                Style::default().fg(Color::Rgb(170, 170, 210)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Managed members:",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    if managed_members.is_empty() {
        lines.push(Line::from(Span::styled(
            "  None of this group's keys match a managed skill yet.",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for skill in managed_members {
            let label = if skill.meta.name == skill.key {
                skill.meta.name.clone()
            } else {
                format!("{} ({})", skill.meta.name, skill.key)
            };
            lines.push(Line::from(Span::styled(
                format!("  • {}", label),
                Style::default().fg(Color::White),
            )));
        }
    }

    if missing_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  {} configured key(s) are not currently managed.",
                missing_count
            ),
            Style::default().fg(Color::Yellow),
        )));
    }

    let detail = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Group Details "),
    );
    f.render_widget(detail, area);
}

fn draw_delete_dialog(f: &mut Frame, app: &App, area: Rect) {
    let Some(target) = &app.delete_confirm else {
        return;
    };

    let dialog = centered_rect(50, 20, area);
    f.render_widget(ratatui::widgets::Clear, dialog);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(dialog);

    let (subject, details) = match target {
        DeleteTarget::Skill(name) => {
            let display_name = app
                .skills
                .iter()
                .find(|skill| skill.key == *name)
                .map(|skill| skill.meta.name.clone())
                .unwrap_or_else(|| name.clone());

            (
                display_name,
                vec![
                    "  This will remove it from the central store".to_string(),
                    "  and all target directories.".to_string(),
                ],
            )
        }
        DeleteTarget::Group(name) => (
            name.clone(),
            vec![
                "  This only removes the group definition.".to_string(),
                "  It will not delete or deactivate any skills.".to_string(),
            ],
        ),
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Delete ", Style::default().fg(Color::Red)),
            Span::styled(
                subject,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(Color::Red)),
        ]),
        Line::from(""),
    ];

    lines.extend(
        details
            .into_iter()
            .map(|detail| Line::from(Span::styled(detail, Style::default().fg(Color::Gray)))),
    );

    let text = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" Confirm Delete "),
    );
    f.render_widget(text, chunks[0]);

    let confirm = Paragraph::new(Line::from(vec![
        Span::styled(
            "  [y]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Delete  ", Style::default().fg(Color::Gray)),
        Span::styled("[n/Esc]", Style::default().fg(Color::Yellow)),
        Span::styled(" Cancel", Style::default().fg(Color::Gray)),
    ]));
    f.render_widget(confirm, chunks[1]);
}

fn draw_group_name_dialog(f: &mut Frame, app: &App, area: Rect) {
    let Some(dialog_state) = &app.group_name_input else {
        return;
    };

    let dialog = centered_rect(50, 24, area);
    f.render_widget(ratatui::widgets::Clear, dialog);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(dialog);

    let title = match dialog_state.mode {
        GroupNameMode::Create => " Create Group ",
        GroupNameMode::Rename { .. } => " Rename Group ",
    };

    let prompt = match dialog_state.mode {
        GroupNameMode::Create => "  New group name:",
        GroupNameMode::Rename { .. } => "  Group name:",
    };

    let header = Paragraph::new(prompt).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(title),
    );
    f.render_widget(header, chunks[0]);

    let value = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default().fg(Color::White)),
        Span::styled(&dialog_state.value, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Yellow)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 80))),
    );
    f.render_widget(value, chunks[1]);

    let error = dialog_state.error.as_deref().unwrap_or("  ");
    let error_style = if dialog_state.error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let error = Paragraph::new(error).style(error_style);
    f.render_widget(error, chunks[2]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  [Enter]", Style::default().fg(Color::Yellow)),
        Span::styled(" Save  ", Style::default().fg(Color::Gray)),
        Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
        Span::styled(" Cancel", Style::default().fg(Color::Gray)),
    ]));
    f.render_widget(footer, chunks[3]);
}

fn draw_group_editor_dialog(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(editor) = &mut app.group_editor else {
        return;
    };

    let dialog = centered_rect(78, 80, area);
    f.render_widget(ratatui::widgets::Clear, dialog);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(dialog);

    let selected_count = editor.members.len();
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  Editing group: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &editor.group_name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {} selected member(s)", selected_count),
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Edit Group Members "),
    );
    f.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = if editor.entries.is_empty() {
        vec![ListItem::new(vec![Line::from(Span::styled(
            "  No managed skills found yet.",
            Style::default().fg(Color::DarkGray),
        ))])]
    } else {
        editor
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let member = editor.members.contains(&entry.key);
                let checkbox = if member { "[x]" } else { "[ ]" };
                let checkbox_style = if member {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let name = if entry.missing {
                    format!("{} (missing)", entry.name)
                } else {
                    entry.name.clone()
                };
                let status = if entry.missing {
                    Style::default().fg(Color::Yellow)
                } else if entry.active {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let lines = vec![
                    Line::from(vec![
                        Span::styled(format!(" {} ", checkbox), checkbox_style),
                        Span::styled(
                            name,
                            if i == editor.selected {
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            },
                        ),
                        Span::styled(
                            if entry.missing {
                                " missing"
                            } else if entry.active {
                                " active"
                            } else {
                                " inactive"
                            },
                            status,
                        ),
                    ]),
                    Line::from(Span::styled(
                        format!("   key: {}", entry.key),
                        if i == editor.selected {
                            Style::default().fg(Color::Rgb(170, 170, 210))
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    )),
                ];

                if i == editor.selected {
                    ListItem::new(lines).style(Style::default().bg(Color::Rgb(30, 30, 50)))
                } else {
                    ListItem::new(lines)
                }
            })
            .collect()
    };

    let list = List::new(items).highlight_symbol("").block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
            .title(" Managed Skills "),
    );
    f.render_stateful_widget(list, chunks[1], &mut editor.list_state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  [Space]", Style::default().fg(Color::Yellow)),
        Span::styled(" Toggle member  ", Style::default().fg(Color::Gray)),
        Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
        Span::styled(" Save  ", Style::default().fg(Color::Gray)),
        Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
        Span::styled(" Cancel", Style::default().fg(Color::Gray)),
    ]));
    f.render_widget(footer, chunks[2]);
}

fn color_for_group(group: &str) -> Color {
    const PALETTE: [(u8, u8, u8); 8] = [
        (84, 176, 142),
        (102, 132, 220),
        (211, 142, 92),
        (171, 119, 214),
        (78, 180, 201),
        (201, 167, 76),
        (204, 110, 147),
        (133, 186, 96),
    ];

    let mut hasher = DefaultHasher::new();
    group.hash(&mut hasher);
    let index = (hasher.finish() as usize) % PALETTE.len();
    let (r, g, b) = PALETTE[index];
    Color::Rgb(r, g, b)
}

fn word_wrap(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
