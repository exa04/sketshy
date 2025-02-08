use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear, Paragraph, Widget, Wrap},
};

use super::Operation;
use crate::app::color_scheme;

#[derive(Clone)]
pub enum Element {
    Box { area: Rect },
    Text { area: Rect, content: String },
}

impl Element {
    pub fn name(&self) -> String {
        match self {
            Self::Box { .. } => "Box".into(),
            Self::Text { content, .. } => format!("Text \"{}\"", content),
        }
    }

    pub fn area(&self) -> &Rect {
        match self {
            Self::Box { area } | Self::Text { area, .. } => area,
        }
    }

    pub fn transform<F: Fn(&Rect) -> Rect>(&mut self, transform: F) {
        match self {
            Self::Box { ref mut area } | Self::Text { ref mut area, .. } => {
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
                .fg(color_scheme::SELECTION_FG)
                .bg(color_scheme::SELECTION)
        } else {
            Style::new().fg(color_scheme::FOREGROUND)
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
                    .wrap(Wrap { trim: false })
                    .render(area, buffer);
            }
        }
    }
}
