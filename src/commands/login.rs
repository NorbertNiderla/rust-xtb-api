use serde::Serialize;

#[allow(non_snake_case)]
#[derive(Serialize)]
struct XtbLoginArguments {
    userId: String,
    password: String,
}

#[derive(Serialize)]
pub struct XtbLoginCommand {
    command: String,
    arguments: XtbLoginArguments,
}

impl XtbLoginCommand {
    pub fn new(user_id: &str, password: &str) -> Self {
        XtbLoginCommand { 
            command: "login".to_string(), 
            arguments: XtbLoginArguments {
                userId: user_id.to_string(),
                password: password.to_string(),
            } 
        }
    }
}