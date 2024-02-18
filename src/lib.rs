use std::{io::{Read, Write}, net::TcpStream, thread, time::Duration};
use chrono::NaiveDateTime;
use native_tls::{TlsConnector, TlsStream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use log::debug;
use rust_decimal::Decimal;

const DEFAULT_XAPI_ADDRESS: &'static str = "xapi.xtb.com";
const DEFAULT_XAPI_PORT: u16 = 5124;    
// const DEFUALT_XAPI_STREAMING_PORT: u16 = 5125;
// pub const XTB_PERIOD_M1: u32 = 1;
// pub const XTB_PERIOD_M5: u32 = 5;
// pub const XTB_PERIOD_M15: u32 = 15;
// pub const XTB_PERIOD_M30: u32 = 30;
// pub const XTB_PERIOD_H1: u32 = 60;
// pub const XTB_PERIOD_H4: u32 = 240;
pub const XTB_PERIOD_D1: u32 = 1440;
// pub const XTB_PERIOD_W1: u32 = 10080;
// pub const XTB_PERIOD_MN1: u32 = 43200;

fn xtb_create_timestamp(datetime: NaiveDateTime) -> i64 {
    datetime.timestamp() * 1000
}

#[derive(Debug)]
pub enum XtbError {
    FailedToSendCommand,
    FailedToParseOutput,
    UnexpectedOutput,
    CommandFailed {
        error_description: String,
    },
    NetworkError,
    ConnectionNotOpened
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum XtbOutput {
    Success {
        status: bool,
        returnData: Value,
    },
    Fail {
        status: bool,
        errorCode: String,
        errorDescr: String,
    },
    LoginSuccessful {
        status: bool,
        streamSessionId: String
    },
    Logout {
        status: bool
    }
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct XtbLoginArguments {
    userId: String,
    password: String,
}

#[derive(Serialize)]
struct XtbLoginCommand {
    command: String,
    arguments: XtbLoginArguments,
}

impl XtbLoginCommand {
    fn new(user_id: &str, password: &str) -> Self {
        XtbLoginCommand { 
            command: "login".to_string(), 
            arguments: XtbLoginArguments {
                userId: user_id.to_string(),
                password: password.to_string(),
            } 
        }
    }

    fn new_json(user_id: &str, password: &str) -> String {
        let login_command = XtbLoginCommand::new(user_id, password);
        serde_json::to_string(&login_command).unwrap()
    }
}

#[derive(Serialize)]
struct XtbLogoutCommand {
    command: String
}

impl XtbLogoutCommand {
    fn new() -> Self {
        XtbLogoutCommand { command: "logout".to_string() }
    }

    fn new_json() -> String {
        let logout_command: XtbLogoutCommand = XtbLogoutCommand::new();
        serde_json::to_string(&logout_command).unwrap()
    }
}

fn parse_xtb_output_json(json: &String) -> Result<XtbOutput, XtbError> {
    let output: Result<XtbOutput, serde_json::Error> = serde_json::from_str(&json);
    match output {
        Ok(xtb_output) => return Ok(xtb_output),
        Err(_) => return Err(XtbError::FailedToParseOutput),
    }
}

#[derive(Serialize)]
struct ChartLastInfoRecord {
    period: u32,
    start: i64,
    symbol: String
}

#[derive(Serialize)]
struct GetLastChartRequestArguments {
    info: ChartLastInfoRecord    
}

#[derive(Serialize)]
struct XtbGetLastChartRequestCommand {
    command: String,
    arguments: GetLastChartRequestArguments
}

impl XtbGetLastChartRequestCommand {
    fn new(symbol: &str, period: u32, start_datetime: NaiveDateTime) -> XtbGetLastChartRequestCommand {
        XtbGetLastChartRequestCommand { 
            command: "getChartLastRequest".to_string(), 
            arguments: GetLastChartRequestArguments {
                info: ChartLastInfoRecord {
                    period: period,
                    start: xtb_create_timestamp(start_datetime),
                    symbol: symbol.to_string(),
                }
            } 
        }
    }

    fn new_json(symbol: &str, period: u32, start_datetime: NaiveDateTime) -> String {
        let command = XtbGetLastChartRequestCommand::new(symbol, period, start_datetime);
        serde_json::to_string(&command).unwrap()
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
#[derive(Debug)]
pub struct RateInfoRecord {
    pub close: Decimal,
    pub ctm: i64,
    pub ctmString: String,
    pub high: Decimal,
    pub low: Decimal,
    pub open: Decimal,
    pub vol: Decimal
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
#[derive(Debug)]
pub struct GetLastChartRequestReturnData {
    pub digits: u8,
    pub rateInfos: Vec<RateInfoRecord>
}

#[derive(Serialize)]
struct XtbGetAllSymbolsCommand {
    command: String
}

impl XtbGetAllSymbolsCommand {
    fn new() -> Self {
        XtbGetAllSymbolsCommand {
            command: "getAllSymbols".to_string()
        }
    }

    fn new_json() -> String {
        let command = XtbGetAllSymbolsCommand::new();
        serde_json::to_string(&command).unwrap()
    }
}

/// 
/// Xtb API implementation.
/// 
pub struct Xtb {
    user_id: String,
    password: String,
    tcp_client: Option<TcpStream>,
    tls_connector: TlsConnector,
    tls_client: Option<TlsStream<TcpStream>>,
}

impl Xtb {
    fn send(&mut self, command: &String) -> Result<(), XtbError> {
        if self.tls_client.is_none() {
            return Err(XtbError::ConnectionNotOpened);
        }

        thread::sleep(Duration::from_millis(200));

        match self.tls_client.as_mut().unwrap().write_all(command.as_bytes()) {
            Ok(_) => {
                debug!("Sent: {}.", command);
                Ok(())
            },
            Err(_) => Err(XtbError::FailedToSendCommand),
        }
    }
    
    fn receive_output(&mut self) -> Result<XtbOutput, XtbError> {
        if self.tls_client.is_none() {
            return Err(XtbError::ConnectionNotOpened);
        }

        let mut received_buffer: [u8; 2048] = [0; 2048];
        let mut received_string  = String::new();
    
        loop {
            let read_bytes = self.tls_client.as_mut().unwrap().read(&mut received_buffer).unwrap();
    
            received_string.push_str(std::str::from_utf8(&received_buffer[..read_bytes]).unwrap());
    
            if read_bytes > 2 {
                if received_buffer[read_bytes - 2] == b'\n' && received_buffer[read_bytes - 1] == b'\n' {
                    break;
                }
            }
        }
    
        received_string.truncate(received_string.len() - 2);
    
        debug!("Received: {}.", received_string);
        parse_xtb_output_json(&received_string)
    } 

    pub fn new(user_id: &str, password: &str) -> Self {
        Xtb { 
            user_id: user_id.to_string(), 
            password: password.to_string(), 
            tls_connector: TlsConnector::new().expect("Failed to create Tls connector."), 
            tls_client: Option::None,
            tcp_client: Option::None,
        }
    }

    pub fn connect(&mut self) -> Result<(), XtbError> {
        match TcpStream::connect((DEFAULT_XAPI_ADDRESS, DEFAULT_XAPI_PORT)) {
            Ok(tcp_client) => self.tcp_client = Option::Some(tcp_client),
            Err(_) => return Err(XtbError::NetworkError),
        }
        
        match self.tls_connector.connect(DEFAULT_XAPI_ADDRESS, self.tcp_client.as_mut().unwrap().try_clone().unwrap()) {
            Ok(tls_client) => self.tls_client = Option::Some(tls_client),
            Err(_) => return Err(XtbError::NetworkError),
        }

        Ok(())
    }

    pub fn login(&mut self) -> Result<XtbOutput, XtbError> {
        if self.tls_client.is_none() {
            return Err(XtbError::ConnectionNotOpened);
        }

        let login_command: String = XtbLoginCommand::new_json(&self.user_id, &self.password);
        self.send(&login_command)?;
        self.receive_output()
    }
    
    pub fn logout(&mut self) -> Result<XtbOutput, XtbError> {
        if self.tls_client.is_none() {
            return Err(XtbError::ConnectionNotOpened);
        }

        let logout_command: String = XtbLogoutCommand::new_json();
        self.send(&logout_command)?;
        self.receive_output()
    }

    pub fn get_last_chart_request(&mut self, symbol: &str, period: u32, start_datetime: NaiveDateTime) -> Result<GetLastChartRequestReturnData, XtbError> {
        if self.tls_client.is_none() {
            return Err(XtbError::ConnectionNotOpened);
        }

        let get_last_chart_request_command = XtbGetLastChartRequestCommand::new_json(symbol, period, start_datetime); 
        self.send(&get_last_chart_request_command)?;
        let output: XtbOutput = self.receive_output()?;
    
        match output {
            XtbOutput::Success { status: _, returnData } => {
                Ok(serde_json::from_value(returnData).unwrap())
            },
            XtbOutput::Fail { status: _, errorCode: _, errorDescr } => {
                Err(XtbError::CommandFailed { error_description: errorDescr })
            },
            _ => panic!("Unexpected Xtb output."),
        }
    }
    
    pub fn xtb_get_all_symbols(&mut self) -> Result<Vec<String>, XtbError> {
        if self.tls_client.is_none() {
            return Err(XtbError::ConnectionNotOpened);
        }

        self.send(&XtbGetAllSymbolsCommand::new_json())?;
        let output: XtbOutput = self.receive_output()?;
    
        let mut symbols: Vec<String> = Vec::new();
    
        match output {
            XtbOutput::Success { status: _, returnData } => {
                match returnData {
                    Value::Array(array) => {
                        for element in array {
                            match element {
                                Value::Object(symbol_data) => {
                                    match symbol_data.get("symbol") {
                                        Some(data) => {
                                            match data {
                                                Value::String(value) => symbols.push(value.clone()),
                                                _ => return Err(XtbError::UnexpectedOutput),
                                            }
                                        },
                                        None => return Err(XtbError::UnexpectedOutput),
                                    }
                                    
                                },
                                _ => return Err(XtbError::UnexpectedOutput),
                            }
                        }
                        return Ok(symbols);
                    },
                    _ => Err(XtbError::UnexpectedOutput),
                }
            },
            XtbOutput::Fail { status: _, errorCode: _, errorDescr } => return Err(XtbError::CommandFailed { error_description: errorDescr }),
            _ => Err(XtbError::UnexpectedOutput),
        }
    }
}