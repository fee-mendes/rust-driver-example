FROM rust:1.84-bullseye

WORKDIR /usr/src
COPY ./check_cluster_healthy .
RUN git clone https://github.com/fee-mendes/rust-driver-example.git
WORKDIR /usr/src/rust-driver-example

RUN cargo fetch
RUN cargo build --workspace
RUN chmod +x /usr/src/check_cluster_healthy

CMD ["/usr/src/check_cluster_healthy"]
