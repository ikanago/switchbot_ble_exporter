FROM lukemathwalker/cargo-chef:0.1.67-rust-1.79.0-bookworm AS chef

FROM chef AS planner
WORKDIR /plan
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /build
COPY --from=planner /plan/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    bluez \
    dbus \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# setup bluetooth permissions
COPY ./bluezuser.conf /etc/dbus-1/system.d/
RUN useradd -m bluezuser  && adduser bluezuser sudo  && passwd -d bluezuser
USER bluezuser

# setup startup script
COPY entrypoint.sh .
CMD ./entrypoint.sh
