mod client;
mod manifest;
mod render;
mod wavelet;

use client::HlsClient;
use pancurses::{curs_set, echo, endwin, initscr, noecho, resize_term, Input};
use render::{render, Packet};
use rodio::source::Buffered;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::ops::Div;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::{collections::VecDeque, io::Cursor};

static WIDTH: usize = 256;
static HEIGHT: usize = 64;
static URL: &str = "https://rd-trtradyo3.medya.trt.com.tr/master_128.m3u8";

fn main() {
  if cfg!(debug_assertions) {
    simple_logging::log_to_file("log", log::LevelFilter::Debug).unwrap();
  } else {
    simple_logging::log_to_file("log", log::LevelFilter::Info).unwrap();
  }

  type P = (Vec<f32>, Buffered<Decoder<Cursor<Box<[u8]>>>>, String);
  let (p_sender, p_receive): (Sender<P>, Receiver<P>) = mpsc::channel();

  /* Thread 1 & 2, Spectral Processing & Hls Subscription */
  let t12_run = Arc::new(Mutex::new(true));
  let t12_run_c = Arc::clone(&t12_run);
  
  let t12_handle = thread::spawn(move || {
    let mut buff = VecDeque::new();
    let mut client = HlsClient::new(URL).expect("Failed to create client");
    while let Some(source) = client.next() {
      /* Exit check. */
      if !(*t12_run_c.lock().unwrap()) {
        client.stop();
        break;
      }

      /* Fill the buffer */
      if buff.len() < 1 {
        let d = 6.0f32; // source.total_duration().unwrap();
        let data: Vec<f32> = source.0.clone().convert_samples().collect();
        let spectrograph = wavelet::morlet_transform(
          &data,
          20_000.0 / (WIDTH as f32),
          d, //.as_secs_f32(),
          WIDTH,
          HEIGHT,
        );
        buff.push_back((spectrograph, source));
      }

      /* Send from buffer. */
      if let Some((spectrograph, source)) = buff.pop_back() {
        p_sender.send((spectrograph, source.0, source.1)).unwrap();
      }
    }
    client.stop();
  });
  
  let mut player_state: Packet = Packet::new(1.0, false);

  let window = initscr();
  window.clear();
  window.refresh();
  window.keypad(true);
  window.nodelay(true);
  curs_set(0);
  noecho();

  let (_stream, handle) = OutputStream::try_default().unwrap();
  let sink = Sink::try_new(&handle).unwrap();
  sink.set_volume(1.0);

  let tick_period = std::time::Duration::from_millis(10);
  let mut data_iter = p_receive.iter();
  'main: loop {
    if let Some(data) = data_iter.next() {
      log::log!(log::Level::Info, "Playing Track: {}", data.2);

      /* Set the data up. */
      sink.append(data.1);
      let buffer_raw: Vec<_> = data.0.as_slice().chunks(WIDTH).collect();
      let spectrograph = buffer_raw.as_slice();

      /* Segment loop until audio finishes. */
      let mut last_time = std::time::Instant::now();
      'frame: loop {
        /* Data */
        let t: f32 = sink.get_pos().as_secs_f32().clamp(0.001, f32::MAX);
        let idx: usize = ((t * HEIGHT as f32) as usize) % HEIGHT;
        let data: Vec<f32> = spectrograph[idx].to_vec();

        /* Events */
        match window.getch() {
          Some(Input::KeyResize) => {
            resize_term(0, 0);
          }
          Some(Input::Character(c)) => {
            match c {
              'q' => break 'main,
              '[' => {
                player_state.volume_down();
                sink.set_volume(player_state.volume);
              },
              ']' => {
                player_state.volume_up();
                sink.set_volume(player_state.volume);
              },
              '\n' => {
                player_state.mute = !player_state.mute;
                match player_state.mute {
                    true => sink.set_volume(0.0f32),
                    false => sink.set_volume(player_state.volume),
                }
              }
              _ => (),
            }
          }
          Some(Input::KeyDC) => break 'main,
          _ => (),
        }

        /* Render */
        window.clear();
        render(&window, &data, 1.0, -1.0, &player_state);
        window.draw_box(0, 0);
        window.refresh();

        /* Tick */
        while std::time::Instant::now() - last_time < tick_period {
          std::thread::sleep(tick_period.div(2));
          if sink.empty() {
            break 'frame;
          }
        }
        last_time = std::time::Instant::now();
      } // 'frame loop
      continue;
    }
    break 'main;
  } // 'main loop

  if let Ok(mut run_guard) = t12_run.lock() {
    *run_guard = false;
  }
  t12_handle.join().unwrap();

  echo();
  curs_set(1);
  endwin();
}
