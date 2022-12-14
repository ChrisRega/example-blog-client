use crate::{resolve_tags, timestamp_to_string, Post, Tag};
use eframe::egui::{Label, ScrollArea, Sense, Ui};

pub fn view_single_post(post: &Post, tags: &[Tag], ui: &mut Ui) {
    ui.heading(post.title.as_str());
    let tags = resolve_tags(post.tags.as_slice(), tags);
    ui.label(format!("Tagged: {}", tags.join(", ")));
    ui.label(format!("Date: {}", timestamp_to_string(post.timestamp)));
    ScrollArea::both()
        .auto_shrink([false; 2])
        .show_viewport(ui, |ui, _| ui.add(Label::new(post.post.as_str())));
}

pub fn view_post_list(posts: &[Post], tags: Option<&[Tag]>, ui: &mut Ui) -> Option<i64> {
    let mut selected_post = None;

    use egui_extras::Size;
    use egui_extras::TableBuilder;

    TableBuilder::new(ui)
        .striped(true)
        .column(Size::remainder().at_least(100.0))
        .column(Size::remainder())
        .column(Size::exact(100.0))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.heading("Title");
            });
            header.col(|ui| {
                ui.heading("Tags");
            });
            header.col(|ui| {
                ui.heading("Time");
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
