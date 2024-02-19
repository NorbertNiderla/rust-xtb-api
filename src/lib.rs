use std::time::Duration;
use async_std::io::{ReadExt, WriteExt};
use async_std::{task, net::TcpStream};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use log::debug;
use async_tls::TlsConnector;
use async_tls::client::TlsStream;

pub mod commands;

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

fn parse_xtb_output_json(json: &String) -> Result<XtbOutput, XtbError> {
    let output: Result<XtbOutput, serde_json::Error> = serde_json::from_str(&json);
    match output {
        Ok(xtb_output) => return Ok(xtb_output),
        Err(_) => return Err(XtbError::FailedToParseOutput),
    }
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

    pub async fn issue_command<T: Serialize>(&mut self, command: &T) -> Result<XtbOutput, XtbError> {
        let json;
        match serde_json::to_string(command) {
            Ok(command_json) => json = command_json,
            Err(_) => return Err(XtbError::FailedToSerializeCommand),
        }

        self.send(&json).await?;
        self.receive_output().await
    }
}