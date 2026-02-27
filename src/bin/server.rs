use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Railway'in vereceği portu dinliyoruz, yoksa 8080 kullanıyoruz.
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // TCP Dinleyicisini başlatıyoruz
    let listener = TcpListener::bind(&addr).expect("Port açılamadı!");
    println!(
        "Sohbet Sunucusu {} portunda TCP üzerinden dinliyor...",
        addr
    );

    // Bağlı kullanıcıları tutacağımız güvenli liste (Thread'ler arası paylaşım için Arc ve Mutex kullanıyoruz)
    let clients: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));

    // Sürekli olarak yeni bağlantıları bekle
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients_clone = Arc::clone(&clients);
                // Her yeni kullanıcı için arka planda bağımsız bir işlem (thread) başlat
                thread::spawn(move || {
                    handle_client(stream, clients_clone);
                });
            }
            Err(e) => println!("Bağlantı hatası: {}", e),
        }
    }
}

// Her bir kullanıcının mesajlarını dinleyen fonksiyon
fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<Vec<TcpStream>>>) {
    let peer_addr = stream.peer_addr().unwrap().to_string();
    println!("Yeni kullanıcı bağlandı: {}", peer_addr);

    // Yeni kullanıcıyı listeye ekle
    {
        let mut clients_lock = clients.lock().unwrap();
        clients_lock.push(stream.try_clone().unwrap());
    }

    let mut buffer = [0; 1024];

    // Bu kullanıcıdan gelen mesajları sürekli dinle
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Okunan veri 0 ise, kullanıcı bağlantıyı kopardı demektir
                println!("Kullanıcı ayrıldı: {}", peer_addr);
                break;
            }
            Ok(size) => {
                // Mesajı al
                let msg = String::from_utf8_lossy(&buffer[..size]).to_string();

                // Mesajı, gönderen kişi hariç diğer tüm kullanıcılara ilet (Broadcast)
                let mut clients_lock = clients.lock().unwrap();
                for client in clients_lock.iter_mut() {
                    // Kendi adresimiz değilse mesajı gönder
                    if client.peer_addr().unwrap().to_string() != peer_addr {
                        let _ = client.write_all(msg.as_bytes());
                    }
                }
            }
            Err(_) => {
                println!("Bağlantı aniden koptu: {}", peer_addr);
                break;
            }
        }
    }

    // Kullanıcı çıkış yaptığında onu listeden temizle
    let mut clients_lock = clients.lock().unwrap();
    clients_lock.retain(|c| c.peer_addr().unwrap().to_string() != peer_addr);
}
