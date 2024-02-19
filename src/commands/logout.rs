use serde::Serialize;

#[derive(Serialize)]
pub struct XtbLogoutCommand {
    command: String
}

impl XtbLogoutCommand {
    pub fn new() -> Self {
        XtbLogoutCommand { command: "logout".to_string() }
    }
}