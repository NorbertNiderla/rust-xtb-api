use std::env;
use chrono::NaiveDateTime;
use rust_xtb_api::commands::login::XtbLoginCommand;
use rust_xtb_api::commands::logout::XtbLogoutCommand;
use rust_xtb_api::commands::get_chart_last::{ChartLastData, XtbGetChartLastRequestCommand};
use rust_xtb_api::{XtbConnection, XtbOutput, XtbPeriod};
use async_std;

#[async_std::test]
async fn test_login_and_logout() {
    let login = env::var("XTB_LOGIN").expect("XTB_LOGIN is not set");
    let password = env::var("XTB_PASSWORD").expect("XTB_PASSWORD is not set");

    let mut xtb = XtbConnection::new().await.expect("Failed to connect to Xtb");
    let login_response = xtb.issue_command(&XtbLoginCommand::new(&login, &password)).await.expect("Failed to login to Xtb");
    assert!(matches!(login_response, XtbOutput::LoginSuccessful { status: _, streamSessionId: _ }),
            "Login response is {:?}", login_response);

    let logout_response = xtb.issue_command(&XtbLogoutCommand::new()).await.expect("Failed to logout from Xtb");
    assert!(matches!(logout_response, XtbOutput::Logout { status: _ }));
}

#[async_std::test]
async fn test_getting_last_chart() {
    let login = env::var("XTB_LOGIN").expect("XTB_LOGIN is not set");
    let password = env::var("XTB_PASSWORD").expect("XTB_PASSWORD is not set");

    let mut xtb = XtbConnection::new().await.expect("Failed to connect to Xtb");
    let login_response = xtb.issue_command(&XtbLoginCommand::new(&login, &password)).await.expect("Failed to login to Xtb");
    assert!(matches!(login_response, XtbOutput::LoginSuccessful { status: _, streamSessionId: _ }),
            "Login response is {:?}", login_response);

    let request_response = xtb.issue_command(&XtbGetChartLastRequestCommand::new(
        "B24.PL",
        XtbPeriod::D1,
        NaiveDateTime::parse_from_str("2023-12-10 07:00:00", "%Y-%m-%d %H:%M:%S").unwrap()
    )).await.expect("Failed to obtain data from Xtb");

    assert!(matches!(request_response, XtbOutput::Success { status: _, returnData: _ }));

    let data = ChartLastData::new(&request_response).expect("Failed to parse get last char request response");

    // This is quite not the best, because it can change some time in the future, but whatever.
    assert_eq!(data.digits, 2);

    let logout_response = xtb.issue_command(&XtbLogoutCommand::new()).await.expect("Failed to logout from Xtb");
    assert!(matches!(logout_response, XtbOutput::Logout { status: _ }));
}
