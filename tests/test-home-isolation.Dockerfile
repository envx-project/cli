FROM ubuntu:26.04
RUN apt-get update && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev ca-certificates python3 && rm -rf /var/lib/apt/lists/*
