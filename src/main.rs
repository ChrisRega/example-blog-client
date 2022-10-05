use crate::egui::Ui;
mod blog_api;
mod ui_helpers;

use crate::blog_api::{
    make_posts_buffer, make_single_post_request, make_tags_buffer, resolve_tags,
    timestamp_to_string, Post, Tag,
};
use eframe::egui;
use example_blog_client::buffered::{SlicePromise, ValuePromise, DataState};

#[tokio::main]
async fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Blog-Demo for async / tokio",
        native_options,
        Box::new(|cc| Box::new(BlogClient::new(cc))),
    );
}

enum Page {
    ListPosts,
    ViewPost(Box<dyn ValuePromise<Post>>),
}

struct BlogClient {
    post_list: Box<dyn SlicePromise<Post>>,
    tag_list: Box<dyn SlicePromise<Tag>>,
    page: Page,
}

impl BlogClient {
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        Self {
            post_list: make_posts_buffer(),
            tag_list: make_tags_buffer(),
            page: Page::ListPosts,
        }
    }
}

impl BlogClient {
    fn ui_view_post(&mut self, ui: &mut Ui) {
        if ui.button("<<").clicked() {
            self.page = Page::ListPosts;
        }

        let post = match &mut self.page {
            Page::ViewPost(post) => post,
            Page::ListPosts => {
                return;
            }
        };
        if ui.button("reload").clicked() {
            post.update();
        }
        match post.poll_state() {
            DataState::UpToDate => {
                let post = post.value().unwrap();
                ui_helpers::view_single_post(post, self.tag_list.as_slice(), ui);
            }
            DataState::Error(msg) => {
                ui.label(format!("Error fetching post: {}", msg));
            }
            _ => {
                ui.spinner();
            }
        }
    }

    fn ui_post_list(&mut self, ui: &mut Ui) {
        match self.post_list.poll_state() {
            DataState::Uninitialized => {
                ui.label("Updating post list");
            }
            DataState::Error(msg) => {
                ui.label(format!("Error occured while fetching post-list: {}", msg));
            }
            DataState::Updating | DataState::UpToDate => {
                let tags = match self.tag_list.poll_state() {
                    DataState::UpToDate => Some(self.tag_list.as_slice()),
                    _ => None,
                };
                if let Some(selected_post) =
                    ui_helpers::view_post_list(self.post_list.as_slice(), tags, ui)
                {
                    self.page = Page::ViewPost(make_single_post_request(selected_post));
                }
            }
        }
        ui.vertical_centered(|ui| {
            if *self.post_list.poll_state() != DataState::UpToDate {
                ui.spinner();
            } else if ui.button("reload").clicked() {
                self.post_list.update();
                self.tag_list.update();
            }
        });
    }
}

impl eframe::App for BlogClient {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| match &mut self.page {
            Page::ListPosts => {
                self.ui_post_list(ui);
            }
            Page::ViewPost(_) => {
                self.ui_view_post(ui);
            }
        });
    }
}
