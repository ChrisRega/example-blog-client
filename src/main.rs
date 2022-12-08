use crate::egui::{ProgressBar, Ui};
use std::sync::Arc;
mod blog_api;
mod ui_helpers;

use crate::blog_api::{
    make_immediate_post_request, make_posts_buffer, make_tags_buffer, resolve_tags,
    timestamp_to_string, Post, Tag,
};
use eframe::egui;
use eframe::egui::TextEdit;
use lazy_async_promise::{
    DataState, ImmediateValuePromise, ImmediateValueState, LazyVecPromise, Promise,
};
use reqwest::StatusCode;

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
    ViewPost(PostState),
    Login(LoginState),
}

#[derive(Default)]
struct LoginState {
    credentials: blog_api::Login,
    login_response: Option<ImmediateValuePromise<StatusCode>>,
}

struct PostState {
    post: ImmediateValuePromise<Post>,
    edit_mode: bool,
}

impl PostState {
    pub fn from_promise(post: ImmediateValuePromise<Post>) -> Self {
        PostState {
            post,
            edit_mode: false,
        }
    }
}

struct BlogClient {
    post_list: LazyVecPromise<Post>,
    tag_list: LazyVecPromise<Tag>,
    page: Page,
    logged_in: bool,
    client: Arc<reqwest::Client>,
}

impl BlogClient {
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        Self {
            post_list: make_posts_buffer(),
            tag_list: make_tags_buffer(),
            page: Page::ListPosts,
            client: Arc::new(
                reqwest::Client::builder()
                    .cookie_store(true)
                    .build()
                    .expect("Could not make client"),
            ),
            logged_in: false,
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
            _ => {
                return;
            }
        };

        let state = post.post.poll_state_mut();
        match state {
            ImmediateValueState::Success(posting) => {
                if self.logged_in && !post.edit_mode && ui.button("edit...").clicked() {
                    post.edit_mode = true;
                } else if self.logged_in && post.edit_mode && ui.button("cancel").clicked() {
                    post.edit_mode = false;
                    self.page = Page::ViewPost(PostState::from_promise(
                        make_immediate_post_request(posting.idx),
                    ));
                } else {
                    ui_helpers::display_single_post(
                        posting,
                        self.tag_list.as_slice(),
                        ui,
                        post.edit_mode,
                    );
                }
            }
            ImmediateValueState::Error(e) => {
                ui.label(format!("Error fetching post: {}", **e));
            }
            _ => {
                ui.spinner();
            }
        }
    }

    fn ui_login(&mut self, ui: &mut Ui) {
        let login = match &mut self.page {
            Page::Login(login) => login,
            _ => {
                return;
            }
        };
        ui.heading("Login to blog");
        ui.label("User");
        ui.text_edit_singleline(&mut login.credentials.user);
        ui.label("Password");
        ui.add(TextEdit::singleline(&mut login.credentials.password).password(true));

        if let Some(response) = &mut login.login_response {
            match response.poll_state() {
                ImmediateValueState::Updating => {
                    ui.spinner();
                }
                ImmediateValueState::Error(e) => {
                    ui.label(format!("Error: {}", **e));
                }
                ImmediateValueState::Success(code) => {
                    if code.is_success() {
                        ui.label("Successfully logged in!");
                        self.logged_in = true;
                    } else {
                        ui.label(format!("Server didn't return success! (code: {})", *code));
                        if ui.button("Retry").clicked() {
                            login.login_response = None;
                        }
                    }
                }
            }
        } else if ui.button("login").clicked() {
            login.login_response = Some(login.credentials.try_login(self.client.clone()));
        }
        if ui.button("back").clicked() {
            self.page = Page::ListPosts;
        }
    }

    fn ui_post_list(&mut self, ui: &mut Ui) {
        match self.post_list.poll_state() {
            DataState::Uninitialized => {
                ui.label("Updating post list");
            }
            DataState::Error(msg) => {
                ui.label(format!("Error occurred while fetching post-list: {}", msg));
            }
            DataState::Updating(_) | DataState::UpToDate => {
                let tags = match self.tag_list.poll_state() {
                    DataState::UpToDate => Some(self.tag_list.as_slice()),
                    _ => None,
                };
                if let Some(selected_post) =
                    ui_helpers::view_post_list(self.post_list.as_slice(), tags, ui)
                {
                    self.page = Page::ViewPost(PostState::from_promise(
                        make_immediate_post_request(selected_post),
                    ));
                }
            }
        }
        ui.vertical_centered(|ui| {
            let state = self.post_list.poll_state();
            let progress = state.get_progress();
            if let Some(progress) = progress {
                let bar = ProgressBar::new(progress.as_f32())
                    .animate(true)
                    .show_percentage();
                ui.add(bar);
            } else {
                if ui.button("reload").clicked() {
                    self.post_list.update();
                    self.tag_list.update();
                }
                if !self.logged_in && ui.button("login").clicked() {
                    self.page = Page::Login(LoginState::default());
                }
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
            Page::Login(_) => {
                self.ui_login(ui);
            }
        });
    }
}
