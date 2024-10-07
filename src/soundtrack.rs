use rodio::Source;
use core::fmt;

#[derive(Clone)]
pub struct SoundChannel {
  pub samples: Vec<f32>,
}

#[derive(Clone)]
pub struct SoundTrack {
  pub index: usize,
  pub sampling_rate: u32,
  pub samples_per_channel: u32,
  pub total_duration: std::time::Duration,
  pub sound_channels: Vec<SoundChannel>,
}

impl SoundTrack {
  pub fn new(path: &std::path::PathBuf) -> Result<SoundTrack, std::io::Error> {
    let file = std::fs::File::open(path)?;
    let decoder = rodio::Decoder::new(std::io::BufReader::new(file))
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let duration: std::time::Duration;
    match decoder.total_duration() {
      Some(dur) => duration = dur,
      None => duration = std::time::Duration::new(0, 0),
    }
    let channel_number = decoder.channels();
    let sampling_rate = decoder.sample_rate();
    let samples: Vec<f32> = decoder.convert_samples().collect();
    let samples_per_channel = samples.len() / channel_number as usize;
    let mut channels: Vec<SoundChannel> = Vec::with_capacity(channel_number as usize);
    for i in 0..channel_number {
      let channel_samples: Vec<f32> = (0..samples_per_channel)
        .map(|j| samples[i as usize + j * channel_number as usize])
        .collect();
      channels.push(SoundChannel { samples: channel_samples });
    }    
    Ok(SoundTrack {
      index: 0,
      total_duration: duration,
      sampling_rate: sampling_rate,
      samples_per_channel: samples_per_channel as u32,
      sound_channels: channels
    })
  }
}

impl rodio::Source for SoundTrack {
  fn channels(&self) -> u16 {
    return self.sound_channels.len() as u16;
  }

  fn total_duration(&self) -> Option<std::time::Duration> {
    return Some(self.total_duration);
  }

  fn sample_rate(&self) -> u32 {
    return self.sampling_rate;
  }

  fn current_frame_len(&self) -> Option<usize> {
    return Some((self.samples_per_channel as usize) * self.sound_channels.len() - self.index);
  }
}

impl Iterator for SoundTrack {
  type Item = f32;

  fn next(&mut self) -> Option<Self::Item> {
    if self.index == (self.samples_per_channel as usize) * self.sound_channels.len() {
      return None;
    }
    let channel_no: usize = self.index % self.sound_channels.len();
    let value: f32 = self.sound_channels[channel_no].samples[self.index / self.sound_channels.len()];
    self.index += 1;
    return Some(value);
  }
}

impl fmt::Debug for SoundTrack {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {    
    write!(
      f,
      "Sound {{ sampling_rate: {}, duration: {}, channels: {} }}",
      self.sampling_rate,
      self.total_duration.as_secs_f32(),
      self.sound_channels.len(),
    )
  }
}
