use iced::Application;
use reddit_browser::reddit_gallery_api;
use roux::util::{FeedOption, TimePeriod};
use roux::Subreddit;
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
    images: Vec<String>,
    selected_image: usize,
}

impl Clone for ImageSearcher {
    fn clone(&self) -> Self {
        Self {
            subreddit: Subreddit::new(&self.subreddit.name),
            next_feed_search_options: self.next_feed_search_options.clone(),
            images: self.images.clone(),
            selected_image: self.selected_image,
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
            images: vec![],
            selected_image: 0,
        }
    }

    fn with_next_search(
        subreddit: Subreddit,
        next_feed: FeedOption,
        images: Vec<String>,
        selected_image: usize,
    ) -> Self {
        Self {
            subreddit,
            next_feed_search_options: next_feed,
            images,
            selected_image,
        }
    }

    fn get_image_link(&self) -> Option<String> {
        self.images.get(self.selected_image).cloned()
    }

    fn select_previous_image(&mut self) {
        self.selected_image -= 1;
    }

    fn restart_slideshow(&mut self) {
        self.selected_image = 0;
    }

    async fn search_next(mut self) -> Result<Self> {
        // short circuit
        if self.images.len() > self.selected_image + 1 {
            self.selected_image += 1;
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
                } else if reddit_gallery_api::is_supported_plain_image_link(&url) {
                    posts.push(url.to_owned());
                } else {
                    println!("Unknown link type {:}", url);
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
            self.selected_image,
        ))
    }
}

#[derive(Debug, Clone)]
enum Message {
    TextInputChanged(String),
    StartImageSearch,
    ImageSearchFinished(Result<ImageSearcher>),
    ImageFetched(Result<iced::image::Handle>, ImageSearcher),
    SlideshowTimerElapsed,
    NextImage,
    PreviousImage,
    RestartSlideshow,
    ToggleSlideshow,
}

enum AppState {
    Start {
        text_input_state: iced::text_input::State,
    },
    InitialLoading,
    ImageSlideshow {
        image: iced::image::Handle,
        image_view_state: iced::image::viewer::State,
        image_searcher: ImageSearcher,
    },
}

struct BunnyBrowser {
    state: AppState,
    subreddit_name: String,
    slideshow_mode: bool,
    previous_button_state: iced::button::State,
    next_button_state: iced::button::State,
    toggle_slideshow_button_state: iced::button::State,
    restart_slideshow_button_state: iced::button::State,
}

impl BunnyBrowser {
    fn new() -> Self {
        Self {
            state: AppState::Start {
                text_input_state: iced::text_input::State::new(),
            },
            subreddit_name: String::from("Rabbits"),
            slideshow_mode: false,
            previous_button_state: iced::button::State::new(),
            next_button_state: iced::button::State::new(),
            toggle_slideshow_button_state: iced::button::State::new(),
            restart_slideshow_button_state: iced::button::State::new(),
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
                self.state = AppState::InitialLoading;
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
                self.state = AppState::ImageSlideshow {
                    image,
                    image_view_state: iced::image::viewer::State::new(),
                    image_searcher: searcher,
                };
                if self.slideshow_mode {
                    iced::Command::perform(
                        tokio::time::sleep(std::time::Duration::from_secs(5)),
                        move |_| Message::SlideshowTimerElapsed,
                    )
                } else {
                    iced::Command::none()
                }
            }
            Message::SlideshowTimerElapsed => {
                if self.slideshow_mode {
                    iced::Command::perform(tokio::task::yield_now(), move |_| Message::NextImage)
                } else {
                    iced::Command::none()
                }
            }
            Message::NextImage => {
                if let AppState::ImageSlideshow {
                    image: _,
                    image_view_state: _,
                    image_searcher,
                } = &self.state
                {
                    iced::Command::perform(
                        image_searcher.clone().search_next(),
                        Message::ImageSearchFinished,
                    )
                } else {
                    iced::Command::none()
                }
            }
            Message::PreviousImage => {
                if let AppState::ImageSlideshow {
                    image: _,
                    image_view_state: _,
                    image_searcher,
                } = &self.state
                {
                    let mut image_searcher_clone = image_searcher.clone();
                    image_searcher_clone.select_previous_image();
                    let image_link = image_searcher_clone.get_image_link().unwrap();
                    iced::Command::perform(fetch_image(image_link), move |res| {
                        Message::ImageFetched(res, image_searcher_clone.clone())
                    })
                } else {
                    iced::Command::none()
                }
            }
            Message::ToggleSlideshow => {
                self.slideshow_mode = !self.slideshow_mode;
                if self.slideshow_mode {
                    println!("Slideshow mode enabled");
                } else {
                    println!("Slideshow mode disabled");
                }
                if self.slideshow_mode {
                    iced::Command::perform(
                        tokio::time::sleep(std::time::Duration::from_secs(5)),
                        move |_| Message::SlideshowTimerElapsed,
                    )
                } else {
                    iced::Command::none()
                }
            }
            Message::RestartSlideshow => {
                if let AppState::ImageSlideshow {
                    image: _,
                    image_view_state: _,
                    image_searcher,
                } = &self.state
                {
                    let mut image_searcher_clone = image_searcher.clone();
                    image_searcher_clone.restart_slideshow();
                    let image_link = image_searcher_clone.get_image_link().unwrap();
                    iced::Command::perform(fetch_image(image_link), move |res| {
                        Message::ImageFetched(res, image_searcher_clone.clone())
                    })
                } else {
                    iced::Command::none()
                }
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
            AppState::ImageSlideshow {
                image,
                image_view_state,
                image_searcher: _,
            } => {
                let content = iced::image::Viewer::new(image_view_state, image.clone());

                // toolbar

                let prev_button =
                    iced::Button::new(&mut self.previous_button_state, iced::Text::new("Previous"))
                        .height(iced::Length::Units(40))
                        .on_press(Message::PreviousImage);
                let toggle_slideshow_button = iced::Button::new(
                    &mut self.toggle_slideshow_button_state,
                    iced::Text::new("Toggle slideshow"),
                )
                .height(iced::Length::Units(40))
                .on_press(Message::ToggleSlideshow);
                let restart_slideshow_button = iced::Button::new(
                    &mut self.restart_slideshow_button_state,
                    iced::Text::new("Restart slideshow"),
                )
                .height(iced::Length::Units(40))
                .on_press(Message::RestartSlideshow);
                let next_button =
                    iced::Button::new(&mut self.next_button_state, iced::Text::new("Next"))
                        .height(iced::Length::Units(40))
                        .on_press(Message::NextImage);

                let row = iced::Row::new()
                    .align_items(iced::Align::Center)
                    .width(iced::Length::Fill)
                    .padding(10)
                    .spacing(10)
                    .push(prev_button)
                    .push(toggle_slideshow_button)
                    .push(restart_slideshow_button)
                    .push(next_button);

                let column = iced::Column::new()
                    .width(iced::Length::Fill)
                    .align_items(iced::Align::Center)
                    .push(
                        iced::Container::new(row)
                            .width(iced::Length::Fill)
                            .center_y(),
                    )
                    .push(
                        iced::Container::new(content)
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .center_x()
                            .center_y(),
                    );
                column.into()
            }
            AppState::InitialLoading => {
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
