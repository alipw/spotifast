//! Loading and playing the personal mix through the existing app paths.

use super::*;

impl App {
    pub(super) fn load_taste_mix(&mut self) {
        if !self.taste_mix.songs.needs_load() {
            return;
        }
        self.load_generation = self.load_generation.wrapping_add(1);
        self.taste_mix.generation = self.load_generation;
        self.taste_mix.songs = Loadable::Loading;
        let liked = self.library.liked.is_complete().then(|| {
            self.library
                .liked
                .items
                .iter()
                .map(|saved| saved.track.clone())
                .collect()
        });
        self.backend.api(ApiRequest::TasteMixSources {
            generation: self.taste_mix.generation,
            liked,
        });
    }

    pub(super) fn receive_taste_mix(
        &mut self,
        generation: u64,
        result: crate::backend::ApiResult<Vec<Vec<Track>>>,
    ) {
        if generation != self.taste_mix.generation {
            return;
        }
        match result {
            Ok(mut sources) => {
                sources.push(
                    self.plays
                        .plays()
                        .iter()
                        .map(|play| play.track.clone())
                        .collect(),
                );
                self.taste_mix.sources = sources;
                self.regenerate_taste_mix();
            }
            Err(error) => self.taste_mix.songs = Loadable::Failed(error.to_string()),
        }
    }

    pub(super) fn regenerate_taste_mix(&mut self) {
        if self.taste_mix.sources.is_empty() {
            return;
        }
        let songs = crate::mix::generate(
            &self.taste_mix.sources,
            crate::mix::MIX_SIZE,
            &mut rand::rng(),
        );
        for track in &songs {
            if let Some(id) = &track.id {
                self.track_cache.insert(id.clone(), track.clone());
                self.track_used.insert(id.clone(), Instant::now());
            }
        }
        self.load_generation = self.load_generation.wrapping_add(1);
        self.taste_mix.generation = self.load_generation;
        self.taste_mix.songs = Loadable::Loaded(songs);
    }

    pub(super) fn play_ordered_uris(&mut self, uris: Vec<String>, index: u32) {
        if uris.is_empty() || index as usize >= uris.len() {
            return;
        }
        let mut request = PlayRequest::tracks(uris).starting_at_index(index);
        request.ordered = true;
        self.play_request(request, false);
    }

    pub(super) fn taste_mix_uris(&self) -> Vec<String> {
        let Some(songs) = self.taste_mix.songs.get() else {
            return Vec::new();
        };
        let items: Vec<_> = songs
            .iter()
            .cloned()
            .map(|song| (PlayableItem::Track(song), None, None))
            .collect();
        crate::ui::collection::view_indices(
            &items,
            "",
            self.table_sorts.get(&Page::TasteMix).copied(),
        )
        .into_iter()
        .map(|index| songs[index].uri.clone())
        .collect()
    }

    pub(super) fn save_taste_mix(&mut self) {
        let add_uris = self.taste_mix_uris();
        if add_uris.is_empty() {
            return;
        }
        self.actions.push(Action::CreatePlaylist {
            name: gettext(self.locale, "Mix my taste").into_owned(),
            public: false,
            add_uris,
        });
    }
}
