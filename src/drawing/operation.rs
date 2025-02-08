use ratatui::layout::{Offset, Position, Rect};

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
    EditText {
        position: u16,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
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
            _ => *area,
        }
    }
}
