use serde::Deserialize;

// TODO(david): This structure should be flattened
#[derive(Deserialize)]
pub struct GalleryApiData {
    data: GalleryData,
}

impl GalleryApiData {
    pub fn get_largest_image_links(&self) -> Vec<String> {
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
