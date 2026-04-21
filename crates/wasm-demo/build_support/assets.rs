use super::text::slugify;
use reqwest::blocking::Client;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn cache_showcase_image(title: &str, image: &str, assets_dir: &Path) -> String {
    let source_url = if image.starts_with("http://") || image.starts_with("https://") {
        image.to_owned()
    } else {
        format!("https://magicui.design{image}")
    };
    let ext = asset_extension(&source_url, "png");
    let file_name = format!("showcase-{}.{}", slugify(title), ext);
    cache_remote_asset(&source_url, &file_name, assets_dir)
}

pub fn cache_video_thumbnails(video_embeds: &[String], assets_dir: &Path) {
    for embed_url in video_embeds {
        let video_id = youtube_video_id(embed_url);
        if video_id.is_empty() {
            continue;
        }
        let thumbnail_url = format!("https://img.youtube.com/vi/{video_id}/hqdefault.jpg");
        let file_name = format!("youtube-{video_id}.jpg");
        cache_remote_asset(&thumbnail_url, &file_name, assets_dir);
    }
}

pub fn cache_remote_asset(url: &str, file_name: &str, assets_dir: &Path) -> String {
    let target = assets_dir.join(file_name);
    if !target.exists() {
        let client = Client::builder()
            .build()
            .unwrap_or_else(|err| panic!("failed to create asset client for {url}: {err}"));
        let response = client
            .get(url)
            .send()
            .unwrap_or_else(|err| panic!("failed to fetch asset {url}: {err}"))
            .error_for_status()
            .unwrap_or_else(|err| panic!("unexpected asset response for {url}: {err}"));
        let bytes = response
            .bytes()
            .unwrap_or_else(|err| panic!("failed to read asset bytes for {url}: {err}"));
        let mut file = fs::File::create(&target)
            .unwrap_or_else(|err| panic!("failed to create asset file {:?}: {err}", target));
        file.write_all(bytes.as_ref())
            .unwrap_or_else(|err| panic!("failed to write asset file {:?}: {err}", target));
    }

    format!("reference-assets/{file_name}")
}

fn asset_extension(url: &str, fallback: &str) -> String {
    let cleaned = url.split('?').next().unwrap_or(url);
    Path::new(cleaned)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or(fallback)
        .to_owned()
}

fn youtube_video_id(embed_url: &str) -> &str {
    embed_url
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or_default()
}

pub fn generate_reference_assets_module(assets_dir: &Path) -> String {
    let mut files = fs::read_dir(assets_dir)
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();

    let mut source = String::from(
        "use iced::widget::image;\n\npub fn handle(path: &str) -> Option<image::Handle> {\n    match path {\n",
    );
    for file in files {
        let Some(file_name) = file.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let include_path = file.to_string_lossy().replace('\\', "/");
        source.push_str(&format!(
            "        \"reference-assets/{file_name}\" => Some(image::Handle::from_bytes(include_bytes!(r#\"{include_path}\"#).as_slice())),\n"
        ));
    }
    source.push_str("        _ => None,\n    }\n}\n");
    source
}
