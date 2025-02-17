use ratatui::{
    layout::{Offset, Position, Rect},
    style::Style,
    widgets::{Block, Clear, Paragraph, Widget},
};

use super::Operation;
use crate::app::color_scheme;

#[derive(Clone)]
pub enum Element {
    Box { area: Rect },
    Text { area: Rect, content: String },
    Line(StraightLine),
}

impl Element {
    pub fn name(&self) -> String {
        match self {
            Self::Box { .. } => "Box".into(),
            Self::Text { content, .. } => format!("Text \"{}\"", content),
            Self::Line(..) => "Line".into(),
        }
    }

    pub fn area(&self) -> Rect {
        match self {
            Self::Box { area } | Self::Text { area, .. } => *area,
            Self::Line(line) => line.area(),
        }
    }

    pub(crate) fn draw_to(
        &self,
        buffer: &mut ratatui::prelude::Buffer,
        selected: bool,
        operation: &Option<Operation>,
    ) {
        let style = if selected {
            Style::new()
                .fg(color_scheme::FG_SELECTION)
                .bg(color_scheme::BG_SELECTION)
        } else {
            Style::new().fg(color_scheme::FG_BASE)
        };

        let trans = operation.clone().and_then(|op| op.apply_transform(self));
        let el = trans.as_ref().unwrap_or(self);

        match el {
            Self::Box { area } => {
                Clear.render(*area, buffer);
                Block::bordered().style(style).render(*area, buffer);
            }
            Self::Text { area, content } => {
                Paragraph::new(content.as_str())
                    .style(style)
                    .render(*area, buffer);
            }
            Self::Line(line) => {
                line.render_to(buffer, style);
            }
        }
    }
}

#[derive(Clone)]
pub struct StraightLine {
    pub from: Position,
    pub to: Position,
    pub direction: LineDirection,
}

#[derive(Clone)]
pub enum LineDirection {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

impl StraightLine {
    pub fn new(from: Position, to: Position) -> Option<Self> {
        let (x, y) = (to.x as f32 - from.x as f32, to.y as f32 - from.y as f32);

        let diamond_angle = if y >= 0. {
            if x >= 0. {
                y / (x + y)
            } else {
                1.0 - x / (-x + y)
            }
        } else if x < 0.0 {
            2.0 - y / (-x - y)
        } else {
            3.0 + x / (x - y)
        };

        match diamond_angle % 4.0 {
            ..0.25 | 3.75.. => Some(Self {
                from: (from.x, from.y).into(),
                to: (to.x, from.y).into(),
                direction: LineDirection::Right,
            }),
            0.25..0.75 => Some(Self {
                from: (from.x, from.y).into(),
                to: (from.x + (to.y - from.y) * 2, to.y).into(),
                direction: LineDirection::DownRight,
            }),
            0.75..1.25 => Some(Self {
                from: (from.x, from.y).into(),
                to: (from.x, to.y).into(),
                direction: LineDirection::Down,
            }),
            1.25..1.75 => Some(Self {
                from: (from.x, from.y).into(),
                to: (from.x - (to.y - from.y) * 2, to.y).into(),
                direction: LineDirection::DownLeft,
            }),
            1.75..2.25 => Some(Self {
                from: (from.x, from.y).into(),
                to: (to.x, from.y).into(),
                direction: LineDirection::Left,
            }),
            2.25..2.75 => Some(Self {
                from: (from.x, from.y).into(),
                to: (from.x - (from.y - to.y) * 2, to.y).into(),
                direction: LineDirection::UpLeft,
            }),
            2.75..3.25 => Some(Self {
                from: (from.x, from.y).into(),
                to: (from.x, to.y).into(),
                direction: LineDirection::Up,
            }),
            3.25..3.75 => Some(Self {
                from: (from.x, from.y).into(),
                to: (from.x + (from.y - to.y) * 2, to.y).into(),
                direction: LineDirection::UpRight,
            }),
            _ => None,
        }
    }
    pub fn offset(&self, offset: Offset) -> StraightLine {
        StraightLine {
            from: Position {
                x: self.from.x.saturating_add_signed(offset.x as i16),
                y: self.from.y.saturating_add_signed(offset.y as i16),
            },
            to: Position {
                x: self.to.x.saturating_add_signed(offset.x as i16),
                y: self.to.y.saturating_add_signed(offset.y as i16),
            },
            direction: self.direction.clone(),
        }
    }
    pub fn area(&self) -> Rect {
        let (min_x, max_x) = if self.from.x < self.to.x {
            (self.from.x, self.to.x)
        } else {
            (self.to.x, self.from.x)
        };

        let (min_y, max_y) = if self.from.y < self.to.y {
            (self.from.y, self.to.y)
        } else {
            (self.to.y, self.from.y)
        };

        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        }
    }
    pub fn render_to(&self, buffer: &mut ratatui::prelude::Buffer, style: Style) {
        match self.direction {
            LineDirection::Right => {
                for x in self.from.x..=self.to.x {
                    if let Some(cell) = buffer.cell_mut(Position::new(x, self.from.y)) {
                        cell.set_char('─');
                        cell.set_style(style);
                    }
                }
            }
            LineDirection::DownRight => {
                for pos in (self.from.x..=self.to.x)
                    .step_by(2)
                    .zip(self.from.y..=self.to.y)
                    .map(Position::from)
                {
                    if let Some(cell) = buffer.cell_mut(pos) {
                        cell.set_char('＼');
                        cell.set_style(style);
                    }
                }
            }
            LineDirection::Down => {
                for y in self.from.y..=self.to.y {
                    if let Some(cell) = buffer.cell_mut(Position::new(self.from.x, y)) {
                        cell.set_char('│');
                        cell.set_style(style);
                    }
                }
            }
            LineDirection::DownLeft => {
                for pos in (self.to.x..=self.from.x)
                    .step_by(2)
                    .zip((self.from.y..=self.to.y).rev())
                    .map(Position::from)
                {
                    if let Some(cell) = buffer.cell_mut(pos) {
                        cell.set_char('／');
                        cell.set_style(style);
                    }
                }
            }
            LineDirection::Left => {
                for x in self.to.x..=self.from.x {
                    if let Some(cell) = buffer.cell_mut(Position::from((x, self.from.y))) {
                        cell.set_char('─');
                        cell.set_style(style);
                    }
                }
            }
            LineDirection::UpLeft => {
                for pos in (self.to.x..=self.from.x)
                    .step_by(2)
                    .zip(self.to.y..=self.from.y)
                    .map(Position::from)
                {
                    if let Some(cell) = buffer.cell_mut(pos) {
                        cell.set_char('＼');
                        cell.set_style(style);
                    }
                }
            }
            LineDirection::Up => {
                for y in self.to.y..=self.from.y {
                    if let Some(cell) = buffer.cell_mut(Position::new(self.from.x, y)) {
                        cell.set_char('│');
                        cell.set_style(style);
                    }
                }
            }
            LineDirection::UpRight => {
                for pos in (self.from.x..=self.to.x)
                    .step_by(2)
                    .zip((self.to.y..=self.from.y).rev())
                    .map(Position::from)
                {
                    if let Some(cell) = buffer.cell_mut(pos) {
                        cell.set_char('／');
                        cell.set_style(style);
                    }
                }
            }
        }
    }
}
