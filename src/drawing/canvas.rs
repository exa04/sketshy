use std::collections::{HashSet, VecDeque};

use ratatui::{buffer::Buffer, layout::Rect};

use super::{Element, Operation};

#[derive(Default)]
pub struct DrawingCanvas {
    pub elements: VecDeque<Element>,
    pub buffer: Buffer,
}

impl DrawingCanvas {
    pub fn render(&mut self, selected_elements: &HashSet<usize>, operation: &Option<Operation>) {
        let areas = self.elements.iter().enumerate().map(|(i, el)| {
            selected_elements
                .get(&i)
                .and_then(|_| operation.as_ref().map(|op| op.apply_transform(el.area())))
                .unwrap_or_else(|| *el.area())
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
}
