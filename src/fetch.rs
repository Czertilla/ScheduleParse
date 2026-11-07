use anyhow::{Context, Result};
use scraper::{Html, Selector};

/// Fetches the Nextcloud public share page and extracts the absolute URL of its
/// "Download" button, which points at the underlying file (typically an .xlsx).
///
/// Nextcloud's public-share template always exposes the same hidden field:
/// `<input type="hidden" id="downloadURL" value="...">`. That is tried first.
/// If a theme ever omits it, we fall back to the visible "Download" button
/// (`#header-primary-action a.button`) and finally to the download entry in the
/// "..." actions menu (`li#download a`).
pub async fn resolve_download_url(client: &reqwest::Client, share_url: &str) -> Result<String> {
    let resp = client
        .get(share_url)
        .send()
        .await
        .context("requesting the share page")?
        .error_for_status()
        .context("share page returned an error status")?;

    let base = resp.url().clone();
    let body = resp.text().await.context("reading share page body")?;
    let document = Html::parse_document(&body);

    let href = find_hidden_input_value(&document, "downloadURL")
        .or_else(|| find_href(&document, "#header-primary-action a.button"))
        .or_else(|| find_href(&document, "li#download a"))
        .with_context(|| "could not find a download link on the share page")?;

    let absolute = base
        .join(&href)
        .map(|u| u.to_string())
        .unwrap_or(href);
    Ok(absolute)
}

fn find_hidden_input_value(document: &Html, id: &str) -> Option<String> {
    let selector = Selector::parse(&format!("input#{id}")).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|el| el.value().attr("value"))
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn find_href(document: &Html, css: &str) -> Option<String> {
    let selector = Selector::parse(css).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|el| el.value().attr("href"))
        .map(|s| s.to_string())
}

/// Downloads the file at `url` and returns its raw bytes.
pub async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let resp = client
        .get(url)
        .send()
        .await
        .context("downloading the schedule file")?
        .error_for_status()
        .context("schedule file download returned an error status")?;
    let bytes = resp.bytes().await.context("reading downloaded file body")?;
    Ok(bytes.to_vec())
}
