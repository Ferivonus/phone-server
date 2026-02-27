use futures_util::{SinkExt, StreamExt};
use std::env;
use std::net::SocketAddr; // Adres karşılaştırması için ekledik
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = TcpListener::bind(&addr).await?;
    println!("Eko Korumalı Ses Sunucusu {} üzerinde aktif!", addr);

    // KANAL DEĞİŞİKLİĞİ: Artık (Gönderen_Adresi, Mesaj) şeklinde bir ikili taşıyoruz
    let (tx, _rx) = broadcast::channel::<(SocketAddr, Message)>(1024);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            println!("Yeni bağlantı: {}", peer_addr);

            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket hatası: {}", e);
                    return;
                }
            };

            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            loop {
                tokio::select! {
                    // 1. Bu istemciden gelen veriyi al
                    msg = ws_receiver.next() => {
                        match msg {
                            Some(Ok(msg)) => {
                                if msg.is_text() || msg.is_binary() {
                                    // Mesajı, gönderen kişinin adresiyle birlikte kanala atıyoruz
                                    let _ = tx.send((peer_addr, msg));
                                }
                            }
                            _ => break,
                        }
                    }

                    // 2. Kanaldan gelen veriyi istemciye gönder (FİLTRE BURADA)
                    res = rx.recv() => {
                        if let Ok((sender_addr, msg)) = res {
                            // KRİTİK NOKTA: Eğer mesajı gönderen kişi, şu anki döngüdeki kişi DEĞİLSE gönder
                            if sender_addr != peer_addr {
                                if let Err(_) = ws_sender.send(msg).await {
                                    break;
                                }
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
