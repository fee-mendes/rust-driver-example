# rust-driver-example

This repo contains 3 simple Rust programs that are meant to demonstrate [ScyllaDB's Rust driver capabilities](https://github.com/scylladb/scylla-rust-driver/).

Ensure to follow the [Driver documentation](https://cvybhu.github.io/scyllabook/index.html) in order to get familiarity with it.

All 3 programs are related to each other. The purpose of these programs is to demonstrate how to use the various capabilities around Scylla Rust driver by simulating a simple IOT workload scenario.

The keyspace `iot` will automatically be created with `replication_factor: 1`. After, the `device` table will be used to populate our data.

- **metric-collector** is a program that demonstrates how to programatically create keyspaces/tables and insert data to a table. By default it will connect to a host `127.0.0.1` and will assume the local datacenter is set to `datacenter1`. To override this behavior, run it as: 

```shell
$ cargo run --bin metric-collector <host> <dc> 
usage: <host> <dc> 
```

- **metric_reader** demonstrates how to perform a full table scan and, as such, read the data written using the previous example in the best performant way. Similarly as the previous program, the contact point IP and datacenter name can be overriden as:

```shell
$ cargo run --bin metric-reader
usage: <host> <dc>
```

- **uuid_finder** is a program which demonstrates how to use the MAX, MIN and AVG functions for a specific device uuid while filtering by a given clustering key via inequality clauses. The following parameters are accepted and are optional: 

`<uuid>`  - The device UUID you want to query for (defaults to `ab914a61-47d9-5c89-99b7-cb4b5acb3d31`)
`<start>` - The start time to retrieve the given device metrics in `YYYY-mm-dd HH:MM:ss` format. (defaults to `2020-01-01 00:00:00`)
`<end>`   - Same as `<start>`, but up to which date to query (defaults to `2020-01-04 00:00:00`)
`<host>`  - Defaults to `127.0.0.1`
`<dc>`    - Defaults to `datacenter1`

For example, to query for metrics of a given device within the period of 23 hours you could use:

```shell
$ cargo run --bin uuid_finder 42e47eeb-cd68-5a75-9146-72904a902425 '2020-01-01 00:00:00' '2020-01-01 23:00:00'
usage: <uuid> <start> <end> <host> <dc>
```
