use crate::{
    context::{Context, Mode},
    utils::{get_key_name, get_scale_name, HELP},
    Cursor,
};
use parking_lot::{lock_api, RawMutex};
use std::{
    io::Stdout,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Clear, Padding, Paragraph, Row, Table},
    Terminal,
};

pub fn draw(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    cursor: &Cursor,
    mode: &mut Mode,
    should_redraw: &Arc<AtomicBool>,
    context_arc: &Arc<lock_api::Mutex<RawMutex, Context>>,
    show_popup: bool,
) {
    terminal
        .draw(|f| {
            let (grid, tempo, divisions) = {
                let context = context_arc.lock();
                (context.grid.clone(), context.tempo, context.divisions)
            };

            let port_name = {
                let context = context_arc.lock();
                if context.is_port(*cursor.cursor_row, *cursor.cursor_col) {
                    let name = context
                        .get_port_name(*cursor.cursor_row, *cursor.cursor_col)
                        .unwrap_or(&"".to_string())
                        .clone();
                    if name == "Global Scale" {
                        let scale_value = context.grid[*cursor.cursor_row][*cursor.cursor_col];
                        if let Some(scale_name) = get_scale_name(scale_value) {
                            format!("{}: {}", name, scale_name)
                        } else {
                            name
                        }
                    } else {
                        name
                    }
                } else {
                    "".to_string()
                }
            };

            let chunk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(10), Constraint::Max(3)].as_ref())
                .split(f.size());

            let rows = grid
                .iter()
                .enumerate()
                .map(|(r, row)| {
                    let cells = row
                        .iter()
                        .enumerate()
                        .map(|(c, &value)| {
                            let display_value = if value != '.' {
                                value
                            } else if r % 9 == 0 && c % 9 == 0 {
                                '+'
                            } else {
                                '.'
                            };

                            let cell = Cell::from(display_value.to_string());
                            let mut style = Style::default();

                            let is_selected = {
                                //let _context = context_arc.lock();
                                if let Mode::Select { start, end } = mode {
                                    let row_in_range = (r >= start.0 && r <= end.0)
                                        || (r <= start.0 && r >= end.0);
                                    let col_in_range = (c >= start.1 && c <= end.1)
                                        || (c <= start.1 && c >= end.1);
                                    row_in_range && col_in_range
                                } else {
                                    false
                                }
                            };

                            if is_selected {
                                style = style.bg(Color::DarkGray);
                            }

                            if *cursor.cursor_row == r && *cursor.cursor_col == c {
                                style = style.fg(Color::Yellow).add_modifier(Modifier::REVERSED);
                            } else {
                                let context = context_arc.lock();
                                if context.is_port(r, c) {
                                    match display_value {
                                        'E' | 'W' | 'N' | 'S' => {
                                            style =
                                                style.fg(Color::Cyan).add_modifier(Modifier::DIM);
                                        }
                                        '*' => {
                                            style = style
                                                .fg(Color::White)
                                                .add_modifier(Modifier::REVERSED);
                                        }
                                        _ => {
                                            style = style
                                                .fg(Color::Cyan)
                                                .add_modifier(Modifier::UNDERLINED)
                                        }
                                    }
                                } else {
                                    match display_value {
                                        'A'..='Z' => {
                                            style = style
                                                .fg(Color::Cyan)
                                                .add_modifier(Modifier::REVERSED)
                                        }
                                        '{' | '}' | '[' | ']' | '@' => {
                                            style = style
                                                .fg(Color::LightYellow)
                                                .add_modifier(Modifier::REVERSED)
                                        }
                                        '^' | '~' | ':' | ';' | '|' | '>' | '?' => {
                                            style = style
                                                .fg(Color::Cyan)
                                                .add_modifier(Modifier::REVERSED)
                                        }
                                        'a'..='z' | '0'..='9' => {
                                            style = style.fg(Color::DarkGray);
                                        }
                                        '.' => {
                                            style = style
                                                .fg(Color::DarkGray)
                                                .add_modifier(Modifier::DIM);
                                        }
                                        '+' => {
                                            style = style.fg(Color::LightCyan)
                                            //.add_modifier(Modifier::DIM);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            cell.style(style)
                        })
                        .collect::<Vec<_>>();
                    Row::new(cells)
                })
                .collect::<Vec<_>>();

            let constraints = {
                let context = context_arc.lock();
                vec![Constraint::Length(1); context.cols]
            };

            let table = Table::new(rows)
                .widths(&constraints)
                .column_spacing(0)
                .block(
                    Block::default()
                        .padding(Padding {
                            left: 3,
                            right: 3,
                            top: 1,
                            bottom: 1,
                        })
                        .border_type(BorderType::Rounded)
                        .border_style(
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::DIM),
                        )
                        .borders(Borders::ALL),
                );
            f.render_widget(table, chunk[0]);

            let statusline_text =
                status_line_text(context_arc, tempo, divisions, cursor, mode, port_name);
            let statusline = Paragraph::new(statusline_text)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .title("")
                        .borders(Borders::NONE)
                        .padding(Padding {
                            left: 3,
                            right: 3,
                            top: 0,
                            bottom: 0,
                        }),
                );
            f.render_widget(statusline, chunk[1]);

            let size = f.size();
            if show_popup {
                let block = Paragraph::new(HELP.trim().to_string())
                    .style(Style::default().fg(Color::Cyan))
                    .alignment(Alignment::Left)
                    .block(Block::default().borders(Borders::ALL));
                let area = help_rect(80, 80, size);
                f.render_widget(Clear, area);
                f.render_widget(block, area);
            }
        })
        .expect("Failed to draw TUI");

    should_redraw.store(false, Ordering::Relaxed);
}

fn status_line_text(
    context_arc: &Arc<lock_api::Mutex<RawMutex, Context>>,
    tempo: u64,
    divisions: u64,
    cursor: &Cursor<'_>,
    mode: &mut Mode,
    port_name: String,
) -> String {
    let context = context_arc.lock();
    format!(
        "{} bpm   {}/4   {},{}  {}  {}   {} {}   {} ",
        tempo,
        divisions,
        cursor.cursor_row,
        cursor.cursor_col,
        context.midi_port_name,
        match mode {
            Mode::Normal => "Insert".to_string(),
            Mode::Select { start: _, end: _ } => "Select".to_string(),
            Mode::Copy => "Copy".to_string(),
            Mode::Move => "Move".to_string(),
        },
        get_key_name(context.global_key).expect("Failed to get key name"),
        get_scale_name(context.global_scale).expect("Failed to get scale name"),
        &port_name
    )
}

fn help_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
                .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
                .as_ref(),
        )
        .split(popup_layout[1])[1]
}
