use ascalon_asset::archive::Archive;
use ascalon_asset::content::{ContentServer, ContentType, Guid};
use ascalon_asset::packfile::Packfile;
use ascalon_asset::packfile::cntc::PackContent;
use async_graphql::http::GraphiQLSource;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::Router;
use axum::response::Html;
use axum::routing::get;

#[tokio::main]
async fn main() {
    let archive = Archive::open("C:\\Program Files\\Guild Wars 2\\Gw2.dat").unwrap();

    let content = (1282830..=1282861)
        .map(|file_id| Packfile::new(archive.read(file_id).unwrap()).unwrap())
        .collect::<Vec<_>>();

    let content = content
        .iter()
        .map(|packfile| {
            assert_eq!(packfile.r#type(), *b"cntc");

            let chunk = packfile
                .chunks()
                .find(|chunk| chunk.name() == *b"Main")
                .unwrap();

            unsafe { &*chunk.bytes().as_ptr().cast::<PackContent>() }
        })
        .collect::<Vec<_>>();

    let schema = Schema::new(
        Query(ContentServer::new(&content)),
        EmptyMutation,
        EmptySubscription,
    );

    let router = Router::new().route(
        "/",
        get(|| async { Html(GraphiQLSource::build().endpoint("/").finish()) })
            .post_service(GraphQL::new(schema)),
    );

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
    achievements: ascalon_asset::content::Achievement,
    cinematics: ascalon_asset::content::Cinematic,
    colors: ascalon_asset::content::Color,
    crafting_recipes: ascalon_asset::content::CraftingRecipe,
    currencies: ascalon_asset::content::Currency,
    emotes: ascalon_asset::content::Emote,
    items: ascalon_asset::content::Item,
    mails: ascalon_asset::content::Mail,
    maps: ascalon_asset::content::Map,
    progresses: ascalon_asset::content::Progress,
    sectors: ascalon_asset::content::Sector,
    skills: ascalon_asset::content::Skill,
    skins: ascalon_asset::content::Skin,
    species: ascalon_asset::content::Species,
    traits: ascalon_asset::content::Trait,
    ;
    color_palettes: ascalon_asset::content::ColorPalette,
    configurations: ascalon_asset::content::Configuration,
    effects: ascalon_asset::content::Effect,
    markers: ascalon_asset::content::Marker,
    tables_int: ascalon_asset::content::TableInt,
    teams: ascalon_asset::content::Team,
}
