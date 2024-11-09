use rustfft::num_complex::{Complex, ComplexFloat};

pub fn morlet(length: usize, dt: f32, freq: f32) -> Vec<f32> {
  assert!(freq > 0.0, "Expected a non-zero positive input frequency.");
  assert!(dt > 0.0, "Expected a non-zero positive input delta time.");
  assert!(length > 0, "Expected a non-zero input length.");
  let wavelet: &dyn Fn(f32, f32) -> f32 =
    &|t: f32, f: f32| (2.0 * std::f32::consts::PI * t * f).cos() * (-0.5 * t.powf(2.0)).exp();
  let mut wavelet_vec: Vec<f32> = Vec::with_capacity(length);
  for i in 0..length {
    wavelet_vec.push(wavelet(((i as f32) - (length as f32) / 2.0) * dt, freq));
  }
  return wavelet_vec;
}

pub fn cubic_interpolate(y0: f32, y1: f32, y2: f32, y3: f32, mu: f32) -> f32 {
  let mu2: f32 = mu * mu;
  let a0: f32 = y3 - y2 - y0 + y1;
  let a1: f32 = y0 - y1 - a0;
  let a2: f32 = y2 - y0;
  let a3: f32 = y1;
  return a0 * mu * mu2 + a1 * mu2 + a2 * mu + a3;
}

pub fn resample(signal: &Vec<f32>, length: usize) -> Vec<f32> {
  assert_ne!(length, 0, "Expected a non-zero positive output length.");
  assert_ne!(
    signal.len(),
    0,
    "Expected a non-zero positive input length."
  );
  if signal.len() == length {
    return signal.clone();
  };

  let di: f32 = ((signal.len() - 1) as f32) / (length as f32);
  let mut out: Vec<f32> = Vec::with_capacity(length);

  for i in 0..length {
    let (y0, y1, y2, y3): (f32, f32, f32, f32);
    let t = (i as f32 + 0.5) * di;

    let y_b = t.floor() as usize;
    let y_e = t.ceil() as usize;
    if y_b == y_e {
      out.push(signal[y_b]);
      continue;
    }

    if y_b == 0 {
      y0 = signal[0];
      y1 = signal[0];
    } else {
      y0 = signal[y_b - 1];
      y1 = signal[y_b];
    }

    if y_e == signal.len() - 1 {
      y2 = signal[y_e];
      y3 = signal[y_e];
    } else {
      y2 = signal[y_e];
      y3 = signal[y_e + 1];
    }

    let value = cubic_interpolate(y0, y1, y2, y3, t.fract());
    out.push(value);
  }

  return out;
}

pub fn morlet_transform(
  signal: &Vec<f32>,
  df: f32,
  duration: f32,
  width: usize,
  height: usize,
) -> Vec<f32> {
  assert_ne!(
    signal.len(),
    0,
    "Expected an input signal with non-zero length."
  );
  assert!(duration > 0.0, "Expected a positive time duration value.");
  assert!(df > 0.0, "Expected a positive delta-frequency value.");
  assert_ne!(height, 0, "Expected a positive height input.");
  assert_ne!(width, 0, "Expected a positive width input.");

  let mut data_raw: Vec<f32> = vec![0.0f32; width * height];
  let mut data_base: Vec<&mut [f32]> = data_raw.as_mut_slice().chunks_mut(width).collect();
  let data = &mut data_base[..];

  let out_len: usize = width + signal.len() - 1;
  let mut planner: rustfft::FftPlanner<f32> = rustfft::FftPlanner::new();
  let fft_forward = planner.plan_fft_forward(out_len);
  let fft_inverse = planner.plan_fft_inverse(out_len);

  let mut start_t;
  let mut signal_complex: Vec<Complex<f32>> =
    signal.iter().map(|&v| Complex::new(v, 0.0)).collect();
  for _ in 0..out_len - signal_complex.len() {
    signal_complex.push(Complex::new(0.0, 0.0));
  }
  fft_forward.process(&mut signal_complex);

  let dt: f32 = duration / (width as f32);
  start_t = std::time::Instant::now();
  for i in 0..height {
    let wavelet: Vec<f32> = morlet(width, dt, df * ((1 + i) as f32));
    let mut wavelet_complex: Vec<Complex<f32>> =
      wavelet.iter().map(|&v| Complex::new(v, 0.0)).collect();
    for _ in 0..out_len - wavelet_complex.len() {
      wavelet_complex.push(Complex::new(0.0, 0.0));
    }
    fft_forward.process(&mut wavelet_complex);
    let mut conv_complex: Vec<Complex<f32>> = signal_complex
      .iter()
      .zip(wavelet_complex.iter())
      .map(|(&v1, &v2)| v1 * v2)
      .collect();
    fft_inverse.process(&mut conv_complex);
    let conv: Vec<f32> = conv_complex
      .iter()
      .map(|&v| v.re() / out_len as f32)
      .collect();
    let useful = conv[width / 2..conv.len() - width / 2].to_vec();
    let slice = resample(&useful, width);
    data[i].copy_from_slice(slice.as_slice());
  }
  log::log!(
    log::Level::Info,
    "Wavelet transform processed: {}",
    start_t.elapsed().as_secs_f32()
  );
  return data_raw;
}

#[cfg(test)]
mod test {
  use super::morlet;
  use super::resample;

  #[test]
  pub fn test_wavelet_20_0p5_1p0() {
    let test: Vec<f32> = morlet(20, 0.5, 1.0);
    let case: Vec<f32> = vec![
      3.72665317e-06,
      -4.00652974e-05,
      3.35462628e-04,
      -2.18749112e-03,
      1.11089965e-02,
      -4.39369336e-02,
      1.35335283e-01,
      -3.24652467e-01,
      6.06530660e-01,
      -8.82496903e-01,
      1.00000000e+00,
      -8.82496903e-01,
      6.06530660e-01,
      -3.24652467e-01,
      1.35335283e-01,
      -4.39369336e-02,
      1.11089965e-02,
      -2.18749112e-03,
      3.35462628e-04,
      -4.00652974e-05,
    ];
    assert_eq!(test, case);
  }

  #[test]
  pub fn test_resample_i2_i6() {
    let signal: Vec<f32> = vec![0.0, 1.0];
    let sample: Vec<f32> = resample(&signal, 6);
    let expect: Vec<f32> = vec![0.083333336, 0.25, 0.4166667, 0.5833334, 0.75, 0.9166667];
    assert_eq!(sample, expect);
  }

  #[test]
  pub fn test_resample_i1_i6() {
    let signal: Vec<f32> = vec![0.0];
    let sample: Vec<f32> = resample(&signal, 6);
    let expect: Vec<f32> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(sample, expect);
  }

  #[test]
  pub fn test_resample_i6_i6() {
    let signal: Vec<f32> = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let sample: Vec<f32> = resample(&signal, 6);
    let expect: Vec<f32> = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(sample, expect);
  }
}
