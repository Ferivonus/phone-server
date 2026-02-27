use dotenvy::dotenv;
use futures_util::{SinkExt, StreamExt};
use std::env;
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mut server_addr = env::var("SERVER_ADDR").expect("HATA: SERVER_ADDR bulunamadı!");

    // Kullanıcı unutsa bile URL'yi Railway'e uygun hale getirelim (wss://)
    if !server_addr.starts_with("ws://") && !server_addr.starts_with("wss://") {
        if server_addr.contains("railway.app") || server_addr.contains("fly.dev") {
            server_addr = format!("wss://{}", server_addr); // Canlı sunucu için güvenli websocket
        } else {
            server_addr = format!("ws://{}", server_addr); // Yerel test için normal websocket
        }
    }

    println!("WebSocket Sunucusuna bağlanılıyor: {} ...", server_addr);

    // Sunucuya asenkron olarak bağlan
    let (ws_stream, _) = match connect_async(&server_addr).await {
        Ok(s) => s,
        Err(e) => {
            println!("Sunucuya bağlanılamadı: {}", e);
            return;
        }
    };

    println!("Bağlantı başarılı! --- Sohbet Başladı --- (Çıkmak için 'cikis' yazın)");

    let (mut sender, mut receiver) = ws_stream.split();

    // 1. Gelen mesajları arka planda dinleme
    tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if msg.is_text() {
                println!("\r[Arkadaşın]: {}", msg.to_text().unwrap());
                print!("Sen: ");
                io::stdout().flush().unwrap();
            }
        }
    });

    // 2. Klavyeden mesaj okuyup sunucuya gönderme
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        print!("Sen: ");
        io::stdout().flush().unwrap();
        line.clear();

        let bytes_read = reader.read_line(&mut line).await.unwrap();
        if bytes_read == 0 {
            break;
        } // EOF

        let msg = line.trim();
        if msg.eq_ignore_ascii_case("cikis") {
            println!("Sohbetten çıkılıyor...");
            break;
        }

        if !msg.is_empty() {
            // Yazdığın mesajı WebSocket paketi olarak sunucuya yolla
            let _ = sender.send(Message::Text(msg.to_string().into())).await;
        }
    }
}
