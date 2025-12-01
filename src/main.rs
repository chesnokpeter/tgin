mod base;
mod lb;
mod route;
mod tgin;
mod update;
mod config;
mod utils;

use crate::tgin::Tgin;
use crate::config::setup::{load_config, build_updates, build_route};

use clap::{Arg, Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Command::new("tgin")
        .about("tgin is a telegram bot routing layer")
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Path to the configuration file")
        );

    let matches = cli.get_matches();

    let config_path: &str = matches.get_one::<String>("file")
        .map(|s| s.as_str())
        .unwrap();


    let conf = load_config(config_path); 
    let inputs = build_updates(conf.updates);
    let lb = build_route(conf.route);

    let tgin = Tgin::new(
        inputs,
        lb,
        conf.dark_threads,
        conf.server_port,
    );

    tgin.run();

    Ok(())
}