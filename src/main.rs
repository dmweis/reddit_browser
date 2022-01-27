mod reddit_gallery_api;

use anyhow::Result;
use reddit_gallery_api::GalleryApiData;
use roux::util::{FeedOption, TimePeriod};
use roux::Subreddit;

fn match_simple_reddit_image(url: &str) -> bool {
    url.starts_with("https://i.redd.it/")
}

fn match_reddit_gallery_link(url: &str) -> bool {
    url.starts_with("https://www.reddit.com/gallery")
}

async fn pull_image_links_from_gallery(url: &str) -> Result<Vec<String>> {
    let hash = url.strip_prefix("https://www.reddit.com/gallery/").unwrap();
    let data: Vec<GalleryApiData> =
        reqwest::get(format!("https://www.reddit.com/comments/{}.json", &hash))
            .await?
            .json()
            .await?;
    let mut all_images = vec![];
    for gallery in data {
        all_images.extend(gallery.get_largest_image_links());
    }
    Ok(all_images)
}

#[tokio::main]
async fn main() -> Result<()> {
    let subreddit = Subreddit::new("Rabbits");

    let mut feed_options = FeedOption::new().period(TimePeriod::ThisYear);
    loop {
        let top = subreddit.top(100, Some(feed_options)).await?;
        for post in top.data.children {
            if let Some(url) = post.data.url {
                if match_reddit_gallery_link(&url) {
                    println!("Gallery at {:?}", url);
                    let links = pull_image_links_from_gallery(&url).await?;
                    for link in links {
                        println!("   {:?}", link);
                    }
                }
                if match_simple_reddit_image(&url) {
                    println!("{:?}", url);
                }
            }
        }
        println!("Getting next batch");
        let after = top.data.after.unwrap();
        feed_options = FeedOption::new().after(&after).period(TimePeriod::ThisYear);
    }
}
