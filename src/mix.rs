//! A personal mix of familiar music, balanced across artists and sources.

use std::collections::{HashMap, HashSet};

use rand::seq::SliceRandom;

use crate::api::models::Track;
use crate::model::Loadable;

pub const MIX_SIZE: usize = 50;

#[derive(Default)]
pub struct TasteMix {
    pub songs: Loadable<Vec<Track>>,
    pub sources: Vec<Vec<Track>>,
    pub generation: u64,
}

fn artist_key(track: &Track) -> String {
    track.artists.first().map_or_else(
        || track.uri.clone(),
        |artist| {
            artist
                .id
                .clone()
                .unwrap_or_else(|| artist.name.to_lowercase())
        },
    )
}

/// Rotate through listening periods, library and history, preferring artists
/// with the fewest selections. Sources overlap; recordings appear only once.
pub fn generate(sources: &[Vec<Track>], limit: usize, rng: &mut impl rand::Rng) -> Vec<Track> {
    let mut pools: Vec<Vec<Track>> = sources.to_vec();
    for pool in &mut pools {
        pool.retain(|track| {
            track.uri.starts_with("spotify:track:")
                && !track.is_local
                && track.is_playable != Some(false)
        });
        pool.shuffle(rng);
    }
    let mut seen_uris = HashSet::new();
    let mut seen_recordings = HashSet::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut songs: Vec<Track> = Vec::new();
    let mut next_source = 0;
    while songs.len() < limit {
        for pool in &mut pools {
            pool.retain(|track| {
                !seen_uris.contains(&track.uri)
                    && track
                        .recording_key()
                        .is_none_or(|key| !seen_recordings.contains(&key))
            });
        }
        // Avoid adjacent songs by the same artist whenever another remains.
        let previous = songs.last().map(artist_key);
        let score = |track: &Track| {
            let artist = artist_key(track);
            (
                previous.as_ref() == Some(&artist),
                counts.get(&artist).copied().unwrap_or(0),
            )
        };
        let Some(best) = pools.iter().flatten().map(&score).min() else {
            break;
        };
        let mut selected = None;
        for step in 0..pools.len() {
            let source = (next_source + step) % pools.len();
            if let Some(index) = pools[source].iter().position(|track| score(track) == best) {
                selected = Some((source, index));
                break;
            }
        }
        let Some((source, index)) = selected else {
            break;
        };
        let track = pools[source].swap_remove(index);
        *counts.entry(artist_key(&track)).or_default() += 1;
        seen_uris.insert(track.uri.clone());
        if let Some(key) = track.recording_key() {
            seen_recordings.insert(key);
        }
        songs.push(track);
        next_source = (source + 1) % pools.len();
    }
    songs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{ArtistRef, ExternalIds};
    use rand::{SeedableRng, rngs::StdRng};

    fn song(id: &str, artist: &str) -> Track {
        Track {
            uri: format!("spotify:track:{id}"),
            artists: vec![ArtistRef {
                id: Some(artist.into()),
                name: artist.into(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn balances_a_dominant_artist_and_spaces_repeats() {
        let mut pool: Vec<_> = (0..100).map(|id| song(&format!("a{id}"), "a")).collect();
        for artist in ["b", "c", "d", "e"] {
            pool.extend((0..20).map(|id| song(&format!("{artist}{id}"), artist)));
        }
        let mix = generate(&[pool], 50, &mut StdRng::seed_from_u64(4));
        let mut counts = HashMap::new();
        for track in &mix {
            *counts.entry(artist_key(track)).or_insert(0) += 1;
        }
        assert_eq!(mix.len(), 50);
        assert!(counts.values().all(|count| *count == 10));
        assert!(
            mix.windows(2)
                .all(|pair| artist_key(&pair[0]) != artist_key(&pair[1]))
        );
    }

    #[test]
    fn includes_sources_without_duplicate_recordings_or_unplayable_tracks() {
        let old = song("old", "old");
        let mut duplicate = old.clone();
        duplicate.uri = "spotify:track:other-release".into();
        duplicate.external_ids = ExternalIds {
            isrc: Some("same".into()),
        };
        let mut old = old;
        old.external_ids = duplicate.external_ids.clone();
        let mut unavailable = song("unavailable", "a");
        unavailable.is_playable = Some(false);
        let mut local = song("local", "a");
        local.is_local = true;
        let sources = vec![
            vec![song("recent", "recent")],
            vec![old.clone()],
            vec![old, duplicate, unavailable, local, song("liked", "liked")],
        ];
        let mix = generate(&sources, 50, &mut StdRng::seed_from_u64(2));
        assert_eq!(mix.len(), 3);
        assert!(mix.iter().any(|track| artist_key(track) == "old"));
        assert!(mix.iter().any(|track| artist_key(track) == "recent"));
        assert!(mix.iter().any(|track| artist_key(track) == "liked"));
    }

    #[test]
    fn handles_empty_and_single_artist_pools_and_regenerates() {
        assert!(generate(&[], 50, &mut StdRng::seed_from_u64(1)).is_empty());
        let sources = vec![(0..60).map(|id| song(&id.to_string(), "a")).collect()];
        let first = generate(&sources, 50, &mut StdRng::seed_from_u64(1));
        let second = generate(&sources, 50, &mut StdRng::seed_from_u64(2));
        assert_eq!(first.len(), 50);
        assert_ne!(first, second);
    }
}
