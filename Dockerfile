# --- 1. AŞAMA: Derleme (Builder) ---
# Rust'ın en GÜNCEL sürümünü alıyoruz (Hatayı çözen kısım burası)
FROM rust:latest AS builder

# Konteyner içinde kendimize bir çalışma klasörü yaratıyoruz
WORKDIR /usr/src/phone-server

# Bilgisayarındaki tüm proje dosyalarını konteynerin içine kopyalıyoruz
COPY . .

# Sadece "server" kodumuzu yayınlanmaya hazır (release) formatta derliyoruz.
RUN cargo build --release --bin server

# --- 2. AŞAMA: Çalıştırma (Runtime) ---
# Çalıştırmak için çok daha hafif, boş bir Linux (Debian) imajı seçiyoruz
FROM debian:bookworm-slim

# Yeni imajda bir çalışma klasörü yaratıyoruz
WORKDIR /app

# 1. aşamada derlenen hazır "server" programını, bu hafif imajın içine kopyalıyoruz
COPY --from=builder /usr/src/phone-server/target/release/server .

# Railway'in dinamik atayacağı portları desteklemesi için PORT değişkeni hazırlıyoruz
ENV PORT=8000

# Konteyner ayağa kalktığında çalıştırılacak komut
CMD ["./server"]