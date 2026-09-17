use crate::app::{ActivePane, DiffApp, LineType};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render_diff_ui(frame: &mut Frame, app: &DiffApp) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(frame.size());

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_layout[0]);

    // -------------------------------------------------------------------------
    // Left Pane: Modified Files List
    // -------------------------------------------------------------------------
    let file_border_style = if app.active_pane == ActivePane::FileList {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let file_items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let stage_mark = if f.is_staged { "[x] " } else { "[ ] " };
            let is_selected = i == app.selected_file_index;

            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if f.is_staged {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            let line_text = format!("{}{:?} {}", stage_mark, f.change_type, f.relative_path.display());
            ListItem::new(line_text).style(style)
        })
        .collect();

    let file_list = List::new(file_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(file_border_style)
            .title(" Ephemeral Files ([Tab] Focus) "),
    );
    frame.render_widget(file_list, content_chunks[0]);

    // -------------------------------------------------------------------------
    // Right Pane: Unified Git Diff & Hunk Inspector
    // Plain-text: Green for Additions (+), Red for Deletions (-)
    // -------------------------------------------------------------------------
    let diff_border_style = if app.active_pane == ActivePane::HunkList {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut diff_spans: Vec<Line> = Vec::new();

    if let Some(current_file) = app.current_file() {
        for (hunk_idx, hunk) in current_file.hunks.iter().enumerate() {
            let is_selected_hunk = hunk_idx == app.selected_hunk_index;
            let hunk_stage = if hunk.is_staged { "[x] " } else { "[ ] " };

            let hunk_header_style = if is_selected_hunk && app.active_pane == ActivePane::HunkList {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };

            diff_spans.push(Line::from(vec![
                Span::styled(format!("Hunk #{}: {}{}", hunk.id, hunk_stage, hunk.header), hunk_header_style),
            ]));

            for line in &hunk.lines {
                match line.line_type {
                    LineType::Addition => {
                        diff_spans.push(Line::from(vec![
                            Span::styled(line.content.clone(), Style::default().fg(Color::Green)),
                        ]));
                    }
                    LineType::Deletion => {
                        diff_spans.push(Line::from(vec![
                            Span::styled(line.content.clone(), Style::default().fg(Color::Red)),
                        ]));
                    }
                    LineType::Header => {
                        diff_spans.push(Line::from(vec![
                            Span::styled(line.content.clone(), Style::default().fg(Color::Cyan)),
                        ]));
                    }
                    LineType::Context => {
                        diff_spans.push(Line::from(vec![
                            Span::styled(line.content.clone(), Style::default().fg(Color::White)),
                        ]));
                    }
                }
            }
            diff_spans.push(Line::from(vec![Span::raw("")]));
        }
    } else {
        diff_spans.push(Line::from(vec![
            Span::styled("No modified files in ephemeral upperdir.", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let diff_paragraph = Paragraph::new(diff_spans)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(diff_border_style)
                .title(" Unified Diff Inspector ([Space] Stage Hunk) "),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(diff_paragraph, content_chunks[1]);

    // -------------------------------------------------------------------------
    // Bottom Bar: Status Message and Interactive Hotkey Controls
    // -------------------------------------------------------------------------
    let footer_text = vec![
        Line::from(vec![
            Span::styled(" [Status] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&app.status_message),
            Span::raw(" | "),
            Span::styled("[Space] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Stage "),
            Span::styled("[P] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("Promote "),
            Span::styled("[R] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("Rollback "),
            Span::styled("[Q] ", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            Span::raw("Quit"),
        ]),
    ];

    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, main_layout[1]);
}
