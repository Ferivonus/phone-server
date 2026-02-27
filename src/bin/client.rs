use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dotenvy::dotenv;
use futures_util::{SinkExt, StreamExt};
use std::env;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let server_addr = env::var("SERVER_ADDR").expect("HATA: SERVER_ADDR bulunamadı!");
    let url = if server_addr.contains("railway.app") {
        format!("wss://{}", server_addr)
    } else {
        format!("ws://{}", server_addr)
    };

    println!("Ses Sunucusuna bağlanılıyor: {} ...", url);
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut sender, mut receiver) = ws_stream.split();

    // --- SES AYARLARI ---
    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("Mikrofon bulunamadı");
    let output_device = host.default_output_device().expect("Hoparlör bulunamadı");
    let config: cpal::StreamConfig = input_device.default_input_config()?.into();

    println!("Ses ayarları yapıldı: {} Hz", config.sample_rate);

    // 1. MİKROFONU DİNLE VE GÖNDER
    let (tx_audio, mut rx_audio) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
    let input_stream = input_device.build_input_stream(
        &config,
        move |data: &[f32], _: &_| {
            // Sesi byte dizisine çevirip kanala at
            let bytes: Vec<u8> = data.iter().flat_map(|&f| f.to_le_bytes()).collect();
            let _ = tx_audio.try_send(bytes);
        },
        |err| eprintln!("Mikrofon hatası: {}", err),
        None,
    )?;
    input_stream.play()?;

    // Mikrofon verilerini WebSocket'e basan görev
    tokio::spawn(async move {
        while let Some(audio_bytes) = rx_audio.recv().await {
            let _ = sender.send(Message::Binary(audio_bytes.into())).await;
        }
    });

    // 2. GELEN SESİ OYNAT
    // Paylaşılan bir buffer (kuyruk) oluşturuyoruz
    let audio_queue = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
    let audio_queue_clone = Arc::clone(&audio_queue);

    let output_stream = output_device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &_| {
            let mut queue = audio_queue_clone.lock().unwrap();
            for sample in data.iter_mut() {
                // Kuyrukta ses varsa hoparlöre ver, yoksa sessizlik (0.0)
                *sample = if !queue.is_empty() {
                    queue.remove(0)
                } else {
                    0.0
                };
            }
        },
        |err| eprintln!("Hoparlör hatası: {}", err),
        None,
    )?;
    output_stream.play()?;

    println!("--- Karşılıklı sesli konuşma başladı! Kapatmak için CTRL+C ---");

    // WebSocket'ten gelen sesleri alıp oynatma kuyruğuna ekle
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Binary(bin) = msg {
            let mut queue = audio_queue.lock().unwrap();
            // Byte'ları tekrar f32 ses verisine çevir
            for chunk in bin.chunks_exact(4) {
                let f = f32::from_le_bytes(chunk.try_into().unwrap());
                queue.push(f);
            }
        }
    }

    Ok(())
}
