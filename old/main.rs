mod base;
mod config;
mod dynamic;
mod lb;
mod route;
mod tgin;
mod update;
mod utils;

mod api;

use crate::config::setup::{build_route, build_updates, load_config};
use crate::tgin::Tgin;

use clap::{Arg, Command};

#[cfg(test)]
mod mock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Command::new("tgin")
        .about("tgin is a telegram bot routing layer")
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Path to the configuration file")
                .default_value("tgin.ron"),
        );

    let matches = cli.get_matches();

    let config_path: &str = matches
        .get_one::<String>("file")
        .map(|s| s.as_str())
        .unwrap();

    let conf = load_config(config_path);
    let inputs = build_updates(conf.updates);
    let lb = build_route(conf.route);

    let mut tgin = Tgin::new(inputs, lb, conf.dark_threads, conf.server_port);

    if let Some(api) = conf.api {
        let api = api::router::Api::new(api.base_path);
        tgin.set_api(api);
    }

    if let Some(ssl) = conf.ssl {
        tgin.set_ssl(ssl.cert, ssl.key);
    }

    tgin.run();

    Ok(())
}
