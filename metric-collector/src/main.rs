#![allow(warnings, unused)]
use chrono::NaiveDate;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rand::Rng;
use scylla::frame::value::CqlTimestamp;
use scylla::prepared_statement::PreparedStatement;
use scylla::statement::Consistency;
use scylla::load_balancing::DefaultPolicy;
use scylla::transport::ExecutionProfile;
use scylla::transport::retry_policy::DefaultRetryPolicy;
use scylla::transport::Compression;
use scylla::IntoTypedRows;
use scylla::{Session, SessionBuilder};
use std::env;
use std::error::Error;
use std::process;
use std::sync::Arc;
use std::time::Duration;
use std::{thread, time};
use tokio::sync::Semaphore;
use uuid::Uuid;

const DAYS: i64 = 3; // Number of days we want to collect
const SAMPLES: i64 = 288; // How many samples we want per day: [ (24 * 60) / 5 ]
const REPORT: i64 = 300; // Report the temperature every 5 minutes (300 secs)
const PARALLEL: usize = 2048; // Concurrency,
                              // let all shards work in parallel rather than hit a single
                              // shard at a time.

// UUID struct
pub const NAMESPACE_UUID: Uuid = Uuid::from_bytes([
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);

fn help() {
    println!("usage: <host> <dc>");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Simple argparse

    let args: Vec<String> = env::args().collect();

    let mut host = "127.0.0.1";
    let mut dc = "datacenter1";
    let mut usr = "scylla";
    let mut pwd = "scylla";

    match args.len() {
        1 => {
            println!("Using default values. Host: {}, DC: {}, Username: {}, Password: ********", host, dc, usr);
        }
        2 => {
            host = &args[1];
        }
        3 => {
            host = &args[1];
            dc = &args[2];
        }
        5 => {
            host = &args[1];
            dc = &args[2];
            usr = &args[3];
            pwd = &args[4];
        }
        _ => {
            help();
        }
    }

    let ks = "iot";
    let table = "device";

    // Initiate cluster session
    // We use Token aware DC Aware Round robin in this example
    // It is also possible to create your own load balancing policy as needed.
    println!("Connecting to {} ...", host);
    let default_policy = DefaultPolicy::builder()
        .prefer_datacenter(dc.to_string())
        .token_aware(true)
        .permit_dc_failover(false)
        .build();

    let profile = ExecutionProfile::builder()
        .load_balancing_policy(default_policy)
        .build();

    let handle = profile.into_handle();

    let session: Session = SessionBuilder::new()
        .known_node(host)
        .schema_agreement_interval(Duration::from_secs(5))
        .auto_await_schema_agreement(true)
        .default_execution_profile_handle(handle)
        .compression(Some(Compression::Lz4))
        .user(usr, pwd)
        .build()
        .await?;
    let session = Arc::new(session);

    println!("Connected successfully! Policy: TokenAware(DCAware())");

    // Create KS and Table
    let ks_stmt = format!("CREATE KEYSPACE IF NOT EXISTS {} WITH replication = {{'class': 'NetworkTopologyStrategy', '{}': 1}}", ks, dc);
    session.query(ks_stmt, &[]).await?;

    let cf_stmt = format!("CREATE TABLE IF NOT EXISTS {}.{} (device uuid, ts timestamp, temperature float, PRIMARY KEY(device, ts)) 
                           WITH default_time_to_live = 2592000 
                           AND compaction = {{'class': 'TimeWindowCompactionStrategy', 'compaction_window_size': 3}}", ks, table);
    session.query(cf_stmt, &[]).await?;

    println!("Keyspace and Table processing is complete");

    // Check for Schema Agreement
    if session
        .check_schema_agreement()
        .await?.is_some()
    {
        println!("Schema is in agreement - Proceeding");
    } else {
        println!("Schema is NOT in agreement - Stop processing");
        process::exit(1);
    }

    // Prepare Statement - use LocalQuorum
    // Always use Prepared Statements whenever possible
    // Prepared Statements are a requirement for TokenAware load balancing
    let stmt = format!(
        "INSERT INTO {}.{} (device, ts, temperature) VALUES (?, ?, ?)",
        ks, table
    );
    let mut ps: PreparedStatement = session.prepare(stmt).await?;
    // LocalQuorum means a majority of replicas need to acknowledge the operation
    // for it to be considered successful
    ps.set_consistency(Consistency::LocalQuorum);

    // Retry policy - the default when not specified
    // Another option would be using 'FalthroughRetryPolicy', which effectively never retries
    // Similarly as the loading balancing policy, it is also possible to implement your own retry policy
    ps.set_retry_policy(Some(Arc::new(DefaultRetryPolicy::new())));

    println!();

    // 1. Spawn a new semaphore
    // 2. Start ingestion
    let sem = Arc::new(Semaphore::new(PARALLEL));
    let num_devices = 100;
    for device in 1..=num_devices {
        println!("Generating data for device {}/{}", device, num_devices);
        // Generate an always random UUID
        // let uuid = Uuid::new_v4();

        // Consistent UUIDs per device in v5
        let uuid: Uuid = Uuid::new_v5(&NAMESPACE_UUID, &[device]);

        // 1. Start writing at January 1st, 2020 00:00 UTC
        // 2. Retrieve epoch and cast to i64
        // 3. Retrieve the total samples we want to record
        let start_dt =
            DateTime::<Utc>::from_utc(NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0), Utc);
        let mut dt = i64::from(start_dt.timestamp());
        let total_samples: i64 = SAMPLES * DAYS;

        // Concurrently write total_samples to cluster
        for write in 1..=total_samples {
            let session = session.clone();
            let ps = ps.clone();
            let permit = sem.clone().acquire_owned().await;
            let temperature: f32 = rand::thread_rng().gen_range(-40.0, 50.0);
            // Round to 2 decimal: Multiply by 1e2 and divide by 1e2
            let temperature = (temperature * 100.0).round() / 100.0;
            // DEBUG println!("{}", uuid);

            tokio::task::spawn(async move {
                session
                    .execute(&ps, (uuid, CqlTimestamp(dt * 1000), temperature))
                    .await
                    .unwrap();

                let _permit = permit;
            });
            dt = dt + REPORT;
        }
    }

    // Wait for all in-flight requests to finish
    for _ in 0..PARALLEL {
        sem.acquire().await.unwrap().forget();
    }

    // Print final metrics
    let metrics = session.get_metrics();
    println!("Queries requested: {}", metrics.get_queries_num());
    println!("Iter queries requested: {}", metrics.get_queries_iter_num());
    println!("Errors occured: {}", metrics.get_errors_num());
    println!("Iter errors occured: {}", metrics.get_errors_iter_num());
    println!("Average latency: {}", metrics.get_latency_avg_ms().unwrap());
    println!(
        "99.9 latency percentile: {}",
        metrics.get_latency_percentile_ms(99.9).unwrap()
    );

    Ok(())
}
