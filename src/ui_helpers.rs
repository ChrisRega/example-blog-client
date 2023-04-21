use crate::{resolve_tags, timestamp_to_string, Post, Tag};
use eframe::egui;
use eframe::egui::{Align, Label, Layout, ScrollArea, Sense, Ui};
use egui_commonmark::CommonMarkViewer;
use egui_extras::Column;

pub fn display_single_post(post: &mut Post, tags: &[Tag], ui: &mut Ui, edit_mode: bool) {
    if edit_mode {
        ui.heading("Edit post...");
        ui.text_edit_singleline(&mut post.title);
        if let Some(outline) = &mut post.outline {
            ui.text_edit_singleline(outline);
            if ui.button("🗑").clicked() {
                post.outline = None;
            }
        } else if ui.button("Add outline").clicked() {
            post.outline = Some(String::new());
        }
        ScrollArea::both().show(ui, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.text_edit_multiline(&mut post.post);
            });
        });
    } else {
        ui.heading(post.title.as_str());
        let tags = resolve_tags(post.tags.as_slice(), tags);
        ui.label(format!("Tagged: {}", tags.join(", ")));
        ui.label(format!("Date: {}", timestamp_to_string(post.timestamp)));
        let mut cache = egui_commonmark::CommonMarkCache::default();

        egui::ScrollArea::vertical().show(ui, |ui| {
            CommonMarkViewer::new("viewer")
                .max_image_width(Some(512))
                .show(ui, &mut cache, post.post.as_str());
        });
    }
}

pub fn view_post_list(posts: &[Post], tags: Option<&[Tag]>, ui: &mut Ui) -> Option<i64> {
    let mut selected_post = None;
    use egui_extras::TableBuilder;
    TableBuilder::new(ui)
        .cell_layout(egui::Layout::left_to_right(Align::Center))
        .striped(true)
        .column(Column::auto().resizable(true))
        .column(Column::auto().resizable(true))
        .column(Column::auto().resizable(true))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.heading("Title");
            });
            header.col(|ui| {
                ui.heading("Tags");
            });
            header.col(|ui| {
                ui.heading("Date");
            });
        })
        .body(|mut body| {
            posts.iter().for_each(|post| {
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        if ui
                            .add(Label::new(post.title.as_str()).sense(Sense::click()))
                            .clicked()
                        {
                            selected_post = Some(post.idx);
                        }
                    });
                    row.col(|ui| {
                        if let Some(tags) = tags {
                            let tags: Vec<_> = resolve_tags(post.tags.as_slice(), tags);
                            ui.label(tags.join(", "));
                        } else {
                            ui.spinner();
                        }
                    });

                    row.col(|ui| {
                        ui.label(timestamp_to_string(post.timestamp).as_str());
                    });
                });
            });
        });
    selected_post
}
