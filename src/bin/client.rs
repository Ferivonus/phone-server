use dotenvy::dotenv;
use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::thread;

fn main() {
    // .env dosyasındaki ayarları yükle
    dotenv().ok();
    let server_addr =
        env::var("SERVER_ADDR").expect("HATA: .env dosyasında SERVER_ADDR bulunamadı!");

    println!("Sunucuya bağlanılıyor: {} ...", server_addr);

    // Sunucuya doğrudan TCP bağlantısı açıyoruz
    let mut stream = match TcpStream::connect(&server_addr) {
        Ok(s) => s,
        Err(e) => {
            println!("Sunucuya bağlanılamadı: {}", e);
            return;
        }
    };

    println!("Bağlantı başarılı! --- Sohbet Başladı --- (Çıkmak için 'cikis' yazın)");

    // Okuma ve yazma işlemlerini aynı anda yapabilmek için bağlantıyı kopyalıyoruz
    let mut stream_clone = stream.try_clone().expect("Bağlantı kopyalanamadı");

    // 1. Gelen mesajları arka planda dinleme (Thread)
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match stream_clone.read(&mut buffer) {
                Ok(0) => {
                    println!("\n[Sistem]: Sunucu bağlantıyı kapattı.");
                    break;
                }
                Ok(size) => {
                    let msg = String::from_utf8_lossy(&buffer[..size]);
                    println!("\r[Arkadaşın]: {}", msg);
                    print!("Sen: ");
                    io::stdout().flush().unwrap();
                }
                Err(_) => break,
            }
        }
    });

    // 2. Mesaj gönderme döngüsü (Ana ekran)
    loop {
        print!("Sen: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let msg = input.trim();

        if msg.eq_ignore_ascii_case("cikis") {
            println!("Sohbetten çıkılıyor...");
            break;
        }

        if !msg.is_empty() {
            // Mesajı sunucuya iletiyoruz, sunucu da arkadaşımıza iletecek
            let _ = stream.write_all(msg.as_bytes());
        }
    }
}
