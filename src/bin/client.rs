#![cfg(feature = "cpal")]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dotenvy::dotenv;
use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
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
    // --- SES AYARLARI ---
    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("Mikrofon bulunamadı");
    let output_device = host.default_output_device().expect("Hoparlör bulunamadı");

    // Mikrofon config'ini değiştirilebilir (mut) yapıyoruz
    let mut in_config: cpal::StreamConfig = input_device.default_input_config()?.into();
    let out_config: cpal::StreamConfig = output_device.default_output_config()?.into();

    // İŞTE ÇÖZÜM BURADA: Mikrofonun hızını zorla hoparlörün hızına eşitliyoruz!
    in_config.sample_rate = out_config.sample_rate;

    let in_channels = in_config.channels as usize;
    let out_channels = out_config.channels as usize;

    println!(
        "Mikrofon: {} Hz, {} Kanal",
        in_config.sample_rate, in_channels
    );
    println!(
        "Hoparlör: {} Hz, {} Kanal",
        out_config.sample_rate, out_channels
    );

    // Sınırları kaldırıyoruz! (Paket kaybını önlemek için unbounded kullandık)
    let (tx_audio, mut rx_audio) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    // 1. MİKROFONU DİNLE VE GÖNDER
    let input_stream = input_device.build_input_stream(
        &in_config,
        move |data: &[f32], _: &_| {
            let mut mono_data = Vec::with_capacity(data.len() / in_channels);

            // Eğer mikrofon Stereo ise Mono'ya (tek sese) düşürüyoruz
            for frame in data.chunks(in_channels) {
                let sum: f32 = frame.iter().sum();
                mono_data.push(sum / in_channels as f32);
            }

            // Sesi byte'a çevirip sınırsız kanala at
            let bytes: Vec<u8> = mono_data.iter().flat_map(|&f| f.to_le_bytes()).collect();
            let _ = tx_audio.send(bytes);
        },
        |err| eprintln!("Mikrofon hatası: {}", err),
        None,
    )?;
    input_stream.play()?;

    tokio::spawn(async move {
        while let Some(audio_bytes) = rx_audio.recv().await {
            let _ = sender.send(Message::Binary(audio_bytes.into())).await;
        }
    });

    // 2. GELEN SESİ OYNAT
    let audio_queue = Arc::new(std::sync::Mutex::new(VecDeque::<f32>::new()));
    let audio_queue_clone = Arc::clone(&audio_queue);

    let mut is_buffering = true;

    let output_stream = output_device.build_output_stream(
        &out_config,
        move |data: &mut [f32], _: &_| {
            let mut queue = audio_queue_clone.lock().unwrap();

            // Jitter Buffer (Tampon) Mantığı:
            // Sesin kesilmemesi için kuyrukta en az 1000 paket birikmesini bekliyoruz.
            if is_buffering && queue.len() > 1000 {
                is_buffering = false;
            } else if !is_buffering && queue.is_empty() {
                is_buffering = true;
            }

            // Gelen Mono sesi, hoparlörün kanallarına (Stereo) dağıtıyoruz
            for frame in data.chunks_mut(out_channels) {
                let sample = if !is_buffering {
                    queue.pop_front().unwrap_or(0.0)
                } else {
                    0.0 // Tampon dolana kadar sessizlik çal
                };

                // Aynı sesi sağ ve sol kulaklığa kopyala
                for out_sample in frame.iter_mut() {
                    *out_sample = sample;
                }
            }
        },
        |err| eprintln!("Hoparlör hatası: {}", err),
        None,
    )?;
    output_stream.play()?;

    println!("--- Karşılıklı sesli konuşma başladı! Kapatmak için CTRL+C ---");

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Binary(bin) = msg {
            let mut queue = audio_queue.lock().unwrap();
            for chunk in bin.chunks_exact(4) {
                let f = f32::from_le_bytes(chunk.try_into().unwrap());
                queue.push_back(f);
            }
        }
    }

    Ok(())
}
