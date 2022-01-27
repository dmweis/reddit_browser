use anyhow::Result;
use iced::Application;
use reddit_browser::reddit_gallery_api;
use roux::util::{FeedOption, TimePeriod};
use roux::Subreddit;

fn is_simple_reddit_image(url: &str) -> bool {
    url.starts_with("https://i.redd.it/")
}

#[derive(Debug)]
enum ImagePostType {
    Image(String),
    Gallery(Vec<String>),
}

struct RedditImageBrowser {
    subreddit: Subreddit,
    feed_search_options: FeedOption,
    current_feed_options: FeedOption,
}

impl RedditImageBrowser {
    fn new(subreddit_name: &str) -> Result<Self> {
        let subreddit = Subreddit::new(subreddit_name);
        let feed_search_options = FeedOption::new().period(TimePeriod::ThisYear);
        let current_feed_options = feed_search_options.clone();
        Ok(Self {
            subreddit,
            feed_search_options,
            current_feed_options,
        })
    }

    async fn pull_image_posts(&mut self) -> Result<Vec<ImagePostType>> {
        let mut posts = vec![];
        let search_results = self
            .subreddit
            .top(25, Some(self.current_feed_options.clone()))
            .await?;
        for post in search_results.data.children {
            if let Some(url) = post.data.url {
                // image post
                if reddit_gallery_api::is_reddit_gallery_link(&url) {
                    // Should this save the gallery link?
                    let post_links =
                        reddit_gallery_api::pull_image_links_from_gallery(&url).await?;
                    posts.push(ImagePostType::Gallery(post_links));
                }
                if is_simple_reddit_image(&url) {
                    posts.push(ImagePostType::Image(url.to_owned()));
                }
            }
        }
        let after_search_token = search_results.data.after.unwrap();
        self.current_feed_options = self.feed_search_options.clone().after(&after_search_token);
        Ok(posts)
    }
}

pub fn main() -> iced::Result {
    BunnyBrowser::run(iced::Settings::default())
}

async fn fetch_image(url: &str) -> Result<iced::image::Handle> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    Ok(iced::image::Handle::from_memory(bytes.as_ref().to_vec()))
}

#[derive(Debug)]
enum Message {
    Images(Result<(Vec<ImagePostType>, iced::image::Handle)>),
}

enum AppState {
    Loading,
    Loaded {
        #[allow(dead_code)]
        images: Vec<ImagePostType>,
        image: iced::image::Handle,
        image_view_state: iced::image::viewer::State,
    },
}

struct BunnyBrowser {
    state: AppState,
}

impl BunnyBrowser {
    fn new() -> Self {
        Self {
            state: AppState::Loading,
        }
    }
}

async fn search_images() -> Result<(Vec<ImagePostType>, iced::image::Handle)> {
    let mut browser = RedditImageBrowser::new("Rabbits").unwrap();
    let data = browser.pull_image_posts().await?;
    let first_image_link = match &data[0] {
        ImagePostType::Image(link) => link.clone(),
        ImagePostType::Gallery(links) => links[0].clone(),
    };
    Ok((data, fetch_image(&first_image_link).await?))
}

impl iced::Application for BunnyBrowser {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, iced::Command<Self::Message>) {
        let app = BunnyBrowser::new();
        (
            app,
            iced::Command::perform(search_images(), Message::Images),
        )
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
            Message::Images(images) => {
                let images = images.unwrap();
                self.state = AppState::Loaded {
                    images: images.0,
                    image: images.1,
                    image_view_state: iced::image::viewer::State::new(),
                };
            }
        }
        iced::Command::none()
    }

    fn view(&mut self) -> iced::Element<Self::Message> {
        match &mut self.state {
            AppState::Loaded {
                images: _,
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
            AppState::Loading => iced::Text::new("Loading").size(40).into(),
        }
    }
}
