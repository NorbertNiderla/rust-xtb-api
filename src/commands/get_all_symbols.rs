use serde::Serialize;

#[derive(Serialize)]
pub struct XtbGetAllSymbolsCommand {
    command: String
}

impl XtbGetAllSymbolsCommand {
    pub fn new() -> Self {
        XtbGetAllSymbolsCommand {
            command: "getAllSymbols".to_string()
        }
    }
}

//todo!("Implement get all symbols return data.");