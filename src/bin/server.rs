use futures_util::{SinkExt, StreamExt};
use std::env;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Railway'in atadığı portu alıyoruz, yoksa 8080 kullanıyoruz.
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // TCP Dinleyicisini başlatıyoruz
    let listener = TcpListener::bind(&addr).await?;
    println!("Ses ve Mesaj Sunucusu {} üzerinde aktif!", addr);

    // Broadcast kanalı: Sunucuya gelen her şeyi bağlı tüm cihazlara dağıtır.
    // Kapasiteyi 1024 yaptık çünkü ses paketleri çok hızlı ve yoğun gelir.
    let (tx, _rx) = broadcast::channel::<Message>(1024);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            println!("Yeni bağlantı: {}", peer_addr);

            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket el sıkışma hatası ({}): {}", peer_addr, e);
                    return;
                }
            };

            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            loop {
                tokio::select! {
                    // 1. Bu istemciden gelen veriyi al (Ses veya Metin)
                    msg = ws_receiver.next() => {
                        match msg {
                            Some(Ok(msg)) => {
                                // Gelen veri boş değilse, gönderen hariç herkese yayınla
                                if msg.is_text() || msg.is_binary() {
                                    // Mesajı kanala atıyoruz (Broadcast)
                                    // Not: Basitlik adına gönderen kişiye de geri gider,
                                    // istemci kodu kendi sesini çalmamak için bunu filtreler.
                                    let _ = tx.send(msg);
                                }
                            }
                            _ => break, // Bağlantı koptu
                        }
                    }

                    // 2. Diğer kullanıcılardan gelen verileri bu istemciye gönder
                    res = rx.recv() => {
                        if let Ok(msg) = res {
                            if let Err(e) = ws_sender.send(msg).await {
                                eprintln!("Veri gönderim hatası: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            println!("Kullanıcı ayrıldı: {}", peer_addr);
        });
    }

    Ok(())
}
