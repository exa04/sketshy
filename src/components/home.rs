use std::{collections::HashSet, fs::File, io::Write, ops::Not, path::PathBuf};

use color_eyre::Result;
use crossterm::event::{KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use layout::{Flex, Offset};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::TextArea;

use super::Component;
use crate::{
    action::Action,
    app::color_scheme,
    config::Config,
    drawing::{Direction, DrawingCanvas, Element, Operation, StraightLine},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    Cursor,
    Box,
    Line,
    Text,
}

const LIST_WIDTH: u16 = 12;
const STYLE_WIDTH: u16 = 20;

const SCROLL_STEP: u16 = 4;

impl Default for Tool {
    fn default() -> Self {
        Self::Cursor
    }
}

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    current_tool: Tool,
    current_operation: Option<Operation>,
    selected_elements: HashSet<usize>,
    canvas: DrawingCanvas,
    scroll_offset: Position,
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
        if let Some(Operation::EditText { textarea }) = &mut self.current_operation {
            textarea.input(key);
        }
        Ok(None)
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        let MouseEvent {
            kind,
            column,
            row,
            modifiers,
        } = mouse;

        if column < LIST_WIDTH {
            return Ok(None);
        }

        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = row + self.scroll_offset.y;
                let column = column - LIST_WIDTH + self.scroll_offset.x;
                match self.current_tool {
                    Tool::Cursor => {
                        if self
                            .selected_elements
                            .iter()
                            .flat_map(|i| self.canvas.elements.get(*i))
                            .any(|el| el.area().contains(Position { x: column, y: row }))
                        {
                            self.current_operation = Some(Operation::Move {
                                origin: Position { x: column, y: row },
                                second: Position { x: column, y: row },
                            });
                            Ok(None)
                        } else {
                            if self.selected_elements.len() == 1 {
                                if let Some(Element::Line(line)) = self
                                    .selected_elements
                                    .iter()
                                    .next()
                                    .and_then(|i| self.canvas.elements.get(*i))
                                {
                                    use crate::drawing::elements::LineDirection::*;

                                    let handles = match &line.direction {
                                        Right => [
                                            (line.from.x as i16 - 1, line.from.y as i16),
                                            (line.to.x as i16 + 1, line.to.y as i16),
                                        ],
                                        DownRight => [
                                            (line.from.x as i16 - 2, line.from.y as i16 - 1),
                                            (line.to.x as i16 + 2, line.to.y as i16 + 1),
                                        ],
                                        Down => [
                                            (line.from.x as i16, line.from.y as i16 - 1),
                                            (line.to.x as i16, line.to.y as i16 + 1),
                                        ],
                                        DownLeft => [
                                            (line.from.x as i16 + 2, line.from.y as i16 - 1),
                                            (line.to.x as i16 - 2, line.to.y as i16 + 1),
                                        ],
                                        Left => [
                                            (line.from.x as i16 + 1, line.from.y as i16),
                                            (line.to.x as i16 - 1, line.to.y as i16),
                                        ],
                                        UpLeft => [
                                            (line.from.x as i16 + 2, line.from.y as i16 + 1),
                                            (line.to.x as i16 - 2, line.to.y as i16 - 1),
                                        ],
                                        Up => [
                                            (line.from.x as i16, line.from.y as i16 + 1),
                                            (line.to.x as i16, line.to.y as i16 - 1),
                                        ],
                                        UpRight => [
                                            (line.from.x as i16 - 2, line.from.y as i16 + 1),
                                            (line.to.x as i16 + 2, line.to.y as i16 - 1),
                                        ],
                                    };

                                    if handles[0].0 + self.scroll_offset.x as i16 == column as i16
                                        && handles[0].1 + self.scroll_offset.y as i16 == row as i16
                                    {
                                        self.current_operation = Some(Operation::MoveLineHandle {
                                            handle: crate::drawing::LineHandle::First,
                                            pos: (column, row).into(),
                                        });
                                        return Ok(Some(Action::RenderBuffer));
                                    } else if handles[1].0 + self.scroll_offset.x as i16
                                        == column as i16
                                        && handles[1].1 + self.scroll_offset.y as i16 == row as i16
                                    {
                                        self.current_operation = Some(Operation::MoveLineHandle {
                                            handle: crate::drawing::LineHandle::Second,
                                            pos: (column, row).into(),
                                        });
                                        return Ok(Some(Action::RenderBuffer));
                                    }
                                } else if let Some(direction) = self
                                    .selected_elements
                                    .iter()
                                    .filter_map(|i| {
                                        self.canvas.elements.get(*i).and_then(|el| {
                                            let area = el.area();

                                            if area.x.saturating_sub(1) == column
                                                && area.y.saturating_sub(1) == row
                                            {
                                                Some(Direction::TopLeft)
                                            } else if area.x + area.width == column
                                                && area.y.saturating_sub(1) == row
                                            {
                                                Some(Direction::TopRight)
                                            } else if area.x.saturating_sub(1) == column
                                                && area.y + area.height == row
                                            {
                                                Some(Direction::BottomLeft)
                                            } else if area.x + area.width == column
                                                && area.y + area.height == row
                                            {
                                                Some(Direction::BottomRight)
                                            } else {
                                                None
                                            }
                                        })
                                    })
                                    .next()
                                {
                                    self.current_operation = Some(Operation::Resize {
                                        direction,
                                        origin: Position { x: column, y: row },
                                        second: Position { x: column, y: row },
                                    });
                                    return Ok(Some(Action::RenderBuffer));
                                }
                            }
                            self.current_operation = Some(Operation::Selection {
                                origin: Position { x: column, y: row },
                                second: Position { x: column, y: row },
                            });

                            if modifiers & KeyModifiers::ALT != KeyModifiers::ALT {
                                self.selected_elements.clear();
                            }

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
                                self.selected_elements.insert(x);
                            }

                            Ok(Some(Action::RenderBuffer))
                        }
                    }
                    Tool::Box | Tool::Line => {
                        self.selected_elements.clear();
                        self.current_operation = Some(Operation::Selection {
                            origin: (column, row).into(),
                            second: (column, row).into(),
                        });
                        Ok(None)
                    }
                    Tool::Text => {
                        if let Some(Operation::EditText { .. }) = &self.current_operation {
                            Ok(Some(Action::CommitText))
                        } else {
                            self.selected_elements.clear();
                            self.current_operation = Some(Operation::Selection {
                                origin: (column, row).into(),
                                second: (column, row).into(),
                            });
                            Ok(None)
                        }
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let row = row + self.scroll_offset.y;
                let column = column - LIST_WIDTH + self.scroll_offset.x;
                match self.current_tool {
                    Tool::Box | Tool::Text | Tool::Line => {
                        if let Some(Operation::Selection { origin: _, second }) =
                            &mut self.current_operation
                        {
                            second.x = column;
                            second.y = row;
                        }
                        Ok(Some(Action::RenderBuffer))
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
                                .filter_map(|(i, el)| el.area().intersects(area).then_some(i))
                                .collect::<HashSet<_>>();

                            Ok(Some(Action::RenderBuffer))
                        }
                        Some(Operation::Move { second: pos, .. })
                        | Some(Operation::Resize { second: pos, .. })
                        | Some(Operation::MoveLineHandle { pos, .. }) => {
                            pos.x = column;
                            pos.y = row;
                            Ok(Some(Action::RenderBuffer))
                        }
                        _ => Ok(None),
                    },
                }
            }
            MouseEventKind::Up(MouseButton::Left) => match self.current_tool {
                Tool::Box => {
                    if let Some(Operation::Selection { origin, second }) = self.current_operation {
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
                                .insert(self.canvas.elements.len() - 1);
                        }
                    }
                    self.current_operation = None;
                    Ok(Some(Action::RenderBuffer))
                }
                Tool::Line => {
                    if let Some(Operation::Selection { origin, second }) = self.current_operation {
                        if origin != second {
                            if let Some(line) = StraightLine::new(origin, second) {
                                self.canvas.elements.push_back(Element::Line(line));
                            }
                            self.reset_tool();
                            self.selected_elements
                                .insert(self.canvas.elements.len() - 1);
                        }
                    }
                    self.current_operation = None;
                    Ok(Some(Action::RenderBuffer))
                }
                Tool::Text => {
                    if let Some(Operation::Selection { origin, second }) = self.current_operation {
                        let area = Rect {
                            x: origin.x.min(second.x),
                            y: origin.y.min(second.y),
                            width: origin.x.abs_diff(second.x) + 1,
                            height: origin.y.abs_diff(second.y) + 1,
                        };

                        if let Some((i, Element::Text { content, area })) = self
                            .canvas
                            .elements
                            .iter()
                            .enumerate()
                            .filter(|(_, el)| matches!(el, Element::Text { .. }))
                            .find(|(_, el)| {
                                el.area().contains(Position {
                                    x: area.x,
                                    y: area.y,
                                })
                            })
                            .filter(|_| area.area() == 1)
                        {
                            self.selected_elements.clear();
                            self.selected_elements.insert(i);
                            let mut textarea = TextArea::from(content.split('\n'));
                            textarea.set_block(
                                Block::new().style(Style::new().bg(color_scheme::BG_ELEVATED)),
                            );
                            textarea.move_cursor(tui_textarea::CursorMove::Jump(
                                origin.y - area.y,
                                origin.x - area.x,
                            ));
                            self.current_operation = Some(Operation::EditText { textarea });
                            Ok(Some(Action::EditText))
                        } else if area.width > 1 && area.height >= 1 {
                            self.canvas.elements.push_back(Element::Text {
                                area,
                                content: "".into(),
                            });
                            self.selected_elements
                                .insert(self.canvas.elements.len() - 1);
                            let mut textarea = TextArea::default();
                            textarea.set_block(
                                Block::new().style(Style::new().bg(color_scheme::BG_ELEVATED)),
                            );
                            self.current_operation = Some(Operation::EditText { textarea });
                            Ok(Some(Action::EditText))
                        } else {
                            self.reset_tool();
                            Ok(Some(Action::RenderBuffer))
                        }
                    } else {
                        self.current_operation = None;
                        Ok(None)
                    }
                }
                Tool::Cursor => {
                    if let Some(op) = &self.current_operation {
                        for i in &self.selected_elements {
                            if let Some(element) = op.apply_transform(&self.canvas.elements[*i]) {
                                self.canvas.elements[*i] = element;
                            }
                        }
                        self.current_operation = None;
                        Ok(Some(Action::RenderBuffer))
                    } else {
                        self.current_operation = None;
                        Ok(Some(Action::RenderBuffer))
                    }
                }
            },
            MouseEventKind::ScrollDown => Ok(Some(Action::ScrollDown)),
            MouseEventKind::ScrollUp => Ok(Some(Action::ScrollUp)),
            MouseEventKind::ScrollRight => Ok(Some(Action::ScrollRight)),
            MouseEventKind::ScrollLeft => Ok(Some(Action::ScrollLeft)),
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::RenderBuffer => {
                self.canvas
                    .render(&self.selected_elements, &self.current_operation);
                Ok(None)
            }
            Action::SwitchTool(tool) => {
                if tool != self.current_tool {
                    if self.current_operation.is_some() {
                        self.current_operation = None;
                    }

                    self.current_tool = tool;
                }
                Ok(None)
            }
            Action::EditText => Ok(Some(Action::RenderBuffer)),
            Action::CommitText => {
                if let Some(Element::Text { content, .. }) = self
                    .selected_elements
                    .iter()
                    .next()
                    .and_then(|i| self.canvas.elements.get_mut(*i))
                {
                    *content =
                        if let Some(Operation::EditText { textarea }) = &self.current_operation {
                            textarea.lines().join("\n")
                        } else {
                            "".into()
                        };
                }
                self.current_operation = None;
                Ok(Some(Action::RenderBuffer))
            }
            Action::SelectAll => {
                self.update_tool(Tool::Cursor);
                self.selected_elements =
                    (0..self.canvas.elements.len()).collect::<HashSet<usize>>();
                Ok(Some(Action::RenderBuffer))
            }
            Action::SelectNone => {
                self.selected_elements.clear();
                Ok(Some(Action::RenderBuffer))
            }
            Action::Delete => {
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
                Ok(Some(Action::SelectNone))
            }
            Action::OpenCommandPalette => {
                self.selected_elements.clear();
                Ok(None)
            }
            Action::ScrollUp => {
                self.scroll_offset.y = self.scroll_offset.y.saturating_sub(SCROLL_STEP);
                Ok(Some(Action::RenderBuffer))
            }
            Action::ScrollDown => {
                self.scroll_offset.y += SCROLL_STEP;
                Ok(Some(Action::RenderBuffer))
            }
            Action::ScrollLeft => {
                self.scroll_offset.x = self.scroll_offset.x.saturating_sub(SCROLL_STEP * 2);
                Ok(Some(Action::RenderBuffer))
            }
            Action::ScrollRight => {
                self.scroll_offset.x += SCROLL_STEP * 2;
                Ok(Some(Action::RenderBuffer))
            }
            Action::Export(path) => {
                let mut file = File::create(PathBuf::from(path))?;
                file.write_all(&self.canvas.to_string())?;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        use Constraint::{Fill, Length};

        frame.render_widget(
            Block::new().style(Style::new().bg(color_scheme::BG_BASE)),
            area,
        );

        let [layers_area, canvas_area, style_area] =
            Layout::horizontal([Length(LIST_WIDTH), Fill(1), Length(STYLE_WIDTH)]).areas(area);

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
                                            Style::new().bg(color_scheme::BG_CHECKERS),
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
            .style(Style::new().fg(color_scheme::FG_SECONDARY)),
            canvas_area,
        );

        // Content

        let xs = 0..canvas_area.width;
        let ys = 0..canvas_area.height;

        for (x, y) in xs.flat_map(|x| ys.clone().map(move |y| (x, y))) {
            if let Some(cell) = self
                .canvas
                .buffer
                .cell((x + self.scroll_offset.x, y + self.scroll_offset.y))
            {
                if let Some(frame_cell) = frame.buffer_mut().cell_mut((x + canvas_area.x, y)) {
                    frame_cell.set_symbol(cell.symbol());
                    frame_cell.set_fg(cell.fg);
                }
            }
        }

        // Resize Handles

        if self.selected_elements.len() == 1 {
            if let Some(el) = self
                .selected_elements
                .iter()
                .next()
                .and_then(|i| self.canvas.elements.get(*i))
            {
                let trans = self
                    .current_operation
                    .as_ref()
                    .and_then(|op| op.apply_transform(el));

                let el = trans.as_ref().unwrap_or(el);

                if let Element::Line(line) = el {
                    use crate::drawing::LineDirection::*;
                    let style = Style::new().fg(color_scheme::FG_SELECTION);

                    match line.direction {
                        Right => [
                            (line.from.x as i16 - 1, line.from.y as i16),
                            (line.to.x as i16 + 1, line.to.y as i16),
                        ],
                        DownRight => [
                            (line.from.x as i16 - 2, line.from.y as i16 - 1),
                            (line.to.x as i16 + 2, line.to.y as i16 + 1),
                        ],
                        Down => [
                            (line.from.x as i16, line.from.y as i16 - 1),
                            (line.to.x as i16, line.to.y as i16 + 1),
                        ],
                        DownLeft => [
                            (line.from.x as i16 + 2, line.from.y as i16 - 1),
                            (line.to.x as i16 - 2, line.to.y as i16 + 1),
                        ],
                        Left => [
                            (line.to.x as i16 - 1, line.to.y as i16),
                            (line.from.x as i16 + 1, line.from.y as i16),
                        ],
                        UpLeft => [
                            (line.to.x as i16 - 2, line.to.y as i16 - 1),
                            (line.from.x as i16 + 2, line.from.y as i16 + 1),
                        ],
                        Up => [
                            (line.to.x as i16, line.to.y as i16 - 1),
                            (line.from.x as i16, line.from.y as i16 + 1),
                        ],
                        UpRight => [
                            (line.to.x as i16 + 2, line.to.y as i16 - 1),
                            (line.from.x as i16 - 2, line.from.y as i16 + 1),
                        ],
                    }
                    .into_iter()
                    .map(|(x, y)| {
                        (
                            area.x.checked_add_signed(
                                x - self.scroll_offset.x as i16 + canvas_area.x as i16,
                            ),
                            area.y.checked_add_signed(
                                y - self.scroll_offset.y as i16 + canvas_area.y as i16,
                            ),
                        )
                    })
                    .filter_map(|(x, y)| x.zip(y).map(Position::from))
                    .filter(|pos| canvas_area.contains(*pos))
                    .map(|Position { x, y }| (Rect::new(x, y, 1, 1), Span::styled("■", style)))
                    .for_each(|(rect, s)| frame.render_widget(s, rect));
                } else {
                    draw_resize_handles(frame, &canvas_area, &el.area(), &self.scroll_offset);
                }
            }
        }

        // Selection

        match &self.current_operation {
            Some(Operation::Selection { origin, second }) => {
                let sel_area = Rect {
                    x: origin.x.min(second.x) - self.scroll_offset.x,
                    y: origin.y.min(second.y) - self.scroll_offset.y,
                    width: origin.x.abs_diff(second.x) + 1,
                    height: origin.y.abs_diff(second.y) + 1,
                }
                .offset(Offset {
                    x: canvas_area.x as i32,
                    y: canvas_area.y as i32,
                })
                .intersection(canvas_area);

                match self.current_tool {
                    Tool::Cursor => frame.render_widget(
                        Block::new().style(Style::new().bg(color_scheme::BG_SELECTION)),
                        sel_area,
                    ),
                    Tool::Box => frame.render_widget(
                        Block::bordered().style(Style::new().fg(color_scheme::FG_BASE)),
                        sel_area,
                    ),
                    Tool::Text => {
                        frame.render_widget(Clear, sel_area);
                        frame.render_widget(
                            Block::new().style(Style::new().bg(color_scheme::BG_ELEVATED)),
                            sel_area,
                        )
                    }
                    Tool::Line => {
                        StraightLine::new(
                            Position::new(
                                canvas_area.x + origin.x - self.scroll_offset.x,
                                canvas_area.y + origin.y - self.scroll_offset.y,
                            ),
                            Position::new(
                                canvas_area.x + second.x - self.scroll_offset.x,
                                canvas_area.y + second.y - self.scroll_offset.y,
                            ),
                        )
                        .inspect(|l| {
                            l.render_to(frame.buffer_mut(), Style::new().fg(color_scheme::FG_BASE))
                        });
                    }
                };
            }
            Some(Operation::EditText { textarea }) => {
                if let Some(el) = self.selected_elements.iter().next() {
                    frame.render_widget(
                        Clear,
                        self.canvas.elements[*el].area().offset(Offset {
                            x: (canvas_area.x as i32 - self.scroll_offset.x as i32),
                            y: (canvas_area.y as i32 - self.scroll_offset.y as i32),
                        }),
                    );
                    frame.render_widget(
                        textarea,
                        self.canvas.elements[*el].area().offset(Offset {
                            x: (canvas_area.x as i32 - self.scroll_offset.x as i32),
                            y: (canvas_area.y as i32 - self.scroll_offset.y as i32),
                        }),
                    );
                }
            }
            _ => (),
        }

        // Toolbox

        let [_, toolbox_area] = Layout::vertical([Fill(1), Length(3)]).areas(canvas_area);

        frame.render_widget(
            Tabs::new(vec!["[v] Cursor", "[b] Box", "[t] Text", "[l] Line"])
                .style(Style::new().bg(color_scheme::BG_BASE))
                .block(
                    Block::bordered()
                        .title("Tools")
                        .style(Style::default().fg(color_scheme::FG_MUTED)),
                )
                .highlight_style(
                    Style::default()
                        .fg(color_scheme::FG_BASE)
                        .add_modifier(Modifier::BOLD),
                )
                .select(match self.current_tool {
                    Tool::Cursor => 0,
                    Tool::Box => 1,
                    Tool::Text => 2,
                    Tool::Line => 3,
                }),
            center_horizontal(toolbox_area, 46),
        );

        // Scrollbars

        let style = Style::new()
            .bg(color_scheme::BG_BASE)
            .fg(color_scheme::FG_MUTED);

        let mut scrollbar_state = ScrollbarState::new(
            self.canvas
                .buffer
                .area
                .height
                .saturating_sub(canvas_area.height) as usize,
        )
        .position(self.scroll_offset.y as usize);

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .track_symbol(None)
                .begin_symbol(None)
                .end_symbol(None)
                .style(style),
            canvas_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );

        let mut scrollbar_state = ScrollbarState::new(
            self.canvas
                .buffer
                .area
                .width
                .saturating_sub(canvas_area.width) as usize,
        )
        .position(self.scroll_offset.x as usize);

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                .thumb_symbol("▄")
                .track_symbol(None)
                .begin_symbol(None)
                .end_symbol(None)
                .style(style),
            canvas_area.inner(Margin {
                vertical: 0,
                horizontal: 1,
            }),
            &mut scrollbar_state,
        );

        // Style Editor

        let [position_area, border_area, shadow_area] =
            Layout::vertical([Length(6), Length(8), Length(6)]).areas(style_area);

        if self.selected_elements.len() == 1 {
            frame.render_widget(
                Block::bordered()
                    .title("Position")
                    .style(Style::new().fg(color_scheme::FG_BASE))
                    .border_style(Style::default().fg(color_scheme::FG_MUTED)),
                position_area,
            );
            frame.render_widget(
                Block::bordered()
                    .title("Border")
                    .style(Style::new().fg(color_scheme::FG_BASE))
                    .border_style(Style::default().fg(color_scheme::FG_MUTED)),
                border_area,
            );
            frame.render_widget(
                Block::bordered()
                    .title("Shadow")
                    .style(Style::new().fg(color_scheme::FG_BASE))
                    .border_style(Style::default().fg(color_scheme::FG_MUTED)),
                shadow_area,
            );
        }

        frame.render_widget(
            List::new(
                self.canvas
                    .elements
                    .iter()
                    .map(|x| format!(" {}", x.name()))
                    .enumerate()
                    .map(|(i, x)| {
                        if self.selected_elements.contains(&i) {
                            x.fg(color_scheme::FG_SELECTION)
                        } else {
                            Span::raw(x)
                        }
                    })
                    .rev()
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::new()
                    .title(Span::styled(
                        " Layers",
                        Style::new().fg(color_scheme::FG_SECONDARY),
                    ))
                    .style(Style::default().fg(color_scheme::FG_MUTED)),
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

fn draw_resize_handles(frame: &mut Frame, canvas_area: &Rect, area: &Rect, offset: &Position) {
    let style = Style::new().fg(color_scheme::FG_SELECTION);

    [
        (-1, -1, "▄"),
        (area.width as i16, -1, "▄"),
        (-1, area.height as i16, "▀"),
        (area.width as i16, area.height as i16, "▀"),
    ]
    .into_iter()
    .map(|(x, y, s)| {
        (
            area.x
                .checked_add_signed(x - offset.x as i16 + canvas_area.x as i16),
            area.y
                .checked_add_signed(y - offset.y as i16 + canvas_area.y as i16),
            s,
        )
    })
    .filter_map(|(x, y, s)| x.zip(y).map(Position::from).zip(Some(s)))
    .filter(|(pos, _)| canvas_area.contains(*pos))
    .map(|(Position { x, y }, s)| (Rect::new(x, y, 1, 1), Span::styled(s, style)))
    .for_each(|(rect, s)| frame.render_widget(s, rect));
}
