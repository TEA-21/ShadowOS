use crate::app::DiffApp;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render_diff_ui(frame: &mut Frame, app: &DiffApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.size());

    // File list
    let items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let style = if i == app.selected_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{:?} {}", f.change_type, f.relative_path.display())).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Modified Files ([Tab] Navigate)"),
    );
    frame.render_widget(list, chunks[0]);

    // Diff view
    let diff_paragraph = Paragraph::new(app.current_diff_preview.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Unified Git Diff ([P] Promote / [R] Rollback / [Q] Quit)"),
    );
    frame.render_widget(diff_paragraph, chunks[1]);
}
