use serde::{Deserialize, Serialize};
use strum::Display;

use crate::components::home::Tool;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    RenderBuffer,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),

    Help,
    OpenCommandPalette,
    CloseCommandPalette,

    SwitchTool(Tool),
    #[serde(skip)]
    EditText,
    CommitText,
    SelectAll,
    SelectNone,
    Delete,

    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,

    Export(String),
}
