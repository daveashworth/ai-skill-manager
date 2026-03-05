use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, ImportConfirm, Screen};

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
        Block::default()
            .style(Style::default().bg(Color::Black)),
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
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title(" Skill Manager "));
    f.render_widget(title, chunks[0]);

    // Skill list
    let items: Vec<ListItem> = app
        .unmanaged
        .iter()
        .map(|s| {
            let source = if s.is_symlink {
                format!(" (symlink → {})", s.symlink_target.as_ref().map(|p| p.display().to_string()).unwrap_or_default())
            } else {
                format!(" (dir: {})", s.source_path.display())
            };
            let desc = if s.meta.description.is_empty() {
                String::new()
            } else {
                let d = &s.meta.description;
                let truncated = if d.len() > 60 { format!("{}…", &d[..60]) } else { d.clone() };
                format!("  {}", truncated)
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::Cyan)),
                    Span::styled(&s.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(source, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(Span::styled(desc, Style::default().fg(Color::Gray))),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " {} skill(s) will be imported to central store ",
            app.unmanaged.len()
        )));
    f.render_widget(list, chunks[1]);

    // Description of what will happen
    let action_text = vec![
        Line::from(vec![
            Span::styled("  Skills will be copied to ", Style::default().fg(Color::Gray)),
            Span::styled("~/.config/skillmanager/skills/", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(Span::styled(
            "  Original files will be replaced with symlinks. All skills will be set to active.",
            Style::default().fg(Color::Gray),
        )),
    ];
    let action = Paragraph::new(action_text)
        .block(Block::default().borders(Borders::ALL).title(" What happens next "));
    f.render_widget(action, chunks[2]);

    // Confirm buttons
    let yes_style = if app.import_confirm == ImportConfirm::Yes {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(Color::Gray)
    };
    let no_style = if app.import_confirm == ImportConfirm::No {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(Color::Gray)
    };

    let confirm = Paragraph::new(Line::from(vec![
        Span::styled("  Import and manage these skills?   ", Style::default().fg(Color::White)),
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
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" Skill Manager", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  {} skills ({} active, {} inactive)", total, active_count, total - active_count),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(title, outer[0]);

    // Main content: skill list + details
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(outer[1]);

    draw_skill_list(f, app, content[0]);
    draw_skill_detail(f, app, content[1]);

    // Delete confirmation overlay
    if let Some(ref name) = app.delete_confirm {
        let dialog = centered_rect(50, 20, area);
        f.render_widget(ratatui::widgets::Clear, dialog);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(dialog);

        let text = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Delete ", Style::default().fg(Color::Red)),
                Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("?", Style::default().fg(Color::Red)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  This will remove it from the central store",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "  and all target directories.",
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Confirm Delete "),
        );
        f.render_widget(text, chunks[0]);

        let confirm = Paragraph::new(Line::from(vec![
            Span::styled("  [y]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" Delete  ", Style::default().fg(Color::Gray)),
            Span::styled("[n/Esc]", Style::default().fg(Color::Yellow)),
            Span::styled(" Cancel", Style::default().fg(Color::Gray)),
        ]));
        f.render_widget(confirm, chunks[1]);
    }

    // Footer
    let footer = if app.searching {
        Paragraph::new(Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Yellow)),
            Span::styled(&app.search_query, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
            Span::styled("  [Enter] done  [Esc] cancel", Style::default().fg(Color::DarkGray)),
        ]))
    } else {
        let mut spans = vec![
            Span::styled(" [Space]", Style::default().fg(Color::Yellow)),
            Span::styled(" Toggle  ", Style::default().fg(Color::Gray)),
            Span::styled("[/]", Style::default().fg(Color::Yellow)),
            Span::styled(" Search  ", Style::default().fg(Color::Gray)),
            Span::styled("[a]", Style::default().fg(Color::Yellow)),
            Span::styled(" Activate all  ", Style::default().fg(Color::Gray)),
            Span::styled("[d]", Style::default().fg(Color::Yellow)),
            Span::styled(" Deactivate all  ", Style::default().fg(Color::Gray)),
            Span::styled("[x]", Style::default().fg(Color::Yellow)),
            Span::styled(" Delete  ", Style::default().fg(Color::Gray)),
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::styled(" Quit", Style::default().fg(Color::Gray)),
        ];
        if !app.search_query.is_empty() {
            spans.push(Span::styled("  [Esc]", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(" Clear filter", Style::default().fg(Color::Gray)));
        }
        Paragraph::new(Line::from(spans))
    };
    f.render_widget(footer, outer[2]);
}

fn draw_skill_list(f: &mut Frame, app: &mut App, area: Rect) {
    let selected = app.selected;

    let items: Vec<ListItem> = app
        .filtered_skills()
        .iter()
        .enumerate()
        .map(|(i, (_, skill))| {
            let marker = if skill.active { "●" } else { "○" };
            let marker_color = if skill.active { Color::Green } else { Color::DarkGray };

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

            let line = Line::from(vec![
                Span::styled(format!(" {} ", marker), Style::default().fg(marker_color)),
                Span::styled(skill.meta.name.clone(), name_style),
                Span::styled(version, Style::default().fg(Color::DarkGray)),
            ]);

            if i == selected {
                ListItem::new(line).style(Style::default().bg(Color::Rgb(30, 30, 50)))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let title = if app.search_query.is_empty() {
        " Skills ".to_string()
    } else {
        format!(" Skills (filter: \"{}\") ", app.search_query)
    };

    let list = List::new(items)
        .highlight_symbol("")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
                .title(title),
        );
    f.render_stateful_widget(list, area, &mut app.list_state);
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
    let status_color = if skill.active { Color::Green } else { Color::DarkGray };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Status:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(&skill.meta.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
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
                format!  ("  {}", wrapped_line),
                Style::default().fg(Color::White),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Path:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            skill.store_path.display().to_string(),
            Style::default().fg(Color::Rgb(100, 100, 140)),
        ),
    ]));

    let detail = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
                .title(" Details "),
        );
    f.render_widget(detail, area);
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
