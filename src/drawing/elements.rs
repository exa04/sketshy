use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{
        canvas::{Canvas, Line},
        Block, Clear, Paragraph, Widget,
    },
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
        from: (bool, bool),
        to: (bool, bool),
        area: Rect,
    },
}

impl Element {
    pub fn name(&self) -> String {
        match self {
            Self::Box { .. } => "Box".into(),
            Self::Text { content, .. } => format!("Text \"{}\"", content),
            Self::Line { .. } => "Line".into(),
        }
    }

    pub fn area(&self) -> &Rect {
        match self {
            Self::Box { area } | Self::Text { area, .. } | Self::Line { area, .. } => area,
        }
    }

    pub fn transform<F: Fn(&Rect) -> Rect>(&mut self, transform: F) {
        match self {
            Self::Box { ref mut area }
            | Self::Text { ref mut area, .. }
            | Self::Line { ref mut area, .. } => {
                *area = transform(area);
            }
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
                area,
                from: (x1, y1),
                to: (x2, y2),
            } => {
                let area = match &operation {
                    Some(x) => x.apply_transform(area),
                    None => *area,
                };

                Canvas::default()
                    .x_bounds([0.0, 1.0])
                    .y_bounds([0.0, 1.0])
                    .paint(|ctx| {
                        ctx.draw(&Line::new(
                            *x1 as u8 as f64,
                            *y1 as u8 as f64,
                            *x2 as u8 as f64,
                            *y2 as u8 as f64,
                            if selected {
                                color_scheme::FG_SELECTION
                            } else {
                                color_scheme::FG_BASE
                            },
                        ));
                    })
                    .render(area, buffer);
            }
        }
    }
}
