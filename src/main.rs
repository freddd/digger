extern crate clap;
extern crate serde;
extern crate serde_yaml;

mod aws;
mod azure;
mod gcp;

use clap::{App, Arg, SubCommand};
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<(), ()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let matches = App::new("digger")
        .version("1.0")
        .author("freddd")
        .about("Finds bucket/container misconfigurations")
        .subcommand(
            SubCommand::with_name("s3")
                .about("scans s3 buckets")
                .arg(Arg::with_name("buckets").required(true).min_values(1))
                .arg(
                    Arg::with_name("region")
                        .short("r")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("gcs")
                .about("scans gcs buckets")
                .arg(Arg::with_name("buckets").required(true).min_values(1)),
        )
        .subcommand(
            SubCommand::with_name("storage")
                .about("scans azure storage")
                .arg(Arg::with_name("containers").required(true).min_values(1))
                .arg(
                    Arg::with_name("account")
                        .required(true)
                        .short("a")
                        .long("account")
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("s3", Some(arg_matches)) => {
            let b: Vec<&str> = arg_matches.values_of("buckets").unwrap().collect();
            let region: &str = arg_matches.value_of("region").unwrap();

            aws::s3::AWSs3::new(region).scan(b).await;
        }
        ("gcs", Some(bucket_matches)) => {
            let b: Vec<&str> = bucket_matches.values_of("buckets").unwrap().collect();
            gcp::gcs::GCS.scan(b).await;
        }
        ("storage", Some(matches)) => {
            let containers: Vec<&str> = matches.values_of("containers").unwrap().collect();
            let account: &str = matches.value_of("account").unwrap();

            azure::storage::AzureStorage::new(account)
                .scan(containers)
                .await;
        }
        _ => unreachable!(),
    }

    Ok(())
}
