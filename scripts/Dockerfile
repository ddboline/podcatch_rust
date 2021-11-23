FROM rust:alpine3.14 as buildstage

RUN apk add musl-dev && mkdir -p /build/podcatch_rust

ADD Cargo.toml /build/podcatch_rust/
COPY src /build/podcatch_rust/src
COPY migrations /build/podcatch_rust/migrations

RUN mkdir -p /podcatch_rust && \
    cd /build/podcatch_rust && \
    cargo install --path=. --root=/podcatch_rust

FROM alpine:3.14

COPY --from=buildstage /podcatch_rust/bin/* /usr/bin/

STOPSIGNAL SIGTERM

CMD ["/usr/bin/podcatch-rust"]
