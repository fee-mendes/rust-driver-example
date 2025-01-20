# rust-driver-example

This repo contains 3 simple Rust programs that are meant to demonstrate [ScyllaDB's Rust driver capabilities](https://github.com/scylladb/scylla-rust-driver/).

Minimum requirements:
- Linux, MacOS or Windows (Windows might require Docker network tuning)
- Docker installed and setup
- Quadcore CPU
- 2 GB of memory available

Setup: 

Clone the repo containing the sample app and start the ScyllaDB docker image:

```
$ git clone https://github.com/fee-mendes/rust-driver-example/
$ cd rust-driver-example/docker-compose
$ docker compose up -d
$ docker exec -it rust-app /bin/bash
```

On successful startup, you will see the following:
```
Creating scylladb-02 ... done
Creating rust-app    ... done
Creating scylladb-01 ... done
Creating scylladb-03 ... done
```
To check the status, run:
```
$ docker-compose logs -f rust-app
```
Your Rust environment is now configured to use ScyllaDB. 

Run the metric-collector sample app:
```
root@rust-app:/usr/src/rust-driver-example# cargo run --bin metric-collector 172.19.0.2 datacenter1
```
You should see data generation and then:
```
Queries requested: 86402
Iter queries requested: 0
Errors occured: 0
Iter errors occured: 0
Average latency: 73
99.9 latency percentile: 2
```
To view the schema anytime, open a new terminal:
```
$ cd docker-compose
$ docker exec -it scylladb-01 cqlsh -e 'DESC SCHEMA;'
```

Refer to the [Driver documentation](https://cvybhu.github.io/scyllabook/index.html) in order to get familiarity with it.

All 3 programs are related to each other. The purpose of these programs is to demonstrate how to use the various capabilities around Scylla Rust driver by simulating a simple IOT workload scenario.

The keyspace `iot` will automatically be created with `replication_factor: 1`. After, the `device` table will be used to populate our data.

- **metric-collector** is a program that demonstrates how to programatically create keyspaces/tables and insert data to a table. By default it will connect to a host `127.0.0.1` and will assume the local datacenter is set to `datacenter1`. To override this behavior, run it as: 

```shell
$ cargo run --bin metric-collector <host> <dc> 
usage: <host> <dc> 
```

For example:
```
cargo run --bin metric-collector 172.19.0.2 datacenter1
```

After the **metric-collector** script is run, it will automatically create a KEYSPACE and TABLE under the following structure:

```
CREATE KEYSPACE iot WITH replication = {'class': 'NetworkTopologyStrategy', 'datacenter1': '1'}  AND durable_writes = true;

CREATE TABLE iot.device (
    device uuid,
    ts timestamp,
    temperature float,
    PRIMARY KEY (device, ts)
) WITH CLUSTERING ORDER BY (ts ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'TimeWindowCompactionStrategy', 'compaction_window_size': '3'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND dclocal_read_repair_chance = 0.0
    AND default_time_to_live = 2592000
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND read_repair_chance = 0.0
    AND speculative_retry = '99.0PERCENTILE';
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
