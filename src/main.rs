mod soundtrack;

use crate::soundtrack::SoundTrack;

use clap::{Parser};
use sdl2::keyboard::Keycode;
use sdl2::pixels::{PixelFormatEnum};
use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use core::f32;
use std::time::Duration;

/// CLI tool to play sound streams.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  // /// Path to a track container, mp3, etc. 
  // #[arg(short, long)]
  // path: std::path::PathBuf,

  /// The volume the music will play at.
  #[arg(short, long, default_value_t = 4u8)]
  volume: u8
}

fn update(texture: &mut Texture, data: &[f32], width: u32, height: u32) -> Result<(), String> {
  let filtered_data: Vec<f32> = data.iter().cloned().filter(|&x| !x.is_nan()).collect();
  let data_count: usize = filtered_data.len();
  let max_value: f32 = filtered_data.iter().cloned().fold(f32::MIN, f32::max);
  let min_value: f32 = filtered_data.iter().cloned().fold(f32::MAX, f32::min);
  let range: f32 = max_value - min_value;

  texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
    let max_d_r = ((height.pow(2) + width.pow(2)) as f32).powf(0.5);
    for y in 0..height {
      for x in 0..width {
        let d_y: f32 = (y as i64 - (height / 2) as i64) as f32;
        let d_x: f32 = (x as i64 - (width / 2) as i64) as f32;
        let d_r: f32 = (d_y.powf(2.0) + d_x.powf(2.0)).powf(0.5);
        
        let sample_index: usize =  ((d_r * data_count as f32) / max_d_r) as usize;
        let mapping: f32 = (filtered_data[sample_index] - min_value)/range;
        let brightness: u8 = (mapping * 254.0) as u8;
        
        let offset = y as usize * pitch + x as usize * 3;
        buffer[offset] = brightness;
        buffer[offset+1] = brightness;
        buffer[offset+2] = brightness;
      }
    }
  })?;
  Ok(())
}

fn get_offset(sound: &SoundTrack, window: Duration, start: Duration) -> (usize, usize) {
  let length: usize = sound.sound_channels[0].samples.len();
  let start_f = start.as_secs_f32();
  let window_f = window.as_secs_f32();
  let duration_f = sound.total_duration.as_secs_f32();

  let offset_b = ((length as f32) * (start_f / duration_f)) as usize;
  let offset_w = ((length as f32) * (window_f / duration_f)) as usize;
  if window + start > sound.total_duration {
    (0, offset_w-1)
  } else {
    (offset_b, offset_w-1)
  }
}

fn main() -> Result<(), String> {
  // Parse args and set constants.
  let _args = Args::parse();
  let width: u32 = 800;
  let height: u32 = 600;

  //  Get a handle to the audio stream and create a window
  let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();

  let sdl_context = sdl2::init().unwrap();
  let video_subsystem = sdl_context.video().unwrap();
  let window = video_subsystem
    .window("FFTlyzer", width, height)
    .position_centered()
    .build()
    .unwrap();
  let mut canvas = window.into_canvas().build().unwrap();
  let texture_creator = canvas.texture_creator();
  let mut texture = texture_creator
    .create_texture_streaming(PixelFormatEnum::RGB24, width, height).unwrap();

  let mut delete_hi = std::path::PathBuf::new();
  delete_hi.push("sample-3s.mp3");
  let sound = SoundTrack::new(&delete_hi).unwrap();
  let mut event_pump = sdl_context.event_pump()?;
  
  match update(&mut texture, &sound.sound_channels[0].samples, width, height) {
    Ok(_) => {},
    Err(e) => eprintln!("Error: {}", e),
  }
  canvas.clear();
  canvas.copy(&texture, None, Some(Rect::new(0, 0, 800, 600)))?;
  canvas.present();

  /* Timing details */
  let mut window: Duration = Duration::new(0, 50_000_000u32);
  let sound_tex: SoundTrack = sound.clone();
  stream_handle.play_raw(sound).unwrap();
  
  let t_sound_begin = std::time::Instant::now();
  'main_loop: loop {
    for event in event_pump.poll_iter() {
      match event {
        Event::Quit {..}
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => break 'main_loop,
          _ => {}
      }
    }

    let t_time_elapsed = std::time::Instant::now() - t_sound_begin;
    if t_time_elapsed > sound_tex.total_duration {return Ok(())}
    println!("Updating with time: {:?}", t_time_elapsed);
    if window > sound_tex.total_duration - t_time_elapsed {
      window = sound_tex.total_duration - t_time_elapsed;
    }
    let (offset, range) = get_offset(&sound_tex, window, t_time_elapsed);
    match update(&mut texture, &sound_tex.sound_channels[0].samples[(offset)..(offset + range)], width, height) {
      Ok(_) => {},
      Err(e) => eprintln!("Error: {}", e),
    }

    canvas.clear();
    canvas.copy(&texture, None, Some(Rect::new(0, 0, 800, 600)))?;
    canvas.present();
  }

  return Ok(())
}
