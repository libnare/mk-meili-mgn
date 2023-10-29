FROM rust:latest AS base
WORKDIR /app

RUN cargo install --config net.git-fetch-with-cli=true cargo-chef

FROM base AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder

COPY --from=planner /app/recipe.json ./
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/mk-meili-mgn ./
CMD ["./mk-meili-mgn"]