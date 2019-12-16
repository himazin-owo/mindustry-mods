#![allow(clippy::non_ascii_literal)]

#[macro_use]
extern crate seed;
extern crate hifitime;
extern crate instant;
extern crate wee_alloc;

use std::convert::TryFrom;
use std::iter;

use seed::{prelude::*, *};
// use wasm_bindgen::prelude::*;

use futures::Future;
use seed::{fetch, Method, Request};
use serde::Deserialize;

use hifitime::Epoch;
use humantime;
use instant::Instant;

// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

mod date {
    use js_sys::Date;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn from_tt(x: f64) -> SystemTime {
        let secs = (x as u64) / 1_000;
        let nanos = ((x as u32) % 1_000) * 1_000_000;
        UNIX_EPOCH + Duration::new(secs, nanos)
    }

    pub fn now() -> SystemTime {
        let x = Date::now();
        from_tt(x)
    }
}

fn link(rel: String, href: String) -> Node<Msg> {
    custom![
        Tag::Custom("link".into()),
        attrs! { At::Href => href, At::Rel => rel }
    ]
}

fn tiny_list(v: &Vec<String>) -> Node<Msg> {
    if !v.is_empty() {
        ul![v
            .iter()
            .filter(|x| x.as_str() != "content")
            .map(|x| li![attrs! { At::Class => x}, x])]
    } else {
        div![]
    }
}

static HOME: &'static str = "/mindustry-mods";
static RGUC: &'static str = "https://raw.githubusercontent.com";

#[derive(Deserialize, Debug, Clone)]
struct Mod {
    name: String,
    stars: u32,
    date_tt: f64,
    desc: String,
    link: String,
    repo: String,
    wiki: Option<String>,
    delta_ago: String,
    icon_raw: Option<String>,
    contents: Vec<String>,
    assets: Vec<String>,
}

impl Mod {
    fn assets_list(&self) -> Node<Msg> {
        tiny_list(&self.assets)
    }

    fn contents_list(&self) -> Node<Msg> {
        tiny_list(&self.contents)
    }

    /// Link to the mod's archive.
    fn archive_link(&self) -> Node<Msg> {
        let l = format!("https://github.com/{}/archive/master.zip", self.repo);
        a![attrs! { At::Href => l }, "zip"]
    }

    /// Endpoint link as a string.
    fn endpoint_href(&self) -> String {
        let path = self.repo.replace("/", "--");
        format!("../m/{}.html", path).into()
    }

    /// Endpoint link to the locally rendered README.md
    fn endpoint_link(&self) -> Node<Msg> {
        a![attrs! { At::Href => self.endpoint_href() }, self.name]
    }

    /// Link to the mods repository.
    fn repo_link(&self) -> Node<Msg> {
        a![attrs! { At::Href => self.link }, "repository"]
    }

    /// Link to the optional wiki.
    fn wiki_link(&self) -> Node<Msg> {
        match &self.wiki {
            Some(link) => a![attrs! { At::Href => link }, "wiki"],
            None => a![style! { "display" => "none" }],
        }
    }

    fn last_commit(&self) -> Node<Msg> {
        span![self.delta_ago, " ago"]
    }

    /// Returns unicode stars.
    fn stars_el(&self) -> Vec<Node<Msg>> {
        let star = || attrs! { At::Class => "star" };

        match usize::try_from(self.stars) {
            Err(_) => vec![div!["err"]],
            Ok(0) => vec![div![attrs! { At::Class => "zero-star"}, "☆"]],
            Ok(x) => iter::repeat("★")
                .take(self.stars as usize)
                .map(|x| div![star(), x])
                .collect(),
        }
    }

    /// Returns an icon link node.
    fn icon(&self) -> Node<Msg> {
        match self.icon_raw.as_ref().map(String::as_str) {
            Some("") | None => a![
                attrs! { At::Href => self.endpoint_href() },
                img![attrs! { At::Src => "../images/nothing.png" },]
            ],

            Some(p) => {
                let i = format!("{}/{}/master/{}", RGUC, self.repo, p);
                a![
                    attrs! { At::Href => self.endpoint_href() },
                    img![attrs! {
                        At::Src => i,
                        At::OnError => "this.src='../images/nothing.png'"
                    }]
                ]
            }
        }
    }

