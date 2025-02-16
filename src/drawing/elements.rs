use ratatui::{
    layout::{Position, Rect},
    style::Style,
    widgets::{Block, Clear, Paragraph, Widget},
};

use super::Operation;
use crate::app::color_scheme;

#[derive(Clone)]
pub enum Element {
    Box {
        area: Rect,
    },
    Text {
        area: Rect,
        content: String,
    },
    Line {
        from: (i32, i32),
        to: (i32, i32),
        character: char,
    },
}

impl Element {
    pub fn name(&self) -> String {
        match self {
            Self::Box { .. } => "Box".into(),
            Self::Text { content, .. } => format!("Text \"{}\"", content),
            Self::Line { from, to, .. } => {
                format!("Line ({},{})-({},{})", from.0, from.1, to.0, to.1)
            }
        }
    }

    pub fn area(&self) -> Rect {
        match self {
            Self::Box { area } | Self::Text { area, .. } => *area,
            Self::Line { from, to, .. } => {
                let (min_x, max_x) = if from.0 < to.0 {
                    (from.0, to.0)
                } else {
                    (to.0, from.0)
                };

                let (min_y, max_y) = if from.1 < to.1 {
                    (from.1, to.1)
                } else {
                    (to.1, from.1)
                };

                Rect {
                    x: min_x as u16,
                    y: min_y as u16,
                    width: (max_x - min_x) as u16 + 1,
                    height: (max_y - min_y) as u16 + 1,
                }
            }
        }
    }

    pub fn transform<F: Fn(&Rect) -> Rect>(&mut self, transform: F) {
        match self {
            Self::Box { ref mut area } | Self::Text { ref mut area, .. } => {
                *area = transform(area);
            }
            _ => (),
        }
    }

    pub(crate) fn straight_line(x1: i32, y1: i32, x2: i32, y2: i32) -> (i32, i32, char) {
        let (x, y) = (x2 as f32 - x1 as f32, y2 as f32 - y1 as f32);

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

        match diamond_angle % 2.0 {
            ..0.25 | 1.75.. => (x2, y1, '─'),
            0.75..1.25 => (x1, y2, '│'),

            0.25..0.75 => {
                let d = if x < 0.0 {
                    (x * 2.0).max(y)
                } else {
                    (x * 2.0).min(y)
                } as i32;
                (x1 + d * 2, y1 + d, '＼')
            }
            1.25..1.75 => {
                let d = if x < 0.0 {
                    (x * 2.0).max(y)
                } else {
                    (x * 2.0).min(y)
                } as i32;
                (x1 - d * 2, y1 + d, '／')
            }

            // 0.4..0.75 => (x2, y2, '\\'),
            // 1.25..1.4 => (x2, y2, '/'),
            _ => (0, 0, ' '),
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

        match self {
            Self::Box { area } => {
                let area = match &operation {
                    Some(x) => x.apply_transform(area),
                    None => *area,
                };

                Clear.render(area, buffer);
                Block::bordered().style(style).render(area, buffer);
            }
            Self::Text { area, content } => {
                let area = match &operation {
                    Some(x) => x.apply_transform(area),
                    None => *area,
                };

                Paragraph::new(content.as_str())
                    .style(style)
                    .render(area, buffer);
            }
            Self::Line {
                from,
                to,
                character,
                ..
            } => {
                use line_drawing::Bresenham;

                for (x, y) in Bresenham::new((from.0, from.1), (to.0, to.1)) {
                    if let Some(cell) = buffer.cell_mut(Position::from((x as u16, y as u16))) {
                        cell.set_char(*character);
                        cell.set_style(style);
                    }
                }
            }
        }
    }
}
