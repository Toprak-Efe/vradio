use std::collections::HashMap;
use std::iter::Peekable;
use std::slice::Iter;
use std::str::FromStr;

#[derive(Debug)]
pub enum HlsManifestMetadataType {
  Text(String),
  Integer(i32),
  Double(f32),
}

impl TryFrom<String> for HlsManifestMetadataType {
  type Error = &'static str;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    Ok(HlsManifestMetadataType::Text(value))
  }
}

impl TryFrom<i32> for HlsManifestMetadataType {
  type Error = &'static str;

  fn try_from(value: i32) -> Result<Self, Self::Error> {
    Ok(HlsManifestMetadataType::Integer(value))
  }
}

impl TryFrom<f32> for HlsManifestMetadataType {
  type Error = &'static str;

  fn try_from(value: f32) -> Result<Self, Self::Error> {
    Ok(HlsManifestMetadataType::Double(value))
  }
}

#[derive(Debug)]
pub struct HlsManifest {
  pub metadata: HashMap<String, HlsManifestMetadataType>,
  pub tracks: Vec<(String, f32)>,
}

impl HlsManifest {
  fn parse_header<T: FromStr>(
    iterator: &mut Peekable<Iter<&str>>,
    map: &mut HashMap<String, HlsManifestMetadataType>,
    key: &str,
  ) where
    T: FromStr,
    HlsManifestMetadataType: TryFrom<T, Error = &'static str>,
  {
    if let Some(&next) = iterator.peek() {
      if let Ok(parsed) = next.parse::<T>() {
        if let Ok(metadata) = HlsManifestMetadataType::try_from(parsed) {
          map.insert(key.into(), metadata);
          iterator.next(); // Move iterator forward
        }
      }
    }
  }

  pub fn new(url: &str, client: &reqwest::blocking::Client) -> Option<Self> {
    let res = client.get(url).send().ok()?.text().ok()?;
    let tokens: Vec<&str> = res
      .split(|c: char| c == ',' || c == ':' || c.is_ascii_whitespace())
      .filter(|p| !p.is_empty())
      .collect();
    if tokens.len() == 0 {
      return None;
    }
    if tokens[0] != "#EXTM3U" {
      return None;
    }

    let mut meta = HashMap::new();
    let mut data: Vec<(String, f32)> = Vec::with_capacity(8);
    let mut iter = tokens.iter().peekable();

    while let Some(token) = iter.next() {
      match *token {
        "#EXT-X-VERSION" => Self::parse_header::<i32>(&mut iter, &mut meta, *token),
        "#EXT-X-TARGETDURATION" => Self::parse_header::<i32>(&mut iter, &mut meta, *token),
        "#EXT-X-MEDIA-SEQUENCE" => Self::parse_header::<i32>(&mut iter, &mut meta, *token),
        "#EXT-X-DISCONTINUITY-SEQUENCE" => Self::parse_header::<i32>(&mut iter, &mut meta, *token),
        "#EXTINF" => match iter.peek() {
          Some(dur) => match dur.parse::<f32>() {
            Ok(duration) => {
              if !duration.is_sign_positive() {
                continue;
              }
              iter.next();
              match iter.peek() {
                Some(uri) => data.push((String::from(**uri), duration)),
                None => continue,
              }
            }
            Err(_) => continue,
          },
          None => continue,
        },
        &_ => {
          continue;
        }
      }
    }
    
    return Some(Self {
      metadata: meta,
      tracks: data,
    });
  }
}
