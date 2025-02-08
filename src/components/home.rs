use std::{collections::VecDeque, ops::Not};

use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use layout::{Flex, Offset};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, app::color_scheme, config::Config, drawing::Element};

#[derive(Default)]
struct DrawingCanvas {
    pub elements: VecDeque<Element>,
}

#[derive(PartialEq, Eq)]
enum Tool {
    Cursor,
    Box,
    Line,
    Text,
}

const LIST_WIDTH: u16 = 14;

impl Default for Tool {
    fn default() -> Self {
        Self::Cursor
    }
}

#[derive(Clone, Copy)]
pub enum Operation {
    Selection {
        origin: Position,
        second: Position,
    },
    Move {
        origin: Position,
        second: Position,
    },
    Resize {
        direction: Direction,
        origin: Position,
        second: Position,
    },
}

impl Operation {
    pub fn apply_transform(&self, area: &Rect) -> Rect {
        match self {
            Operation::Move { origin, second } => area.offset(Offset {
                x: second.x as i32 - origin.x as i32,
                y: second.y as i32 - origin.y as i32,
            }),
            Operation::Resize {
                direction,
                origin,
                second,
            } => match direction {
                Direction::TopLeft => Rect {
                    x: (area.x as i32
                        + (second.x as i32 - origin.x as i32).min(area.width as i32 - 2))
                        as u16,
                    y: (area.y as i32
                        + (second.y as i32 - origin.y as i32).min(area.height as i32 - 2))
                        as u16,
                    width: (area.width as i32 - (second.x as i32 - origin.x as i32)).max(2) as u16,
                    height: (area.height as i32 - (second.y as i32 - origin.y as i32)).max(2)
                        as u16,
                },
                Direction::TopRight => Rect {
                    x: area.x,
                    y: (area.y as i32
                        + (second.y as i32 - origin.y as i32).min(area.height as i32 - 2))
                        as u16,
                    width: (area.width as i32 + (second.x as i32 - origin.x as i32)).max(2) as u16,
                    height: (area.height as i32 - (second.y as i32 - origin.y as i32)).max(2)
                        as u16,
                },
                Direction::BottomLeft => Rect {
                    x: (area.x as i32
                        + (second.x as i32 - origin.x as i32).min(area.width as i32 - 2))
                        as u16,
                    y: area.y,
                    width: (area.width as i32 - (second.x as i32 - origin.x as i32)).max(2) as u16,
                    height: (area.height as i32 + (second.y as i32 - origin.y as i32)).max(2)
                        as u16,
                },
                Direction::BottomRight => Rect {
                    x: area.x,
                    y: area.y,
                    width: (area.width as i32 + (second.x as i32 - origin.x as i32)).max(2) as u16,
                    height: (area.height as i32 + (second.y as i32 - origin.y as i32)).max(2)
                        as u16,
                },
            },
            _ => area.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    current_tool: Tool,
    current_operation: Option<Operation>,
    selected_elements: Vec<usize>,
    canvas: DrawingCanvas,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    fn update_tool(&mut self, tool: Tool) {
        if tool == self.current_tool {
            return;
        }

        if self.current_operation.is_some() {
            self.current_operation = None;
        }

        self.current_tool = tool;
    }

    fn reset_tool(&mut self) {
        self.update_tool(Tool::Cursor);
        self.selected_elements.clear();
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key {
            KeyEvent {
                code,
                modifiers: _,
                kind: _,
                state: _,
            } => match code {
                crossterm::event::KeyCode::Char('v') => self.update_tool(Tool::Cursor),
                crossterm::event::KeyCode::Char('b') => self.update_tool(Tool::Box),
                crossterm::event::KeyCode::Char('l') => self.update_tool(Tool::Line),
                crossterm::event::KeyCode::Char('i') => self.update_tool(Tool::Text),
                crossterm::event::KeyCode::Char('a') => {
                    self.update_tool(Tool::Cursor);
                    self.selected_elements = (0..self.canvas.elements.len()).collect::<Vec<_>>()
                }
                crossterm::event::KeyCode::Char('d') | crossterm::event::KeyCode::Delete => {
                    self.canvas.elements = self
                        .canvas
                        .elements
                        .iter()
                        .enumerate()
                        .filter_map(|(i, el)| {
                            self.selected_elements
                                .contains(&i)
                                .not()
                                .then_some(el.clone())
                        })
                        .collect::<Vec<_>>()
                        .into();
                    self.selected_elements.clear();
                }
                _ => {}
            },
        };

        Ok(None)
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        match mouse {
            MouseEvent {
                kind,
                column,
                row,
                modifiers: _,
            } => {
                if column < LIST_WIDTH {
                    Ok(None)
                } else {
                    match kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let column = column - LIST_WIDTH;
                            match self.current_tool {
                                Tool::Cursor => {
                                    if self
                                        .selected_elements
                                        .iter()
                                        .map(|i| &self.canvas.elements[*i])
                                        .filter(|el| {
                                            el.area().contains(Position { x: column, y: row })
                                        })
                                        .next()
                                        .is_some()
                                    {
                                        self.current_operation = Some(Operation::Move {
                                            origin: Position { x: column, y: row },
                                            second: Position { x: column, y: row },
                                        })
                                    } else {
                                        if self.selected_elements.len() == 1 {
                                            if let Some(direction) = self
                                                .selected_elements
                                                .iter()
                                                .filter_map(|i| {
                                                    let area = self.canvas.elements[*i].area();

                                                    if area.x.saturating_sub(1) == column
                                                        && area.y.saturating_sub(1) == row
                                                    {
                                                        Some(Direction::TopLeft)
                                                    } else if (area.x + area.width) == column
                                                        && area.y.saturating_sub(1) == row
                                                    {
                                                        Some(Direction::TopRight)
                                                    } else if area.x.saturating_sub(1) == column
                                                        && (area.y + area.height) == row
                                                    {
                                                        Some(Direction::BottomLeft)
                                                    } else if (area.x + area.width) == column
                                                        && (area.y + area.height) == row
                                                    {
                                                        Some(Direction::BottomRight)
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .next()
                                            {
                                                self.current_operation = Some(Operation::Resize {
                                                    direction,
                                                    origin: Position { x: column, y: row },
                                                    second: Position { x: column, y: row },
                                                });
                                                return Ok(None);
                                            }
                                        }
                                        self.current_operation = Some(Operation::Selection {
                                            origin: Position { x: column, y: row },
                                            second: Position { x: column, y: row },
                                        });

                                        self.selected_elements.clear();

                                        if let Some(x) = self
                                            .canvas
                                            .elements
                                            .iter()
                                            .enumerate()
                                            .rev()
                                            .filter_map(|(i, el)| {
                                                el.area()
                                                    .contains(Position { x: column, y: row })
                                                    .then_some(i)
                                            })
                                            .next()
                                        {
                                            self.selected_elements.push(x);
                                        }
                                    }
                                }
                                Tool::Box => {
                                    self.current_operation = Some(Operation::Selection {
                                        origin: (column, row).into(),
                                        second: (column, row).into(),
                                    });
                                }
                                _ => {}
                            }
                            Ok(None)
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            match self.current_tool {
                                Tool::Box => match self.current_operation {
                                    Some(Operation::Selection { origin, second }) => {
                                        let area = Rect {
                                            x: origin.x.min(second.x),
                                            y: origin.y.min(second.y),
                                            width: origin.x.abs_diff(second.x) + 1,
                                            height: origin.y.abs_diff(second.y) + 1,
                                        };

                                        if area.width > 1 && area.height > 1 {
                                            self.canvas.elements.push_back(Element::Box { area });
                                            self.reset_tool();
                                            self.selected_elements
                                                .push(self.canvas.elements.len() - 1);
                                        }
                                    }
                                    _ => {}
                                },
                                Tool::Cursor => {
                                    if let Some(op) = self.current_operation {
                                        for i in &self.selected_elements {
                                            self.canvas.elements[*i]
                                                .transform(|area| op.apply_transform(area));
                                        }
                                    }
                                }
                                _ => {}
                            }
                            self.current_operation = None;
                            Ok(None)
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            let column = column - LIST_WIDTH;
                            match self.current_tool {
                                Tool::Box => {
                                    if let Some(ref mut operation) = &mut self.current_operation {
                                        match operation {
                                            Operation::Selection { origin: _, second } => {
                                                second.x = column;
                                                second.y = row;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Tool::Cursor => match &mut self.current_operation {
                                    Some(Operation::Selection { origin, second }) => {
                                        second.x = column;
                                        second.y = row;

                                        let area = Rect {
                                            x: origin.x.min(second.x),
                                            y: origin.y.min(second.y),
                                            width: origin.x.abs_diff(second.x) + 1,
                                            height: origin.y.abs_diff(second.y) + 1,
                                        };

                                        self.selected_elements = self
                                            .canvas
                                            .elements
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(i, el)| {
                                                el.area().intersects(area).then_some(i)
                                            })
                                            .collect::<Vec<_>>();
                                    }
                                    Some(Operation::Move { origin: _, second })
                                    | Some(Operation::Resize {
                                        direction: _,
                                        origin: _,
                                        second,
                                    }) => {
                                        second.x = column;
                                        second.y = row;
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                            Ok(None)
                        }
                        // MouseEventKind::Moved => todo!(),
                        // MouseEventKind::ScrollDown => todo!(),
                        // MouseEventKind::ScrollUp => todo!(),
                        // MouseEventKind::ScrollLeft => todo!(),
                        // MouseEventKind::ScrollRight => todo!(),
                        _ => Ok(None),
                    }
                }
            }
        }
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        use Constraint::{Fill, Length};

        frame.render_widget(
            Block::new().style(Style::new().bg(color_scheme::BACKGROUND)),
            area,
        );

        let [layers_area, canvas_area, style_area] =
            Layout::horizontal([Length(LIST_WIDTH), Fill(1), Length(24)]).areas(area);

        frame.render_widget(
            Paragraph::new(Text::from(
                (0..canvas_area.height)
                    .map(|y| {
                        let y = y % 2;
                        Line::from(
                            (0..canvas_area.width)
                                .map(|x| {
                                    if x % 2 == y {
                                        Span::styled(
                                            "  ".to_owned(),
                                            Style::new().bg(color_scheme::CHECKERS),
                                        )
                                    } else {
                                        Span::raw("  ".to_owned())
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>(),
            ))
            .style(Style::new().fg(color_scheme::SECONDARY)),
            canvas_area,
        );

        // Content

        let move_vec = if let Some(Operation::Move { origin, second }) = self.current_operation {
            Some(Offset {
                x: second.x as i32 - origin.x as i32,
                y: second.y as i32 - origin.y as i32,
            })
        } else {
            None
        };

        for (i, element) in self.canvas.elements.iter().enumerate() {
            let selected = self.selected_elements.contains(&i);
            element.draw_to(
                frame,
                &canvas_area,
                selected,
                if selected {
                    &self.current_operation
                } else {
                    &None
                },
            );
        }

        // Resize Handles

        if self.selected_elements.len() == 1 {
            let el = &self.canvas.elements[self.selected_elements[0]];
            draw_resize_handles(
                frame,
                &self
                    .current_operation
                    .map(|o| o.apply_transform(el.area()))
                    .unwrap_or(*el.area())
                    .offset(Offset {
                        x: canvas_area.x as i32,
                        y: canvas_area.y as i32,
                    }),
            );
        }

        // Operation

        if let Some(operation) = &self.current_operation {
            match operation {
                Operation::Selection { origin, second } => {
                    let sel_area = Rect {
                        x: origin.x.min(second.x),
                        y: origin.y.min(second.y),
                        width: origin.x.abs_diff(second.x) + 1,
                        height: origin.y.abs_diff(second.y) + 1,
                    };

                    match self.current_tool {
                        Tool::Cursor => frame.render_widget(
                            Block::new().style(Style::new().bg(color_scheme::SELECTION)),
                            sel_area.offset(Offset {
                                x: canvas_area.x as i32,
                                y: canvas_area.y as i32,
                            }),
                        ),
                        Tool::Box => frame.render_widget(
                            Block::bordered().style(Style::new().fg(color_scheme::FOREGROUND)),
                            sel_area.offset(Offset {
                                x: canvas_area.x as i32,
                                y: canvas_area.y as i32,
                            }),
                        ),
                        _ => {}
                    };
                }
                _ => {}
            }
        }

        // Toolbox

        let [_, toolbox_area] = Layout::vertical([Fill(1), Length(3)]).areas(canvas_area);

        frame.render_widget(
            Tabs::new(vec!["[v] Cursor", "[b] Box", "[l] Line", "[i] Text"])
                .style(Style::new().bg(color_scheme::BACKGROUND))
                .block(
                    Block::bordered()
                        .title("Tools")
                        .style(Style::default().fg(color_scheme::MUTED)),
                )
                .highlight_style(
                    Style::default()
                        .fg(color_scheme::FOREGROUND)
                        .add_modifier(Modifier::BOLD),
                )
                .select(match self.current_tool {
                    Tool::Cursor => 0,
                    Tool::Box => 1,
                    Tool::Line => 2,
                    Tool::Text => 3,
                }),
            center_horizontal(toolbox_area, 46),
        );

        // Style Editor

        let [position_area, border_area, shadow_area] =
            Layout::vertical([Length(6), Length(8), Length(6)])
                .margin(1)
                .areas(style_area);

        frame.render_widget(
            Block::bordered()
                .title("Position")
                .style(Style::new().fg(color_scheme::FOREGROUND))
                .border_style(Style::default().fg(color_scheme::MUTED)),
            position_area,
        );
        frame.render_widget(
            Block::bordered()
                .title("Border")
                .style(Style::new().fg(color_scheme::FOREGROUND))
                .border_style(Style::default().fg(color_scheme::MUTED)),
            border_area,
        );
        frame.render_widget(
            Block::bordered()
                .title("Shadow")
                .style(Style::new().fg(color_scheme::FOREGROUND))
                .border_style(Style::default().fg(color_scheme::MUTED)),
            shadow_area,
        );

        frame.render_widget(
            List::new(
                self.canvas
                    .elements
                    .iter()
                    .rev()
                    .map(|x| format!(" {}", x.name()))
                    .enumerate()
                    .map(|(i, x)| {
                        if self.selected_elements.contains(&i) {
                            x.fg(color_scheme::SELECTION_FG)
                        } else {
                            Span::raw(x)
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::new()
                    .title(" [e] Elements")
                    .style(Style::default().fg(color_scheme::MUTED)),
            ),
            layers_area,
        );

        Ok(())
    }
}

fn center_horizontal(area: Rect, width: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    area
}

fn draw_resize_handles(frame: &mut Frame, area: &Rect) {
    let style = Style::new().fg(color_scheme::SELECTION_FG);

    frame.render_widget(
        Paragraph::new("▄").style(style),
        area.offset(Offset { x: -1, y: -1 }),
    );
    frame.render_widget(
        Paragraph::new("▀").style(style),
        area.offset(Offset {
            x: -1,
            y: area.height as i32,
        }),
    );
    frame.render_widget(
        Paragraph::new("▄").style(style),
        area.offset(Offset {
            x: area.width as i32,
            y: -1,
        }),
    );
    frame.render_widget(
        Paragraph::new("▀").style(style),
        area.offset(Offset {
            x: area.width as i32,
            y: area.height as i32,
        }),
    );
}
