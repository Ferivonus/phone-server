use std::env;
use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    // Railway bize hangi portu verirse onu kullanmalıyız.
    // Eğer yerel bilgisayarda çalıştırıyorsak varsayılan olarak 8000 kullanır.
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    let socket = UdpSocket::bind(&addr)?;
    println!("Aracı sunucu {} üzerinde UDP bağlantılarını dinliyor...", addr);

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

            // 1. kişiye 2. kişinin adresini yolla
            socket.send_to(peer2.to_string().as_bytes(), peer1)?;
            // 2. kişiye 1. kişinin adresini yolla
            socket.send_to(peer1.to_string().as_bytes(), peer2)?;

            // Yeni kişilerin bağlanabilmesi için listeyi temizle
            clients.clear();
        }
    }
}