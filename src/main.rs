extern crate clap;
extern crate serde;
extern crate serde_yaml;

mod aws;
mod azure;
mod gcp;

use clap::{Arg, Command};
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<(), ()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let matches = clap::Command::new("digger")
        .version("1.0")
        .author("freddd")
        .about("Finds bucket/container misconfigurations")
        .subcommand(
            Command::new("s3")
                .about("scans s3 buckets")
                .arg(Arg::new("buckets").required(true).num_args(1..))
                .arg(Arg::new("region").short('r').required(true).num_args(1)),
        )
        .subcommand(
            Command::new("gcs")
                .about("scans gcs buckets")
                .arg(Arg::new("buckets").required(true).num_args(1..)),
        )
        .subcommand(
            Command::new("storage")
                .about("scans azure storage")
                .arg(Arg::new("containers").required(true).num_args(1..))
                .arg(
                    Arg::new("account")
                        .required(true)
                        .short('a')
                        .long("account")
                        .num_args(1),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("s3", matches)) => {
            let b: Vec<&str> = matches
                .get_many::<String>("buckets")
                .unwrap()
                .map(|s| s.as_str())
                .collect();
            let region: &str = matches.get_one::<String>("region").unwrap();

            aws::s3::AWSs3::new(region).scan(b).await;
        }
        Some(("gcs", matches)) => {
            let b: Vec<&str> = matches
                .get_many::<String>("buckets")
                .unwrap()
                .map(|s| s.as_str())
                .collect();
            gcp::gcs::Gcs.scan(b).await;
        }
        Some(("storage", matches)) => {
            let b: Vec<&str> = matches
                .get_many::<String>("buckets")
                .unwrap()
                .map(|s| s.as_str())
                .collect();
            let account: &str = matches.get_one::<String>("account").unwrap();

            azure::storage::AzureStorage::new(account).scan(b).await;
        }
        _ => unreachable!(),
    };

    Ok(())
}
