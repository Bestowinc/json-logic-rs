use std::io;
use std::io::Read;

use anyhow::{Context, Result};
use clap::{App, Arg};
use serde_json;
use serde_json::Value;

use jsonlogic_rs;

fn configure_args<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.version(env!("CARGO_PKG_VERSION"))
        .author("Matthew Planchard <msplanchard@gmail.com>")
        .about(
            "Parse JSON data with a JsonLogic rule.\n\
            \n\
            When no <data> or <data> is -, read from stdin.
            \n\
            The result is written to stdout as JSON, so multiple calls \n\
            can be chained together if desired.",
        )
        .arg(
            Arg::with_name("logic")
                .help("A JSON logic string")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("data")
                .help("A string of JSON data to parse. May be provided as stdin.")
                .required(false)
                .takes_value(true),
        )
        .after_help(
            r#"EXAMPLES:
    jsonlogic '{"===": [{"var": "a"}, "foo"]}' '{"a": "foo"}'
    jsonlogic '{"===": [1, 1]}' null
    echo '{"a": "foo"}' | jsonlogic '{"===": [{"var": "a"}, "foo"]}'

Inspired by and conformant with the original JsonLogic (jsonlogic.com).

Report bugs to github.com/Bestowinc/json-logic-rs."#,
        )
}

fn main() -> Result<()> {
    let app = configure_args(App::new("jsonlogic"));
    let matches = app.get_matches();

    let logic = matches.value_of("logic").expect("logic arg expected");
    let json_logic: Value =
        serde_json::from_str(logic).context("Could not parse logic as JSON")?;

    // let mut data: String;
    let data_arg = matches.value_of("data").unwrap_or("-");

    let mut data: String;
    if data_arg != "-" {
        data = data_arg.to_string();
    } else {
        data = String::new();
        io::stdin().lock().read_to_string(&mut data)?;
    }
    let json_data: Value =
        serde_json::from_str(&data).context("Could not parse data as JSON")?;

    let result = jsonlogic_rs::apply(&json_logic, &json_data)
        .context("Could not execute logic")?;

    println!("{}", result.to_string());

    Ok(())
}
