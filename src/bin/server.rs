use futures_util::{SinkExt, StreamExt};
use std::env;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Asenkron TCP Dinleyici başlat
    let listener = TcpListener::bind(&addr).await.expect("Port açılamadı!");
    println!("WebSocket Sunucusu {} portunda dinliyor...", addr);

    // Tüm istemcilere aynı anda mesaj gönderebilmek için bir yayın (broadcast) kanalı açıyoruz
    let (tx, _rx) = broadcast::channel(100);

    // Sürekli olarak yeni bağlantıları bekle
    while let Ok((stream, peer_addr)) = listener.accept().await {
        let tx = tx.clone();
        let mut rx = tx.subscribe(); // Bu istemci için bir alıcı oluştur

        // Her kullanıcı için arka planda yeni bir asenkron görev (task) başlat
        tokio::spawn(async move {
            println!("Yeni TCP bağlantısı yakalandı: {}", peer_addr);

            // Standart TCP'yi WebSocket'e yükseltiyoruz (İşte sihir burada!)
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    println!("WebSocket hatası: {}", e);
                    return;
                }
            };

            println!("Kullanıcı WebSocket'e yükseltildi: {}", peer_addr);

            // WebSocket'i okuyan ve yazan olarak ikiye bölüyoruz
            let (mut sender, mut receiver) = ws_stream.split();

            loop {
                // select! makrosu: Aynı anda hem istemciden gelen mesajı hem de
                // diğer odalardan gelen broadcast mesajlarını dinlememizi sağlar
                tokio::select! {
                    msg = receiver.next() => {
                        match msg {
                            Some(Ok(msg)) => {
                                if msg.is_text() {
                                    let text = msg.to_text().unwrap();
                                    // Mesajı diğer herkese gönderilmesi için kanala at
                                    let broadcast_msg = format!("{}: {}", peer_addr, text);
                                    let _ = tx.send(broadcast_msg);
                                }
                            }
                            _ => break, // Kullanıcı bağlantıyı kopardı
                        }
                    }
                    broadcast_msg = rx.recv() => {
                        if let Ok(msg) = broadcast_msg {
                            // Kendi yazdığımız mesajı kendimize geri göndermeyelim
                            if !msg.starts_with(&peer_addr.to_string()) {
                                // IP kısmını ayırıp sadece metni gönderiyoruz
                                let clean_msg = msg.splitn(2, ": ").nth(1).unwrap_or(&msg);
                                let ws_msg = tokio_tungstenite::tungstenite::protocol::Message::Text(clean_msg.to_string().into());
                                let _ = sender.send(ws_msg).await;
                            }
                        }
                    }
                }
            }
            println!("Kullanıcı ayrıldı: {}", peer_addr);
        });
    }
}
