use std::time::Duration;
use async_std::io::{ReadExt, WriteExt};
use async_std::{task, net::TcpStream};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use log::debug;
use rust_decimal::Decimal;
use async_tls::TlsConnector;
use async_tls::client::TlsStream;

const DEFAULT_XAPI_ADDRESS: &'static str = "xapi.xtb.com";
const DEFAULT_XAPI_PORT: u16 = 5124;    
const DEFUALT_XAPI_STREAMING_PORT: u16 = 5125;

fn xtb_create_timestamp(datetime: NaiveDateTime) -> i64 {
    datetime.timestamp() * 1000
}

pub enum XtbPeriod {
    M1 = 1,
    M5 = 5,
    M15 = 15,
    M30 = 30,
    H1 = 60,
    H4 = 240,
    D1 = 1440,
    W1 = 10080,
    MN1 = 43200
}

#[derive(Debug)]
pub enum XtbError {
    FailedToConnect,
    FailedToSendCommand,
    SendTimeout,
    FailedToSerializeCommand,
    FailedToReceive,
    FailedToDecodeFromUtf8,
    FailedToParseOutput
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

#[derive(Serialize)]
pub struct XtbLogoutCommand {
    command: String
}

impl XtbLogoutCommand {
    pub fn new() -> Self {
        XtbLogoutCommand { command: "logout".to_string() }
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
pub struct XtbGetChartLastRequestCommand {
    command: String,
    arguments: GetLastChartRequestArguments
}

impl XtbGetChartLastRequestCommand {
    pub fn new(symbol: &str, period: XtbPeriod, start_datetime: NaiveDateTime) -> XtbGetChartLastRequestCommand {
        XtbGetChartLastRequestCommand { 
            command: "getChartLastRequest".to_string(), 
            arguments: GetLastChartRequestArguments {
                info: ChartLastInfoRecord {
                    period: period as u32,
                    start: xtb_create_timestamp(start_datetime),
                    symbol: symbol.to_string(),
                }
            } 
        }
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
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
#[derive(Deserialize, Debug)]
pub struct ChartLastData {
    pub digits: u8,
    pub rateInfos: Vec<RateInfoRecord>
}

impl ChartLastData {
    pub fn new(response: &XtbOutput) -> Option<ChartLastData> {
        assert!(matches!(response, XtbOutput::Success { status: _, returnData: _ }));
        
        match response {
            XtbOutput::Success { status: _, returnData } => {
                match serde_json::from_value(returnData.clone()) {
                    Ok(chart_data) => Option::Some(chart_data),
                    Err(_) => Option::None,
                }
            },
            _ => return Option::None,
        }
    }
}

#[derive(Serialize)]
pub struct XtbGetAllSymbolsCommand {
    command: String
}

impl XtbGetAllSymbolsCommand {
    fn new() -> Self {
        XtbGetAllSymbolsCommand {
            command: "getAllSymbols".to_string()
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum XtbCommand {
    Login(XtbLoginCommand),
    Logout(XtbLogoutCommand),
    GetChartLastRequest(XtbGetChartLastRequestCommand),
    GetAllSymbols(XtbGetAllSymbolsCommand),
}

pub struct XtbConnection {
    tls_client: TlsStream<TcpStream>
}

impl XtbConnection {
    async fn send(&mut self, command: &String) -> Result<(), XtbError> {
        task::sleep(Duration::from_millis(200)).await;

        match self.tls_client.write_all(command.as_bytes()).await {
            Ok(_) => {
                debug!("Sent: {}.", command);
                Ok(())
            },
            Err(_) => Err(XtbError::FailedToSendCommand),
        }
    }

    async fn receive_output(&mut self) -> Result<XtbOutput, XtbError> {
        let mut received_buffer: [u8; 2048] = [0; 2048];
        let mut received_string  = String::new();
    
        loop {
            let read_bytes;

            match self.tls_client.read(&mut received_buffer).await {
                Ok(recv) => read_bytes = recv,
                Err(_) => return Err(XtbError::FailedToReceive),
            }
    
            match std::str::from_utf8(&received_buffer[..read_bytes]) {
                Ok(message) => received_string.push_str(message),
                Err(_) => return Err(XtbError::FailedToDecodeFromUtf8),
            }
    
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

    pub async fn new() -> Result<Self, XtbError> {
        match TcpStream::connect((DEFAULT_XAPI_ADDRESS, DEFAULT_XAPI_PORT)).await {
            Ok(tcp_client) => {
                let tls_connector = TlsConnector::new();
                match tls_connector.connect(DEFAULT_XAPI_ADDRESS, tcp_client).await {
                    Ok(tls_client) => return Ok(XtbConnection { tls_client: tls_client }),
                    Err(_) => return Err(XtbError::FailedToConnect),
                }
            },
            Err(_) => return Err(XtbError::FailedToConnect),
        }
    }

    pub async fn issue_command(&mut self, command: &XtbCommand) -> Result<XtbOutput, XtbError> {
        let json;
        match serde_json::to_string(command) {
            Ok(command_json) => json = command_json,
            Err(_) => return Err(XtbError::FailedToSerializeCommand),
        }

        println!("{}", json);

        self.send(&json).await?;
        self.receive_output().await
    }
}
    
//     pub fn xtb_get_all_symbols(&mut self) -> Result<Vec<String>, XtbError> {
//         if self.tls_client.is_none() {
//             return Err(XtbError::ConnectionNotOpened);
//         }

//         self.send(&XtbGetAllSymbolsCommand::new_json())?;
//         let output: XtbOutput = self.receive_output()?;
    
//         let mut symbols: Vec<String> = Vec::new();
    
//         match output {
//             XtbOutput::Success { status: _, returnData } => {
//                 match returnData {
//                     Value::Array(array) => {
//                         for element in array {
//                             match element {
//                                 Value::Object(symbol_data) => {
//                                     match symbol_data.get("symbol") {
//                                         Some(data) => {
//                                             match data {
//                                                 Value::String(value) => symbols.push(value.clone()),
//                                                 _ => return Err(XtbError::UnexpectedOutput),
//                                             }
//                                         },
//                                         None => return Err(XtbError::UnexpectedOutput),
//                                     }
                                    
//                                 },
//                                 _ => return Err(XtbError::UnexpectedOutput),
//                             }
//                         }
//                         return Ok(symbols);
//                     },
//                     _ => Err(XtbError::UnexpectedOutput),
//                 }
//             },
//             XtbOutput::Fail { status: _, errorCode: _, errorDescr } => return Err(XtbError::CommandFailed { error_description: errorDescr }),
//             _ => Err(XtbError::UnexpectedOutput),
//         }
//     }
// }