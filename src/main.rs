mod render;
mod wavelet;

use clap::Parser;
use pancurses::{curs_set, echo, endwin, initscr, noecho, resize_term, Input};
use rodio::Source;
use std::ops::Div;

/// CLI tool to play sound streams.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  /// Path to a track container, mp3, etc.
  #[arg(short, long)]
  path: std::path::PathBuf,

  /// Amount of frequencies to display.
  #[arg(short, long, default_value_t = 8)]
  width: usize,

  /// Definition of the animation played.
  #[arg(short, long, default_value_t = 16)]
  height: usize,
}

fn main() {
  /* Setup Curses & Rodio */
  let args = Args::parse();
  let window = initscr();
  window.clear();
  window.refresh();
  window.keypad(true);
  window.nodelay(true);
  curs_set(0);
  noecho();

  let file = std::fs::File::open("music-u.mp3").unwrap();
  let source = rodio::Decoder::new(file).unwrap().buffered();
  let duration = source.total_duration().unwrap();
  let source_data: Vec<f32> = source.clone().convert_samples().collect();
  let d_width: usize = args.width;
  let d_height: usize = args.height;
  let spectrograph_raw = wavelet::morlet_transform(
    &source_data,
    20_000.0 / (d_width as f32),
    duration.as_secs_f32(),
    d_width,
    d_height,
  );
  let spectrograph_base: Vec<_> = spectrograph_raw.as_slice().chunks(d_width).collect();
  let spectrograph = spectrograph_base.as_slice();

  let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
  let sink = rodio::Sink::try_new(&handle).unwrap();
  sink.append(source);
  sink.set_volume(0.2);

  let tick_period = std::time::Duration::new(0, 50_000_000);
  let mut last_time = std::time::Instant::now();
  'main_loop: loop {
    /* Data Input */
    let t: f32 = sink.get_pos().as_secs_f32().clamp(0.001, f32::MAX) / duration.as_secs_f32();
    let idx: usize = ((t * d_height as f32) as usize) % d_height;
    let data: Vec<f32> = spectrograph[idx].to_vec();

    /* Events */
    match window.getch() {
      Some(Input::KeyResize) => {
        resize_term(0, 0);
      }
      Some(Input::Character(c)) if c == 'q' => break,
      Some(Input::KeyDC) => break,
      _ => (),
    }

    /* Render */
    window.clear();
    render::render_data(&window, &data, 1.0, -1.0);
    window.draw_box(0, 0);
    window.mvprintw(0, 0, format!("Index: {idx}"));
    window.refresh();

    /* Timing */
    while std::time::Instant::now() - last_time < tick_period {
      std::thread::sleep(tick_period.div(2));
      if sink.empty() {
        break 'main_loop;
      }
    }
    last_time = std::time::Instant::now();
  }

  echo();
  curs_set(1);
  endwin();
}
