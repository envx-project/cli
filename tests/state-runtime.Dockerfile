FROM ubuntu:26.04
RUN apt-get update && apt-get install -y --no-install-recommends libssl3t64 ca-certificates python3 && rm -rf /var/lib/apt/lists/*
ENV HOME=/tmp/envx-test-home
