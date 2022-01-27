use anyhow::Result;
use roux::util::{FeedOption, TimePeriod};
use roux::Subreddit;
use serde::Deserialize;
use std::collections::HashMap;

#[allow(dead_code)]
fn match_simple_reddit_image(url: &str) -> bool {
    url.starts_with("https://i.redd.it/")
}

#[allow(dead_code)]
fn match_reddit_gallery_link(url: &str) -> bool {
    url.starts_with("https://www.reddit.com/gallery")
}

#[derive(Deserialize)]
struct GalleryItem {
    // kind: String,
    data: GalleryItemData,
}

impl GalleryItem {
    fn get_largest_image_links(&self) -> Vec<String> {
        let mut links = vec![];
        for child in &self.data.children {
            // if let Some(media_metadata) = &child.data.media_metadata {
            //     // for (hash, image_data) in media_metadata {
            //     //     links.push(format!("https://i.redd.it/{}.jpg", hash));
            //     //     // if let Some(res_data) = &image_data.p {
            //     //     //     let mut size = 0;
            //     //     //     let mut biggest_link = None;
            //     //     //     for entry in res_data {
            //     //     //         let new_size = entry.x * entry.y;
            //     //     //         if new_size > size {
            //     //     //             size = new_size;
            //     //     //             biggest_link = Some(entry.u.to_owned());
            //     //     //         }
            //     //     //     }
            //     //     //     if let Some(link) = biggest_link {
            //     //     //         links.push(link);
            //     //     //     }
            //     //     // }
            //     // }
            // }
            if let Some(gallery_data) = &child.data.gallery_data {
                for gallery_item in &gallery_data.items {
                    links.push(format!("https://i.redd.it/{}.jpg", gallery_item.media_id));
                }
            }
        }
        links
    }
}

#[derive(Deserialize)]
struct GalleryItemData {
    children: Vec<GalleryItemDataChildren>,
}

#[derive(Deserialize)]
struct GalleryItemDataChildren {
    data: GalleryItemDataChildrenData,
}

#[derive(Deserialize)]
struct GalleryItemDataChildrenData {
    media_metadata: Option<HashMap<String, ImageData>>,
    gallery_data: Option<GalleryDataItem>,
}

#[derive(Deserialize)]
struct GalleryDataItem {
    items: Vec<GalleryImageData>,
}

#[derive(Deserialize)]
struct GalleryImageData {
    media_id: String,
    id: i32,
}

#[derive(Deserialize)]
struct ImageData {
    // status: String,
    // e: String,
    // m: String,
    p: Option<Vec<ResolutionData>>,
}

#[derive(Deserialize)]
struct ResolutionData {
    x: i32,
    y: i32,
    u: String,
}

async fn pull_image_links_from_gallery(url: &str) -> Result<Vec<String>> {
    let hash = url.strip_prefix("https://www.reddit.com/gallery/").unwrap();
    let data: Vec<GalleryItem> =
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
                    println!("{:?}", url);
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
