use anyhow::{anyhow, Result};
use serde::Deserialize;

fn is_simple_reddit_image(url: &str) -> bool {
    url.starts_with("https://i.redd.it/")
}

fn is_simple_imgur_image(url: &str) -> bool {
    url.starts_with("https://i.imgur.com/")
}

pub fn is_supported_plain_image_link(url: &str) -> bool {
    is_simple_reddit_image(url) || is_simple_imgur_image(url)
}

pub fn is_reddit_gallery_link(url: &str) -> bool {
    url.starts_with("https://www.reddit.com/gallery")
}

pub async fn pull_image_links_from_gallery(url: &str) -> Result<Vec<String>> {
    let gallery_id = url
        .strip_prefix("https://www.reddit.com/gallery/")
        .ok_or(anyhow!("failed to strip gallery id from link"))?;
    let data: Vec<GalleryApiData> = reqwest::get(format!(
        "https://www.reddit.com/comments/{}.json",
        &gallery_id
    ))
    .await?
    .json()
    .await?;
    let mut all_images = vec![];
    for gallery in data {
        all_images.extend(gallery.get_largest_image_links());
    }
    Ok(all_images)
}

// TODO(david): This structure should be flattened
#[derive(Deserialize)]
struct GalleryApiData {
    data: GalleryData,
}

impl GalleryApiData {
    fn get_largest_image_links(&self) -> Vec<String> {
        let mut links = vec![];
        for child in &self.data.children {
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
struct GalleryData {
    children: Vec<GalleryDataChildren>,
}

#[derive(Deserialize)]
struct GalleryDataChildren {
    data: GalleryChildrenData,
}

#[derive(Deserialize)]
struct GalleryChildrenData {
    gallery_data: Option<GalleryDataItem>,
}

#[derive(Deserialize)]
struct GalleryDataItem {
    items: Vec<GalleryImageData>,
}

#[derive(Deserialize)]
struct GalleryImageData {
    media_id: String,
}
