#[derive(Debug)]
struct HlsManifest {
  metadata: HlsManifestMetadata,
  segments: Vec<(String, f32)>
}

#[derive(Debug)]
struct HlsManifestMetadata {
  version: Option<usize>,
  media_sequence: Option<usize>,
  target_duration: Option<usize>,
  discontinuity_sequence: Option<usize>
}

impl HlsManifest {
  fn parse_metadata<T: std::str::FromStr>(iterator: &mut std::iter::Peekable<std::slice::Iter<&str>>, map: &mut std::collections::HashMap<String, T>, key: &str) -> Result<(), ()> {
    match iterator.peek() {
      Some(&next_token) => {
        match next_token.parse::<T>() {
          Ok(parsed_value) => {
            map.insert(String::from(key), parsed_value);
            iterator.next(); // Consume the token after parsing
            return Ok(());
          },
          Err(_) => return Err(()),
        }
      },
      None => return Err(()),
    }
  }

  pub fn new(msg: &String) -> Option<Self> {
    /* Tokenize */
    let tokens: Vec<&str> = msg
      .split(|c: char| c == ',' || c ==':' || c.is_ascii_whitespace())
      .filter(|p| !p.is_empty())
      .collect();

    if cfg!(debug_assertions) {
      log::debug!("Tokens: [");
      tokens.iter().map(|tok| log::debug!(" {:?} ", tok)).last();
      log::debug!("]\n");
    }
    
    if tokens.len() == 0 { return None; }
    if tokens[0] != "#EXTM3U" { return None; }
    let mut metadata: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut hlssegments: Vec<(String, f32)> = Vec::with_capacity(16);

    let mut iterator = tokens.iter().peekable();
    while let Some(token) = iterator.next() {
      match *token {
        "#EXT-X-VERSION" => {
          if let Err(_) = Self::parse_metadata(&mut iterator, &mut metadata, *token) {
            continue;
          }
        },
        "#EXT-X-TARGETDURATION" => {
          if let Err(_) = Self::parse_metadata(&mut iterator, &mut metadata, *token) {
            continue;
          }
        },
        "#EXT-X-MEDIA-SEQUENCE" => {
          if let Err(_) = Self::parse_metadata(&mut iterator, &mut metadata, *token) {
            continue;
          }
        },
        "#EXT-X-DISCONTINUITY-SEQUENCE" => {
          if let Err(_) = Self::parse_metadata(&mut iterator, &mut metadata, *token) {
            continue;
          }
        },
        "#EXTINF" => {
          match iterator.peek() {
            Some(dur) => {
              match dur.parse::<f32>() {
                Ok(duration) => {
                  iterator.next();
                  match iterator.peek() {
                    Some(uri) => hlssegments.push((String::from(**uri), duration)),
                    None => continue,
                  }
                },
                Err(_) => continue,
              }
            },
            None => continue,
          }
        },
        _ => continue,
      }
    }

    if hlssegments.len() == 0 {
      return None;
    }

    let hlsmetadata = HlsManifestMetadata {
      version: metadata.get("#EXT-X-VERSION").cloned(),
      media_sequence: metadata.get("#EXT-X-TARGETDURATION").cloned(),
      target_duration: metadata.get("#EXT-X-MEDIA-SEQUENCE").cloned(),
      discontinuity_sequence: metadata.get("#EXT-X-DISCONTINUITY-SEQUENCE").cloned(),
    };

    return Some(HlsManifest {
      metadata: hlsmetadata,
      segments: hlssegments
    });
  }
}

#[cfg(test)]
mod test {
  use super::HlsManifest;

  #[test]
  pub fn test_manifest_fetch() {
    let url = "https://rd-trtradyo3.medya.trt.com.tr/master_128.m3u8";
    let client = reqwest::blocking::Client::new();
    let res = client.get(url)
      .send()
      .unwrap();
    let text = res.text().unwrap();
    let manifest = HlsManifest::new(&text);
    println!("Debugging: {:?}", manifest);
  }
}