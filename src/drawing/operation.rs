use ratatui::layout::{Offset, Position, Rect};
use tui_textarea::TextArea;

use super::{Element, StraightLine};

#[derive(Clone)]
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
    MoveLineHandle {
        handle: LineHandle,
        pos: Position,
    },
    EditText {
        textarea: TextArea<'static>,
    },
}

#[derive(Clone)]
pub enum LineHandle {
    First,
    Second,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Operation {
    fn transform_area(&self, area: &Rect) -> Rect {
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
                    width: (area.width as i32 - (second.x as i32 - origin.x as i32)).max(1) as u16,
                    height: (area.height as i32 - (second.y as i32 - origin.y as i32)).max(1)
                        as u16,
                },
                Direction::TopRight => Rect {
                    x: area.x,
                    y: (area.y as i32
                        + (second.y as i32 - origin.y as i32).min(area.height as i32 - 2))
                        as u16,
                    width: (area.width as i32 + (second.x as i32 - origin.x as i32)).max(1) as u16,
                    height: (area.height as i32 - (second.y as i32 - origin.y as i32)).max(1)
                        as u16,
                },
                Direction::BottomLeft => Rect {
                    x: (area.x as i32
                        + (second.x as i32 - origin.x as i32).min(area.width as i32 - 2))
                        as u16,
                    y: area.y,
                    width: (area.width as i32 - (second.x as i32 - origin.x as i32)).max(1) as u16,
                    height: (area.height as i32 + (second.y as i32 - origin.y as i32)).max(1)
                        as u16,
                },
                Direction::BottomRight => Rect {
                    x: area.x,
                    y: area.y,
                    width: (area.width as i32 + (second.x as i32 - origin.x as i32)).max(1) as u16,
                    height: (area.height as i32 + (second.y as i32 - origin.y as i32)).max(1)
                        as u16,
                },
            },
            _ => *area,
        }
    }

    pub fn apply_transform(&self, element: &Element) -> Option<Element> {
        match element {
            Element::Box { area } => Some(Element::Box {
                area: self.transform_area(area),
            }),
            Element::Text { area, content } => Some(Element::Text {
                area: self.transform_area(area),
                content: content.clone(),
            }),
            Element::Line(line) => match self {
                Operation::Move { origin, second } => {
                    let area = line.area();
                    Some(Element::Line(line.offset(Offset {
                        x: (second.x as i32 - origin.x as i32).max(-(area.x as i32)),
                        y: (second.y as i32 - origin.y as i32).max(-(area.y as i32)),
                    })))
                }
                Operation::MoveLineHandle { handle, pos } => match handle {
                    super::LineHandle::First => StraightLine::new(line.to, *pos),
                    super::LineHandle::Second => StraightLine::new(line.from, *pos),
                }
                .map(Element::Line),
                _ => None,
            },
        }
    }
}