    /// Description paragraph of the mode for the listing.
    fn description(&self) -> Node<Msg> {
        p![attrs! { At::Class => "description" }, self.desc]
    }

    /// Handles path names, which occur when mod.json doesn't exist.
    fn mod_name(&self) -> String {
        match self.name.rfind("/") {
            Some(x) => self.name.split_at(x + 1).1.into(),
            None => self.name.clone(),
        }
    }

    /// Title (name) of the mod in the listing.
    fn listing_title(&self) -> Node<Msg> {
        a![attrs! { At::Href => self.endpoint_href() }, self.mod_name()]
    }

    // fn version_render(&self) -> Node<Msg> {
    //     if let "" = &self.version {
    //         return span![style! { "display" => "none" }];
    //     }
    // }

    /// Returns the `Node<Msg>` for the listing.
    fn listing_item(&self) -> Node<Msg> {
        div![
            attrs! { At::Class => "outside" },
            div![
                attrs! { At::Class => "wrapper" },
                div![attrs! { At::Class => "box icon" }, self.icon()],
                div![attrs! { At::Class => "box name" }, self.listing_title()],
                // div![attrs! { At::Class => "box none" }],
                div![attrs! { At::Class => "box desc" }, self.description()],
                div![
                    attrs! { At::Class => "box links" },
                    self.repo_link(),
                    self.archive_link(),
                    self.wiki_link(),
                ],
                div![attrs! { At::Class => "box stars" }, self.stars_el()],
                div![attrs! { At::Class => "box assets" }, self.assets_list()],
                div![attrs! { At::Class => "box contents" }, self.contents_list()]
            ]
        ]
    }
}

struct Model {
    count: i32,

    /// instant the app started
    dt: Instant,

    /// number of requests submitted for date updates
    data_requested: u32,

    data: Vec<Mod>,

    /// state of sorting, which is initially none
    sort_state: Option<SortBy>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            count: 0,
            dt: Instant::now(),
            data_requested: 0,
            data: vec![],
            sort_state: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Order {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
enum SortBy {
    Stars(Order),
}

#[derive(Debug, Clone)]
enum Msg {
    FetchData(fetch::ResponseDataResult<Vec<Mod>>),
    SortStarsToggle,
}

fn fetch_data() -> impl Future<Item = Msg, Error = Msg> {
    Request::new("data/modmeta.1.0.json")
        .method(Method::Get)
        .fetch_json_data(Msg::FetchData)
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::FetchData(data) => model.data = data.unwrap(),

        Msg::SortStarsToggle => {
            model.data.sort_by_key(|x| x.stars);
            if let Some(SortBy::Stars(Order::Ascending)) = model.sort_state {
                model.sort_state = Some(SortBy::Stars(Order::Descending));
            } else {
                model.data.reverse();
                model.sort_state = Some(SortBy::Stars(Order::Ascending));
            }
        }
    }
}

fn view(model: &Model) -> impl View<Msg> {
    // let now = date::now();
    // let before = date::from_tt(457.3892);
    div![
        attrs! { At::Class => "app" },
        header![h1!["Mindustry Mods"]],
        link("StyleSheet".into(), "css/listing.css".into()),
        button![simple_ev(Ev::Click, Msg::SortStarsToggle), "stars"],
        div![
            attrs! { At::Class => "listing-container" },
            model.data.iter().map(|r| r.listing_item())
        ]
    ]
}

fn after_mount(_: Url, orders: &mut impl Orders<Msg>) -> AfterMount<Model> {
    orders.perform_cmd(fetch_data());
    AfterMount::default()
}

#[wasm_bindgen(start)]
pub fn render() {
    seed::App::builder(update, view)
        .after_mount(after_mount)
        .build_and_start();
}