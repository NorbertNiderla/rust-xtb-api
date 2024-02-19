use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::Serialize;
use serde::Deserialize;
use crate::XtbOutput;
use crate::xtb_create_timestamp;
use crate::XtbPeriod;

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