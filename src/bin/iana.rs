use std::fs::{create_dir_all, metadata, write};

use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};

#[tokio::main]
async fn main() {
    let client = Client::builder().user_agent(USER_AGENT).build().unwrap();
    let rsp = client.get(ROOT).send().await.unwrap();
    let src = rsp.text().await.unwrap();
    let html = Html::parse_document(&src);

    let (mut registries, mut types) = (vec![], vec![]);
    for elem in html.select(&Selector::parse("table").unwrap()) {
        if let Some(id) = elem.value().id() {
            if !id.starts_with("table-") {
                continue;
            }
        }

        for a in elem.select(&Selector::parse("tbody > tr > td:nth-child(2) > a").unwrap()) {
            let ty = a.text().collect::<String>();
            let split = ty.find('/').unwrap();
            registries.push(ty[..split].to_owned());
            types.push(ty);
        }
    }

    for registry in registries {
        create_dir_all(format!("{}/{}", CACHE_DIR, registry)).unwrap();
    }

    let mut group = types
        .into_iter()
        .filter_map(|ty| {
            let file = format!("{}/{}.txt", CACHE_DIR, ty);
            match metadata(&file) {
                Ok(_) => return None,
                Err(_) => {}
            };

            let url = format!("{}/{}", BASE, ty);
            let client = client.clone();
            Some(async move { (file, client.get(&url).send().await.unwrap().text().await) })
        })
        .collect::<FuturesUnordered<_>>();
    loop {
        let (cache, data) = match group.next().await {
            Some(x) => x,
            None => break,
        };

        write(&cache, data.unwrap()).unwrap();
    }
}

const BASE: &str = "https://www.iana.org/assignments/media-types/";
const CACHE_DIR: &str = "cache/iana";
const ROOT: &str = "https://www.iana.org/assignments/media-types/media-types.xhtml";
const USER_AGENT: &str = "media-types/0.1.0";
