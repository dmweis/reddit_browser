use iced::Application;
use reddit_browser::reddit_gallery_api;
use roux::util::{FeedOption, TimePeriod};
use roux::Subreddit;
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AppError {
    #[error("error fetching image")]
    ImageFetchingError,
    #[error("error talking to the reddit api")]
    RedditApiError,
}

type Result<T> = std::result::Result<T, AppError>;

pub fn main() -> iced::Result {
    BunnyBrowser::run(iced::Settings::default())
}

async fn fetch_image(url: String) -> Result<iced::image::Handle> {
    println!("Fetching {}", url);
    let bytes = reqwest::get(&url)
        .await
        .map_err(|_| AppError::ImageFetchingError)?
        .bytes()
        .await
        .map_err(|_| AppError::ImageFetchingError)?;
    Ok(iced::image::Handle::from_memory(bytes.as_ref().to_vec()))
}

struct ImageSearcher {
    subreddit: Subreddit,
    next_feed_search_options: FeedOption,
    images: VecDeque<String>,
}

impl Clone for ImageSearcher {
    fn clone(&self) -> Self {
        Self {
            subreddit: Subreddit::new(&self.subreddit.name),
            next_feed_search_options: self.next_feed_search_options.clone(),
            images: self.images.clone(),
        }
    }
}

impl std::fmt::Debug for ImageSearcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageSearcher")
            .field("subreddit", &self.subreddit.name)
            .field("next_feed_search_options", &self.next_feed_search_options)
            .field("images", &self.images)
            .finish()
    }
}

impl ImageSearcher {
    fn new(subreddit_name: &str) -> Self {
        let subreddit = Subreddit::new(subreddit_name);
        let next_feed_search_options = FeedOption::new().period(TimePeriod::ThisYear);
        Self {
            subreddit,
            next_feed_search_options,
            images: VecDeque::new(),
        }
    }

    fn with_next_search(subreddit: Subreddit, next_feed: FeedOption, images: Vec<String>) -> Self {
        Self {
            subreddit,
            next_feed_search_options: next_feed,
            images: images.into(),
        }
    }

    fn get_image_link(&self) -> Option<String> {
        self.images.get(0).cloned()
    }

    async fn search_next(mut self) -> Result<Self> {
        // short circuit
        if !self.images.is_empty() {
            self.images.pop_front();
            return Ok(self);
        }
        let search_results = self
            .subreddit
            .top(25, Some(self.next_feed_search_options.clone()))
            .await
            .map_err(|_| AppError::RedditApiError)?;
        let mut posts = vec![];
        for post in search_results.data.children {
            if let Some(url) = post.data.url {
                // image post
                if reddit_gallery_api::is_reddit_gallery_link(&url) {
                    // Should this save the gallery link?
                    let post_links = reddit_gallery_api::pull_image_links_from_gallery(&url)
                        .await
                        .map_err(|_| AppError::RedditApiError)?;
                    posts.extend(post_links);
                }
                if reddit_gallery_api::is_supported_plain_image_link(&url) {
                    posts.push(url.to_owned());
                }
            }
        }

        let after_search_token = search_results.data.after.unwrap();
        let next_feed_options = self
            .next_feed_search_options
            .clone()
            .after(&after_search_token);

        Ok(Self::with_next_search(
            self.subreddit,
            next_feed_options,
            posts,
        ))
    }
}

#[derive(Debug, Clone)]
enum Message {
    TextInputChanged(String),
    StartImageSearch,
    ImageSearchFinished(Result<ImageSearcher>),
    ImageFetched(Result<iced::image::Handle>, ImageSearcher),
    SleepElapsed(ImageSearcher),
}

enum AppState {
    Start {
        text_input_state: iced::text_input::State,
    },
    Loading,
    Loaded {
        image: iced::image::Handle,
        image_view_state: iced::image::viewer::State,
    },
}

struct BunnyBrowser {
    state: AppState,
    subreddit_name: String,
}

impl BunnyBrowser {
    fn new() -> Self {
        Self {
            state: AppState::Start {
                text_input_state: iced::text_input::State::new(),
            },
            subreddit_name: String::from("Rabbits"),
        }
    }
}

impl iced::Application for BunnyBrowser {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, iced::Command<Self::Message>) {
        let app = BunnyBrowser::new();
        (app, iced::Command::none())
    }

    fn title(&self) -> String {
        String::from("Bunny browser")
    }

    fn update(
        &mut self,
        message: Self::Message,
        _clipboard: &mut iced::Clipboard,
    ) -> iced::Command<Self::Message> {
        match message {
            Message::TextInputChanged(input) => {
                self.subreddit_name = input;
                iced::Command::none()
            }
            Message::StartImageSearch => {
                self.state = AppState::Loading;
                let image_searcher = ImageSearcher::new(&self.subreddit_name);
                iced::Command::perform(image_searcher.search_next(), Message::ImageSearchFinished)
            }
            Message::ImageSearchFinished(image_search_result) => {
                let searcher = image_search_result.unwrap();
                let image_link = searcher.get_image_link().unwrap();
                // TODO(David): Why can't I move searcher here?
                iced::Command::perform(fetch_image(image_link), move |res| {
                    Message::ImageFetched(res, searcher.clone())
                })
            }
            Message::ImageFetched(image_result, searcher) => {
                let image = image_result.unwrap();
                self.state = AppState::Loaded {
                    image,
                    image_view_state: iced::image::viewer::State::new(),
                };
                iced::Command::perform(
                    tokio::time::sleep(std::time::Duration::from_secs(2)),
                    move |_| Message::SleepElapsed(searcher.clone()),
                )
            }
            Message::SleepElapsed(searcher) => {
                iced::Command::perform(searcher.search_next(), Message::ImageSearchFinished)
            }
        }
    }

    fn view(&mut self) -> iced::Element<Self::Message> {
        match &mut self.state {
            AppState::Start { text_input_state } => {
                let input_field = iced::text_input::TextInput::new(
                    text_input_state,
                    "Subreddit name",
                    &self.subreddit_name,
                    Message::TextInputChanged,
                )
                .size(30)
                .width(iced::Length::Units(300))
                .padding(15)
                .on_submit(Message::StartImageSearch);
                iced::Container::new(input_field)
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .center_x()
                    .center_y()
                    .into()
            }
            AppState::Loaded {
                image,
                image_view_state,
            } => {
                let content = iced::image::Viewer::new(image_view_state, image.clone());
                iced::Container::new(content)
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .center_x()
                    .center_y()
                    .into()
            }
            AppState::Loading => {
                let text = iced::Text::new("Loading")
                    .horizontal_alignment(iced::HorizontalAlignment::Center)
                    .size(40);
                iced::Container::new(text)
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .center_x()
                    .center_y()
                    .into()
            }
        }
    }
}
