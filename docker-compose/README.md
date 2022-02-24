# rust-driver-example - Docker compose

This folder contains a simple docker-compose file that can be used to quickly functional test the rust application.
Currently, it spins up a 3 node Scylla cluster. TODO: Add an application container with the actual Rust application :-)

- Simply run `docker-compose up -d` to get started.
- You may then run `docker-compose logs -f` to see the cluster coming up
- After the initial bootstrap process, you may run `nodetool status` on one of the nodes to confirm the cluster is up and available. For example:

```shell
$ docker exec -it scylladb-01 nodetool status
Using /etc/scylla/scylla.yaml as the config file
Datacenter: datacenter1
=======================
Status=Up/Down
|/ State=Normal/Leaving/Joining/Moving
--  Address     Load       Tokens       Owns    Host ID                               Rack
UN  172.19.0.3  802.74 KB  256          ?       825b7556-355c-41b7-b0ac-f40086723b3e  rack1
UN  172.19.0.2  620.18 KB  256          ?       c95fe5d0-706f-4f05-aaca-b29facecbab6  rack1
UN  172.19.0.4  797.4 KB   256          ?       98a3687f-c517-4ba8-acf9-bb22f219da5e  rack1
```

Before you run the actual application, it is also recommended to set-up the [ScyllaDB Monitoring Stack](https://monitoring.docs.scylladb.com/stable/install/monitoring_stack.html) to have live insights as you move forward.
