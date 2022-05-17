#![allow(warnings, unused)]
use chrono::NaiveDate;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rand::Rng;
use scylla::frame::value::Timestamp;
use scylla::frame::value::ValueList;
use scylla::prepared_statement::PreparedStatement;
use scylla::statement::Consistency;
use scylla::transport::load_balancing::{DcAwareRoundRobinPolicy, TokenAwarePolicy};
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
    println!("Connecting to {} ...", host);
    let dc_robin = Box::new(DcAwareRoundRobinPolicy::new(dc.to_string()));
    let policy = Arc::new(TokenAwarePolicy::new(dc_robin));

    let session: Session = SessionBuilder::new()
        .known_node(host)
        .load_balancing(policy)
        .compression(Some(Compression::Lz4))
        .user(usr, pwd)
        .build()
        .await?;
    let session = Arc::new(session);

    println!("Connected successfully! Policy: TokenAware(DCAware())");

    // Set-up full table scan token ranges, shards and nodes

    let min_token = -(i128::pow(2, 63) - 1);
    let max_token = (i128::pow(2, 63) - 1);
    println!("Min token: {} \nMax token: {}", min_token, max_token);

    let num_nodes = 3;
    let cores_per_node = 2;

    // Parallelism is number of nodes * shards * 300% (ensure we keep them busy)
    let PARALLEL = (num_nodes * cores_per_node * 3);

    // Subrange is a division applied to our token ring. How many queries we'll send in total ?
    let SUBRANGE = PARALLEL * 1000;

    println!(
        "Max Parallel queries: {}\nToken-ring Subranges:{}",
        PARALLEL, SUBRANGE
    );

    // How many tokens are there?
    let total_token = max_token - min_token;
    println!("Total tokens: {}", total_token);

    // Chunk size determines the number of iterations needed to query the whole token ring
    let chunk_size = total_token / SUBRANGE;
    println!("Number of iterations: {}", chunk_size);

    // Prepare Statement - use LocalQuorum
    let stmt = format!("SELECT device, COUNT(device) AS total FROM {}.{} WHERE token(device) >= ? AND token(device) <= ? BYPASS CACHE", ks, table);
    let mut ps: PreparedStatement = session.prepare(stmt).await?;
    ps.set_consistency(Consistency::LocalQuorum);

    // Retry policy
    ps.set_retry_policy(Box::new(DefaultRetryPolicy::new()));

    println!();

    // 1. Spawn a new semaphore
    let sem = Arc::new(Semaphore::new((PARALLEL) as usize));

    let mut count: i64 = 0;
    let mut device_count = 0;

    // Initiate querying the token ring
    for x in (min_token..max_token).step_by((chunk_size as usize)) {
        let session = session.clone();
        let ps = ps.clone();
        let permit = sem.clone().acquire_owned().await;

        let mut v = vec![(x as i64), ((x + chunk_size - 1) as i64)];
        // Debug: Run this [ cargo run 172.17.0.2 north ks bla| egrep '^.[0-9][0-9][0-9][0-9][0-9]' | wc -l ]
        // This will return exact SUBRANGES numbers ;-)
        // println!("Querying: {} to {}", v[0], v[1]);
        tokio::task::spawn(async move {
            if let Some(query_result) = session.execute(&ps, (&v[0], &v[1])).await.unwrap().rows {
                for row in query_result {
                    if row.columns[0] != None {
                        let typed_row = row.into_typed::<(Uuid, i64)>();
                        let (device, metric_num) = typed_row.unwrap();
                        device_count += 1;
                        count += metric_num;
                        println!("Found device {} - Rows: {}", device, count);
                    }
                }
            }
            let _permit = permit;
        });
    }

    // Wait for all in-flight requests to finish
    for _ in 0..PARALLEL {
        sem.acquire().await.unwrap().forget();
    }

    // Won't work with tokio - but works without it (runs slower :-)
    //
    // println!("Total number of devices found: {}", device_count);
    // println!("Aggregated number of records: {}", count);

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
