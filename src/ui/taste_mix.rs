//! Preview and controls for the listener's generated mix.

use std::sync::Arc;

use crate::api::models::PlayableItem;
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Loadable, Page, RowContext};
use crate::theme;

use super::collection::{Table, remember_table_items, table, table_items_hit};
use super::widgets;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    ui.add_space(12.0);
    theme::text(
        ui,
        gettext(app.locale, "Mix my taste"),
        theme::bold(30.0),
        palette.text,
    );
    ui.add_space(6.0);
    theme::text(
        ui,
        gettext(
            app.locale,
            "Favorites across listening periods, Liked Songs and local history.",
        ),
        theme::regular(13.5),
        palette.secondary,
    );
    ui.add_space(18.0);
    let tracks = match &app.taste_mix.songs {
        Loadable::Loaded(tracks) => tracks,
        Loadable::Loading | Loadable::NotLoaded => {
            widgets::loading_row(ui, &palette, app.locale);
            return;
        }
        Loadable::Failed(error) => {
            let error = error.clone();
            widgets::error_row(ui, app, &error, Some(Page::TasteMix));
            return;
        }
    };
    if tracks.is_empty() {
        theme::text(
            ui,
            gettext(
                app.locale,
                "No playable songs found. Like some songs or listen to music, then retry.",
            ),
            theme::regular(14.0),
            palette.secondary,
        );
        if ui.button(gettext(app.locale, "Retry")).clicked() {
            app.actions.push(Action::Reload(Page::TasteMix));
        }
        return;
    }
    let generation = app.taste_mix.generation;
    let names = app.user_names_revision;
    let items =
        if let Some(items) = table_items_hit(app, &Page::TasteMix, generation, generation, names) {
            items
        } else {
            let rows = tracks
                .iter()
                .cloned()
                .map(|track| (PlayableItem::Track(track), None, None))
                .collect();
            remember_table_items(app, Page::TasteMix, generation, generation, names, rows)
        };
    ui.horizontal_wrapped(|ui| {
        if theme::pill_button(ui, &palette, &gettext(app.locale, "Play"), true).clicked() {
            app.actions.push(Action::PlayTasteMix);
        }
        if theme::pill_button(ui, &palette, &gettext(app.locale, "Regenerate"), false).clicked() {
            app.actions.push(Action::RegenerateTasteMix);
        }
        if theme::pill_button(ui, &palette, &gettext(app.locale, "Save playlist"), false).clicked()
        {
            app.actions.push(Action::SaveTasteMix);
        }
    });
    ui.add_space(12.0);
    let uris: Arc<[String]> = items
        .iter()
        .map(|(item, _, _)| item.uri().to_string())
        .collect::<Vec<_>>()
        .into();
    table(
        app,
        ui,
        Table {
            items: &items,
            row_offset: 0,
            pagination: None,
            context: RowContext::OrderedUris(uris),
            show_album: true,
            show_cover: true,
            show_added: false,
            show_added_by: false,
            page: Page::TasteMix,
            loading: false,
            error: None,
            can_load_more: false,
            filter: "",
            items_revision: generation,
        },
    );
}
