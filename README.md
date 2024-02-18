# rust_xtb_api
XTB API in Rust.

# Example
```rust
let mut xtb = XtbConnection::new().await.expect("Failed to connect to Xtb");
let login_response = xtb.issue_command(&XtbCommand::Login(XtbLoginCommand::new(&login, &password))).await.expect("Failed to login to Xtb");

let request_response = xtb.issue_command(&XtbCommand::GetLastChartRequest(XtbGetLastChartRequestCommand::new(
        "B24.PL",
        XtbPeriod::D1,
        NaiveDateTime::parse_from_str("2023-12-10 07:00:00", "%Y-%m-%d %H:%M:%S").unwrap()
))).await.expect("Failed to obtain data from Xtb");

let data = GetLastChartRequestReturnData::new(&request_response).expect("Failed to parse get last char request response");
    
let logout_response = xtb.issue_command(&XtbCommand::Logout(XtbLogoutCommand::new())).await.expect("Failed to logout from Xtb");
```

# Tests
In order to run tests you have to specify login and password to real XTB account as environment variables.
```
Windows command_line:
set XTB_LOGIN="login"
set XTB_PASSWORD="password"
```
, or add them to [build] section in `Config.toml`.