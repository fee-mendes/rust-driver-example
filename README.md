# rust-driver-example

This repo contains 3 simple Rust programs that are meant to demonstrate [ScyllaDB's Rust driver capabilities](https://github.com/scylladb/scylla-rust-driver/).

Ensure to follow the [Driver documentation](https://cvybhu.github.io/scyllabook/index.html) in order to get familiarity with it.

All 3 programs are related to each other. The purpose of these programs is to demonstrate how to use the various capabilities around Scylla Rust driver by simulating a simple IOT workload scenario.

- **metric-collector** is a program that demonstrates how to programatically create keyspaces/tables and insert data to a table. It requires 4 parameters:

```shell
$ cargo run
usage: <host> <dc> <ks> <table>
```

- **metric_reader** demonstrates how to perform a full table scan and, as such, read the data written using the previous example in the best performant way. Similarly as the previous program, it requires the same 4 parameters:

```shell
$ cargo run
usage: <host> <dc> <ks> <table>
```

- **uuid_finder** is a program which demonstrates how to use the MAX, MIN and AVG functions for a specific device uuid while filtering by a given clustering key via inequality clauses. It requires 6 parameters:

```shell
$ cargo run
usage: <host> <dc> <ks> <table> <uuid> <start_date> <end_date>

$ cargo run 172.1.7.0.2 datacenter1 ks tbl ab914a61-47d9-5c89-99b7-cb4b5acb3d31 '2020-01-02 01:00:00' '2021-01-01 00:00:00'
```
