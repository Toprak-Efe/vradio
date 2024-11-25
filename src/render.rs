pub struct Packet {
  pub volume: f32,
  pub mute: bool,
}

impl Packet {
  pub fn new(volume: f32, mute: bool) -> Self {
    Packet { volume, mute }
  }

  pub fn volume_up(&mut self) {
    self.volume += 0.05f32;
    if self.volume > 1.0f32 {
      self.volume = 1.0f32;
    }
  }
  
  pub fn volume_down(&mut self) {
    self.volume -= 0.05f32;
    if self.volume < 0.0f32 {
      self.volume = 0.0f32;
    }
  }

}

pub fn render(win: &pancurses::Window, data: &Vec<f32>, v_max: f32, v_min: f32, state: &Packet) {
  /* Input check */
  assert_ne!(
    data.len(),
    0,
    "Expected an input vector with positive length."
  );
  assert_ne!(
    v_max, v_min,
    "Expected min-max inputs with different values."
  );
  let (rows, cols) = win.get_max_yx();
  if cols < 3 {
    return;
  }
  if rows < 3 {
    return;
  }
  let r_max: i32 = 1;
  let r_min: i32 = rows - 2;

  /* Offset Creation */
  /* Sample from the input data */
  let mut values: Vec<f32> = Vec::with_capacity((cols - 2) as usize);
  let dt: f32 = (data.len() as f32) / ((cols - 2) as f32);
  for i in 0..cols - 2 {
    let mut val = data[((i as f32) * dt) as usize];
    if val > v_max {
      val = v_max;
    }
    if val < v_min {
      val = v_min;
    }
    values.push(val);
  }

  /* Calculate the offsets per column */
  let dr = r_max - r_min;
  let dv = v_max - v_min;

  let offsets: Vec<i32> = values
    .iter()
    .map(|v| ((*v - v_min) * (dr as f32) / (dv) + r_min as f32) as i32)
    .collect();
  // rows - 2, 1

  /* Plot Drawing */
  for i in 1..cols - 1 {
    for j in 0..offsets[(i - 1) as usize] {
      win.mvprintw(rows - 2 - j, i, ".");
    }
  }

  /* Audio Drawing */
  win.mvprintw(1, 1, "TRT3");
  let s = format!("[{:.0}%%]", state.volume*100.0f32);
  win.mvprintw(1, 6, s.clone());
  let c;
  if state.mute { c = 'P'; } // P for Pause
  else { c = '>'; } // > for Play
  win.mvprintw(1, 6 + (s.len() as i32), c.to_string());
  //win.mvprintw(1, 4, '\u{1F378}'.to_string());
}
