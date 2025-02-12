use crate::{action::Action, app::color_scheme, commands};

use super::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use layout::Flex;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListState},
};

use color_eyre::Result;
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::TextArea;

#[derive(Default)]
pub struct CommandPalette {
    textarea: TextArea<'static>,
    active: bool,
    action_tx: Option<UnboundedSender<Action>>,
    completions: Vec<commands::Completion>,
    list_state: ListState,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl Component for CommandPalette {
    fn draw(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) -> Result<()> {
        if self.active {
            let [area] = Layout::horizontal([92]).flex(Flex::Center).areas(area);
            let [area] = Layout::vertical([24]).flex(Flex::Center).areas(area);
            let [text_area, completions] =
                Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);

            frame.render_widget(Clear, area);
            frame.render_widget(&self.textarea, text_area);
            frame.render_stateful_widget(
                List::new(self.completions.iter().map(|completion| {
                    Line::default().spans([
                        Span::styled(
                            format!(
                                "{}{}",
                                completion.val.clone(),
                                " ".repeat(32usize.saturating_sub(completion.val.len()))
                            ),
                            Style::new().fg(color_scheme::FG_SECONDARY),
                        ),
                        Span::styled(
                            completion.description.clone().unwrap_or_default(),
                            Style::new().fg(color_scheme::FG_MUTED),
                        ),
                    ])
                }))
                .highlight_style(
                    Style::new()
                        .fg(color_scheme::FG_SELECTION)
                        .bg(color_scheme::BG_SELECTION),
                )
                .block(Block::bordered().borders(Borders::LEFT | Borders::BOTTOM | Borders::RIGHT))
                .style(
                    Style::new()
                        .bg(color_scheme::BG_ELEVATED)
                        .fg(color_scheme::FG_SECONDARY),
                ),
                completions,
                &mut self.list_state,
            );
        }
        Ok(())
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            match key {
                KeyEvent {
                    code: KeyCode::Enter,
                    ..
                } => {
                    self.active = false;

                    if let Some(action) = commands::parse_command(&self.textarea.lines()[0]) {
                        if let Some(tx) = &self.action_tx {
                            let _ = tx.send(action);
                        }
                    }

                    Ok(Some(Action::CloseCommandPalette))
                }
                KeyEvent {
                    code: KeyCode::Esc, ..
                } => {
                    self.active = false;

                    Ok(Some(Action::CloseCommandPalette))
                }
                KeyEvent {
                    code: KeyCode::Tab, ..
                } => {
                    self.textarea.move_cursor(tui_textarea::CursorMove::End);
                    self.textarea.delete_word();
                    if let Some(completion) = self
                        .list_state
                        .selected()
                        .and_then(|i| self.completions.get(i))
                    {
                        self.textarea.delete_line_by_head();
                        self.textarea.insert_str(&completion.full);
                        self.completions = commands::get_completions(&self.textarea.lines()[0]);
                        self.list_state.select_first();
                    }
                    Ok(None)
                }
                KeyEvent {
                    code: KeyCode::Left,
                    modifiers: KeyModifiers::ALT,
                    ..
                } => {
                    self.textarea
                        .move_cursor(tui_textarea::CursorMove::WordBack);
                    Ok(None)
                }
                KeyEvent {
                    code: KeyCode::Right,
                    modifiers: KeyModifiers::ALT,
                    ..
                } => {
                    self.textarea
                        .move_cursor(tui_textarea::CursorMove::WordForward);
                    Ok(None)
                }
                KeyEvent {
                    code: KeyCode::Up, ..
                } => {
                    self.list_state.select_previous();
                    Ok(None)
                }
                KeyEvent {
                    code: KeyCode::Down,
                    ..
                } => {
                    self.list_state.select_next();
                    Ok(None)
                }
                _ => {
                    if self.textarea.input_without_shortcuts(key) {
                        self.completions = commands::get_completions(&self.textarea.lines()[0]);
                        self.list_state.select_first();
                    }
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenCommandPalette => {
                self.textarea = TextArea::default();
                self.textarea
                    .set_style(Style::new().fg(color_scheme::FG_SECONDARY));
                self.textarea
                    .set_placeholder_style(Style::new().fg(color_scheme::FG_MUTED));
                self.textarea.set_placeholder_text(
                    "Enter a command, use the arrow keys and <tab> to select completions",
                );
                self.textarea
                    .set_block(Block::bordered().style(Style::new().bg(color_scheme::BG_ELEVATED)));

                self.list_state.select_first();

                self.active = true;
                self.completions = commands::get_completions("");
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
