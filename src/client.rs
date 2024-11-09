use reqwest::blocking::Client;
use reqwest::Url;
use rodio::{source::Buffered, Decoder, Source};
use std::{
  collections::VecDeque,
  io::Cursor,
  sync::{Arc, Mutex},
  thread::JoinHandle,
};

use crate::manifest::HlsManifest;

pub struct HlsClient {
  idx: usize,
  uri: Url,
  run: Arc<Mutex<bool>>,
  client: reqwest::blocking::Client,
  handle: Option<JoinHandle<()>>,
  pub tracks: Arc<Mutex<VecDeque<(String, f32)>>>,
}

impl Drop for HlsClient {
  fn drop(&mut self) {
    if let Ok(mut run_guard) = self.run.lock() {
      *run_guard = false;
    }

    if let Some(handle) = self.handle.take() {
      if handle.join().is_err() {
        eprintln!("Failed to join the thread.");
      }
    }
  }
}

impl HlsClient {
  pub fn stop(&mut self) {
    if let Ok(mut run_guard) = self.run.lock() {
      *run_guard = false;
    }
    match self.handle.take() {
      Some(handle) => {
        if handle.join().is_err() {
          eprintln!("Failed to join the thread.");
        }
      }
      None => {}
    }
  }

  pub fn new(url: &str) -> Option<Self> {
    let run = Arc::new(Mutex::new(true));
    let tracks: Arc<Mutex<VecDeque<(String, f32)>>> =
      Arc::new(Mutex::new(VecDeque::with_capacity(256)));
    let url_clone = url.to_string();
    let run_clone = Arc::clone(&run);
    let tracks_clone = Arc::clone(&tracks);

    let handle = Some(std::thread::spawn(move || {
      let client = Client::new();
      while *run_clone.lock().unwrap() {
        let manifest = HlsManifest::new(&url_clone, &client).unwrap();
        for (name, length) in manifest.tracks {
          if let Ok(mut tracks_guard) = tracks_clone.lock() {
            if !tracks_guard.iter().any(|(n, _)| n.to_string() == name) {
              tracks_guard.push_back((name, length));
            }
          }
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
      }
    }));

    let mut parsed_url = Url::parse(url).unwrap();
    parsed_url.set_path("");

    return Some(HlsClient {
      idx: 0,
      run: run,
      uri: parsed_url,
      client: Client::new(),
      handle: handle,
      tracks: tracks,
    });
  }
}

impl Iterator for HlsClient {
  type Item = (Buffered<Decoder<Cursor<Box<[u8]>>>>, String);

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      std::thread::sleep(std::time::Duration::from_millis(200));
      let tracks_guard = self.tracks.lock().unwrap();
      if self.idx < tracks_guard.len() {
        let track = tracks_guard.get(self.idx).unwrap();
        self.idx += 1;

        /* Process and return the element */
        let track_url = self.uri.join(&track.0).unwrap();
        let data = self
          .client
          .get(track_url)
          .send()
          .ok()
          .unwrap()
          .bytes()
          .unwrap()
          .to_vec()
          .into_boxed_slice();
        let file = Cursor::new(data);
        return Some((Decoder::new(file).unwrap().buffered(), track.0.clone()));
      }
    }
  }
}
