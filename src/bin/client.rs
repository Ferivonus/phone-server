use dotenvy::dotenv; // dotenv kütüphanesini dahil ettik
use std::env;
use std::io::{self, Write};
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    // Adım 1: .env dosyasını bul ve içindeki değişkenleri sisteme yükle
    dotenv().ok();

    // Adım 2: SERVER_ADDR değişkenini çek. Eğer dosya yoksa veya değişken eksikse hata ver.
    let server_addr = env::var("SERVER_ADDR")
        .expect("HATA: .env dosyasında SERVER_ADDR bulunamadı! Lütfen kontrol edin.");

    println!("Yapılandırma yüklendi. Hedef Sunucu: {}", server_addr);

    // İşletim sisteminden rastgele bir boş port alıyoruz
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    // Sunucuya kayıt ol
    println!("Sunucuya bağlanılıyor...");
    socket.send_to(b"KAYIT", &server_addr)?;

    // Arkadaşının gelmesini bekle
    let mut buf = [0; 1024];
    println!("Arkadaşının bağlanması bekleniyor...");
    let (amt, _) = socket.recv_from(&mut buf)?;
    let peer_addr = String::from_utf8_lossy(&buf[..amt]).to_string();

    println!("Eşleşme başarılı! Arkadaşının adresi: {}", peer_addr);

    let socket_listener = socket.try_clone()?;

    // --- BURADAN SONRASI ESKİ KOD İLE TAMAMEN AYNI ---
    // (Mesaj dinleme thread'i ve mesaj gönderme döngüsü burada yer alacak)

    thread::spawn(move || {
        let mut buf = [0; 1024];
        loop {
            if let Ok((amt, _src)) = socket_listener.recv_from(&mut buf) {
                let msg = String::from_utf8_lossy(&buf[..amt]);
                if msg != "DELIK_ACMA" {
                    println!("\r[Arkadaşın]: {}", msg);
                    print!("Sen: ");
                    io::stdout().flush().unwrap();
                }
            }
        }
    });

    println!("Bağlantı tüneli açılıyor (Hole Punching)...");
    for _ in 0..3 {
        socket.send_to(b"DELIK_ACMA", &peer_addr)?;
        thread::sleep(Duration::from_millis(500));
    }

    println!("--- Sohbet Başladı! Mesajını yaz ve Enter'a bas. (Çıkmak için 'cikis' yaz) ---");

    loop {
        print!("Sen: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let msg = input.trim();

        if msg.eq_ignore_ascii_case("cikis") {
            println!("Sohbetten çıkıldı.");
            break;
        }

        if !msg.is_empty() {
            socket.send_to(msg.as_bytes(), &peer_addr)?;
        }
    }

    Ok(())
}
