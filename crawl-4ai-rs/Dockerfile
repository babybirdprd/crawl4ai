# Use official Rust image
FROM rust:latest as builder

# Install dependencies for building and running Chrome
RUN apt-get update && apt-get install -y \
    wget \
    gnupg \
    libnss3 \
    libxss1 \
    libasound2 \
    libatk-bridge2.0-0 \
    libgtk-3-0 \
    libgbm1 \
    ca-certificates \
    fonts-liberation \
    libappindicator3-1 \
    lsb-release \
    xdg-utils \
    && rm -rf /var/lib/apt/lists/*

# Install Chrome (or Chromium)
# Here we install chromium
RUN apt-get update && apt-get install -y chromium && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build the project (release mode)
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies for Chromium
RUN apt-get update && apt-get install -y \
    chromium \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/crawl-4ai-rs .

# Set env var for Chrome executable
ENV CHROME_EXECUTABLE=/usr/bin/chromium

# Command to run (placeholder, as this is a library primarily, but if there's a binary...)
# Assuming this is a library, but if we want to run tests or a demo:
CMD ["./crawl-4ai-rs"]
