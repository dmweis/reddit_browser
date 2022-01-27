use anyhow::Result;
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

#[tokio::main]
async fn main() -> Result<()> {
    let mut image_browser = RedditImageBrowser::new("Rabbits")?;

    loop {
        let posts = image_browser.pull_image_posts().await?;
        for post in posts {
            match post {
                ImagePostType::Image(link) => {
                    println!("{}", link);
                }
                ImagePostType::Gallery(links) => {
                    for link in links {
                        println!("{}", link);
                    }
                }
            }
        }
    }
}
