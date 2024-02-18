# rust_xtb_api
XTB API in Rust.

# Example
```
let xtb_login: &'static str = "11113333";
let xtb_password: &'static str = "some_password";
    
let mut xtb = Xtb::new(xtb_login, xtb_password);

xtb.connect()?;
let login_output = xtb.login()?;

match login_output {
    XtbOutput::Fail { status: _, errorCode: _, errorDescr } => panic!("Failed to loging to Xtb: {}.", errorDescr),
    XtbOutput::LoginSuccessful { status, streamSessionId: _ } => info!("Logged to Xtb: {}.", status),
    _ => panic!("Unexpected output."),
}

let last_chart: GetLastChartRequestReturnData = xtb.get_last_chart_request(
    &"PKN.PL_9", 
    XTB_PERIOD_D1, 
    NaiveDateTime::parse_from_str("2022-12-10 07:00:00", "%Y-%m-%d %H:%M:%S").unwrap())?;        
```
