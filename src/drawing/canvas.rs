use std::collections::{HashSet, VecDeque};

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
};

use super::{Element, Operation};

#[derive(Default)]
pub struct DrawingCanvas {
    pub elements: VecDeque<Element>,
    pub buffer: Buffer,
}

impl DrawingCanvas {
    pub fn render(&mut self, selected_elements: &HashSet<usize>, operation: &Option<Operation>) {
        let areas = self.elements.iter().enumerate().map(|(i, el)| {
            if let Some(el) = selected_elements
                .get(&i)
                .and_then(|_| operation.as_ref().and_then(|op| op.apply_transform(el)))
            {
                el.area()
            } else {
                el.area()
            }
        });

        let max_width = areas
            .clone()
            .map(|el| el.x + el.width)
            .max()
            .unwrap_or_default();

        let max_height = areas.map(|el| el.y + el.height).max().unwrap_or_default();

        if max_width != self.buffer.area().width || max_height != self.buffer.area().height {
            self.buffer.resize(Rect {
                x: 0,
                y: 0,
                width: max_width,
                height: max_height,
            });
        }

        self.buffer.content.fill(' '.into());

        for (i, element) in self.elements.iter().enumerate() {
            let selected = selected_elements.contains(&i);
            element.draw_to(
                &mut self.buffer,
                selected,
                if selected { operation } else { &None },
            );
        }
    }
    pub fn to_string(&self) -> Vec<u8> {
        let mut out_string = String::with_capacity(self.buffer.area().area() as usize);

        for y in 0..self.buffer.area.height {
            for x in 0..self.buffer.area.width {
                out_string.push(
                    self.buffer
                        .cell((x, y))
                        .and_then(|c| c.symbol().chars().nth(0))
                        .unwrap_or_default(),
                );
            }
            out_string.push('\n');
        }

        out_string.as_bytes().to_vec()
    }
}
