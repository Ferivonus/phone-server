# --- 1. AŞAMA: Derleme (Builder) ---
FROM rust:latest AS builder

WORKDIR /usr/src/phone-server
COPY . .

# Sunucu için gereksiz ses özelliklerini (cpal) devre dışı bırakarak derliyoruz
RUN cargo build --release --bin server --no-default-features

# --- 2. AŞAMA: Çalıştırma (Runtime) ---
FROM debian:bookworm-slim

# Eksik olan SSL ve sertifika kütüphanelerini yüklüyoruz
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Derlenen dosyayı kopyala
COPY --from=builder /usr/src/phone-server/target/release/server .

# Railway PORT ayarı
ENV PORT=8000

CMD ["./server"]