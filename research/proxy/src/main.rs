use std::collections::HashMap;

use ascalon_asset::content::{ContentServer, ContentType, Guid};
use ascalon_asset::inflate::inflate;
use ascalon_asset::packfile::Packfile;
use ascalon_asset::packfile::armf::PackAssetRootManifest;
use ascalon_asset::packfile::cntc::PackContent;
use ascalon_asset::packfile::mfst::PackAssetManifest;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::Router;
use axum::routing::post_service;
use futures::{StreamExt, stream};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();

    let manifest: HashMap<u32, (u32, u32)> = {
        let latest = reqwest::get("http://assetcdn.101.arenanetworks.com/latest64/101")
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let mut latest = latest.split_whitespace().skip(3);

        let packfile = Packfile::new(
            fetch_compressed(
                &client,
                latest.next().unwrap().parse().unwrap(),
                latest.next().unwrap().parse().unwrap(),
            )
            .await,
        )
        .unwrap();

        let root_manifest = packfile
            .chunks()
            .find(|chunk| chunk.name() == *b"ARMF")
            .map(|chunk| unsafe { &*(chunk.bytes().as_ptr().cast::<PackAssetRootManifest>()) })
            .unwrap();

        let manifest = unsafe { root_manifest.manifests.as_slice() }
            .iter()
            .find(|manifest| unsafe { manifest.name.as_bytes() } == b"C\0l\0i\0e\0n\0t\0C\0o\0n\0t\0e\0n\0t\06\04\0")
            .unwrap();

        let packfile = Packfile::new(
            fetch_compressed(&client, manifest.fileId.get(), manifest.size.get()).await,
        )
        .unwrap();

        let manifest = packfile
            .chunks()
            .find(|chunk| chunk.name() == *b"MFST")
            .map(|chunk| unsafe { &*(chunk.bytes().as_ptr().cast::<PackAssetManifest>()) })
            .unwrap();

        unsafe { manifest.records.as_slice() }
            .iter()
            .map(|record| {
                (
                    record.baseId.get(),
                    (record.fileId.get(), record.size.get()),
                )
            })
            .collect()
    };

    let packfiles: Vec<_> = stream::iter(1282830u32..=1282861)
        .map(|base_id| {
            let client = client.clone();
            let (file_id, size) = manifest[&base_id];

            async move { Packfile::new(fetch_compressed(&client, file_id, size).await).unwrap() }
        })
        .buffered(8)
        .collect()
        .await;

    let content: Vec<_> = packfiles
        .iter()
        .map(|packfile| {
            assert_eq!(packfile.r#type(), *b"cntc");

            packfile
                .chunks()
                .find(|chunk| chunk.name() == *b"Main")
                .map(|chunk| unsafe { &*(chunk.bytes().as_ptr().cast::<PackContent>()) })
                .unwrap()
        })
        .collect();

    let schema = Schema::new(
        Query(ContentServer::new(&content)),
        EmptyMutation,
        EmptySubscription,
    );

    let router = Router::new()
        .route("/", post_service(GraphQL::new(schema)))
        .layer(CorsLayer::permissive());

    axum::serve(
        tokio::net::TcpListener::bind("127.0.0.1:8000")
            .await
            .unwrap(),
        router,
    )
    .await
    .unwrap();
}

struct Query(ContentServer);

unsafe impl Send for Query {}
unsafe impl Sync for Query {}

impl Query {
    fn query<T: ContentType>(
        &self,
        content_guid: Option<Guid>,
        content_name: Option<&str>,
        data_id: Option<u32>,
    ) -> Vec<&T> {
        if let Some(guid) = content_guid {
            self.0.by_guid::<T>(guid).into_iter().collect()
        } else if let Some(name) = content_name {
            self.0.by_name::<T>(name).into_iter().collect()
        } else if let Some(data_id) = data_id {
            self.0.by_data_id::<T>(data_id).into_iter().collect()
        } else {
            self.0.by_type::<T>().to_vec()
        }
    }
}

macro_rules! content_queries {
    (
        $($field:ident: $ty:ty,)*
        ;
        $($field_none:ident: $ty_none:ty,)*
    ) => {
        #[async_graphql::Object]
        impl Query {
            $(
                async fn $field(
                    &self,
                    content_guid: Option<Guid>,
                    content_name: Option<String>,
                    data_id: Option<u32>,
                ) -> Vec<&$ty> {
                    self.query(content_guid, content_name.as_deref(), data_id)
                }
            )*

            $(
                async fn $field_none(
                    &self,
                    content_guid: Option<Guid>,
                    content_name: Option<String>,
                ) -> Vec<&$ty_none> {
                    self.query(content_guid, content_name.as_deref(), None)
                }
            )*
        }
    };
}

content_queries! {
    achievements    : ascalon_asset::content::Achievement,
    cinematics      : ascalon_asset::content::Cinematic,
    colors          : ascalon_asset::content::Color,
    crafting_recipes: ascalon_asset::content::CraftingRecipe,
    currencies      : ascalon_asset::content::Currency,
    emotes          : ascalon_asset::content::Emote,
    guild_upgrades  : ascalon_asset::content::GuildUpgrade,
    items           : ascalon_asset::content::Item,
    magic_door_skins: ascalon_asset::content::MagicDoorSkin,
    mails           : ascalon_asset::content::Mail,
    maps            : ascalon_asset::content::Map,
    progress        : ascalon_asset::content::Progress,
    sectors         : ascalon_asset::content::Sector,
    skills          : ascalon_asset::content::Skill,
    skins           : ascalon_asset::content::Skin,
    species         : ascalon_asset::content::Species,
    traits          : ascalon_asset::content::Trait,
    ;
    color_palettes  : ascalon_asset::content::ColorPalette,
    configurations  : ascalon_asset::content::Configuration,
    effects         : ascalon_asset::content::Effect,
    markers         : ascalon_asset::content::Marker,
    tables_int      : ascalon_asset::content::TableInt,
    teams           : ascalon_asset::content::Team,
}

async fn fetch_compressed(client: &reqwest::Client, file_id: u32, size: u32) -> Vec<u8> {
    let compressed = client
        .get(format!(
            "http://assetcdn.101.arenanetworks.com/program/101/1/0/{file_id}/compressed"
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .await
        .unwrap();

    tokio::task::spawn_blocking(move || {
        let mut bytes = vec![0; size as usize];
        inflate(&compressed, &mut bytes).unwrap();
        bytes
    })
    .await
    .unwrap()
}
