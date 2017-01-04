FROM ubuntu:16.04

COPY emscripten.sh /
RUN bash /emscripten.sh 1.37.1

RUN apt-get install --no-install-recommends -y \
    ca-certificates \
    gcc \
    libc6-dev \
    nodejs \
    python

ENV NODE=/usr/bin/nodejs
