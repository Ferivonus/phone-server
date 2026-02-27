use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, UdpSocket};
use std::thread;

fn main() -> std::io::Result<()> {
    // ---------------------------------------------------------
    // 1. HEALTH CHECK SUNUCUSU (Arka Planda Çalışan HTTP/TCP)
    // ---------------------------------------------------------
    // Health check için sabit 8080 portunu kullanıyoruz.
    thread::spawn(|| {
        let tcp_listener = TcpListener::bind("0.0.0.0:8080").expect("TCP Portu açılamadı");
        println!("Health Check sunucusu HTTP 8080 portunda dinleniyor...");

        for stream in tcp_listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buffer = [0; 1024];
                // Gelen isteği oku
                if stream.read(&mut buffer).is_ok() {
                    let request = String::from_utf8_lossy(&buffer);

                    // Eğer gelen istek "/check" ile başlıyorsa OK dön
                    if request.starts_with("GET /check ") {
                        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nOK";
                        let _ = stream.write_all(response.as_bytes());
                    } else {
                        // Başka bir sayfaya girilirse 404 dön
                        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
                        let _ = stream.write_all(response.as_bytes());
                    }
                }
            }
        }
    });

    // ---------------------------------------------------------
    // 2. ANA SİNYALLEŞME SUNUCUSU (Ön Planda Çalışan UDP)
    // ---------------------------------------------------------
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let socket = UdpSocket::bind(&addr)?;
    println!(
        "Aracı sunucu {} üzerinde UDP bağlantılarını dinliyor...",
        addr
    );

    let mut clients = Vec::new();
    let mut buf = [0; 1024];

    loop {
        // İstemcilerden gelen bağlantıları dinle
        let (_amt, src) = socket.recv_from(&mut buf)?;

        if !clients.contains(&src) {
            println!("Yeni kullanıcı bağlandı: {}", src);
            clients.push(src);
        }

        // İki kişi bağlandığında adresleri çapraz olarak gönder
        if clients.len() == 2 {
            println!("İki kullanıcı eşleşti! Adresler takas ediliyor...");
            let peer1 = clients[0];
            let peer2 = clients[1];

            socket.send_to(peer2.to_string().as_bytes(), peer1)?;
            socket.send_to(peer1.to_string().as_bytes(), peer2)?;

            clients.clear();
        }
    }
}
