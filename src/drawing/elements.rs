use ratatui::{
    layout::{Offset, Rect},
    style::Style,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{app::color_scheme, components::home::Operation};

#[derive(Clone)]
pub enum Element {
    Box { area: Rect },
}

impl Element {
    pub fn name(&self) -> String {
        "Box".to_owned()
    }

    pub fn area(&self) -> &Rect {
        match self {
            Self::Box { area } => area,
        }
    }

    pub fn draw_to(
        &self,
        frame: &mut Frame,
        canvas_region: &Rect,
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
            Element::Box { area } => {
                let area = operation
                    .map(|x| x.apply_transform(area))
                    .unwrap_or(*area)
                    .offset(Offset {
                        x: canvas_region.x as i32,
                        y: canvas_region.y as i32,
                    });

                frame.render_widget(
                    Paragraph::new(
                        (0..area.height)
                            .map(|_| {
                                let mut s = (0..area.width).map(|_| ' ').collect::<String>();
                                s.push('\n');
                                s
                            })
                            .collect::<String>(),
                    )
                    .block(Block::bordered().style(style)),
                    area.clone(),
                );
            }
        }
    }

    pub fn transform<F: Fn(&Rect) -> Rect>(&mut self, transform: F) {
        match self {
            Element::Box { ref mut area } => {
                *area = transform(area);
            }
        }
    }
}
