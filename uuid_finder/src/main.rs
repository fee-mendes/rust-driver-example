#![allow(warnings, unused)]
use chrono::Duration;
use chrono::NaiveDate;
use chrono::{Date, DateTime, NaiveDateTime, TimeZone, Utc};
use parse_duration::parse;
use rand::Rng;
use scylla::frame::value::Timestamp;
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
use std::{thread, time};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Simple argparse

    let args: Vec<String> = env::args().collect();

    if args.len() < 7 {
        eprintln!("usage: <host> <dc> <ks> <table> <uuid> <start_date> <end_date>");
        process::exit(1);
    }

    let host = &args[1];
    let dc = &args[2];
    let ks = &args[3];
    let table = &args[4];
    let uuid_search = &args[5];
    let start_date = &args[6];
    let end_date = &args[7];

    // Convert dates, ensure UTC TZ
    let naive_start = NaiveDateTime::parse_from_str(start_date, "%Y-%m-%d %H:%M:%S").unwrap();
    let start = DateTime::<Utc>::from_utc(naive_start, Utc);

    let naive_end = NaiveDateTime::parse_from_str(end_date, "%Y-%m-%d %H:%M:%S").unwrap();
    let end = DateTime::<Utc>::from_utc(naive_end, Utc);

    // To Duration (see: https://cvybhu.github.io/scyllabook/data-types/timestamp.html)
    let to_start: Duration = Duration::seconds(start.timestamp());
    let to_end: Duration = Duration::seconds(end.timestamp());

    let hdr = "===========================================================================================================";

    // Initiate cluster session
    println!("Connecting to {} ...", host);
    let dc_robin = Box::new(DcAwareRoundRobinPolicy::new(dc.to_string()));
    let policy = Arc::new(TokenAwarePolicy::new(dc_robin));

    let session: Session = SessionBuilder::new()
        .known_node(host)
        .load_balancing(policy)
        .compression(Some(Compression::Lz4))
        .user("cassandra", "cassandra")
        .build()
        .await?;
    let session = Arc::new(session);

    println!("Connected successfully! Policy: TokenAware(DCAware())");

    // Prepare Statement - use LocalQuorum
    let max_stmt = format!(
        "SELECT max(temperature) FROM {}.{} WHERE device={} AND ts >= ? AND ts <= ?",
        ks, table, uuid_search
    );
    let min_stmt = format!(
        "SELECT min(temperature) FROM {}.{} WHERE device={} AND ts >= ? AND ts <= ?",
        ks, table, uuid_search
    );
    let avg_stmt = format!(
        "SELECT avg(temperature) FROM {}.{} WHERE device={} AND ts >= ? AND ts <= ?",
        ks, table, uuid_search
    );

    let mut max_ps: PreparedStatement = session.prepare(max_stmt).await?;
    let mut min_ps: PreparedStatement = session.prepare(min_stmt).await?;
    let mut avg_ps: PreparedStatement = session.prepare(avg_stmt).await?;

    max_ps.set_consistency(Consistency::LocalQuorum);
    min_ps.set_consistency(Consistency::LocalQuorum);
    avg_ps.set_consistency(Consistency::LocalQuorum);

    // Retry policy
    max_ps.set_retry_policy(Box::new(DefaultRetryPolicy::new()));
    min_ps.set_retry_policy(Box::new(DefaultRetryPolicy::new()));
    avg_ps.set_retry_policy(Box::new(DefaultRetryPolicy::new()));

    println!();
    println!("\n\n{}", hdr);
    println!(
        "Start querying {} from {} to {}\n",
        uuid_search, start_date, end_date
    );

    // Query data
    if let Some(rows) = session
        .execute(&max_ps, (Timestamp(to_start), Timestamp(to_end)))
        .await?
        .rows
    {
        for row in rows.into_typed::<(f32,)>() {
            let (max,): (f32,) = row?;
            println!("Device {} MAX {}", uuid_search, max);
        }
    }

    if let Some(rows) = session
        .execute(&min_ps, (Timestamp(to_start), Timestamp(to_end)))
        .await?
        .rows
    {
        for row in rows.into_typed::<(f32,)>() {
            let (min,): (f32,) = row?;
            println!("Device {} MIN {}", uuid_search, min);
        }
    }

    if let Some(rows) = session
        .execute(&avg_ps, (Timestamp(to_start), Timestamp(to_end)))
        .await?
        .rows
    {
        for row in rows.into_typed::<(f32,)>() {
            let (avg,): (f32,) = row?;
            println!("Device {} AVG {}", uuid_search, avg);
        }
    }

    println!("{}\n\n", hdr);

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
