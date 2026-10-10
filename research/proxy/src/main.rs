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
        .map(|id| Packfile::new(archive.read(id).unwrap()).unwrap())
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
        guid: Option<Guid>,
        name: Option<&str>,
        data_id: Option<u32>,
    ) -> Vec<&T> {
        if let Some(guid) = guid {
            self.0.by_guid::<T>(guid).into_iter().collect()
        } else if let Some(name) = name {
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
        $($with_id:ident: $with_ty:ty,)*
        ;
        $($without_id:ident: $without_ty:ty,)*
    ) => {
        #[async_graphql::Object]
        impl Query {
            $(
                async fn $with_id(
                    &self,
                    guid: Option<Guid>,
                    name: Option<String>,
                    data_id: Option<u32>,
                ) -> Vec<&$with_ty> {
                    self.query::<$with_ty>(guid, name.as_deref(), data_id)
                }
            )*

            $(
                async fn $without_id(
                    &self,
                    guid: Option<Guid>,
                    name: Option<String>,
                ) -> Vec<&$without_ty> {
                    self.query::<$without_ty>(guid, name.as_deref(), None)
                }
            )*
        }
    };
}

content_queries! {
    achievement: ascalon_asset::content::Achievement,
    cinematic: ascalon_asset::content::Cinematic,
    color: ascalon_asset::content::Color,
    crafting_recipe: ascalon_asset::content::CraftingRecipe,
    currency: ascalon_asset::content::Currency,
    emote: ascalon_asset::content::Emote,
    item: ascalon_asset::content::Item,
    mail: ascalon_asset::content::Mail,
    map: ascalon_asset::content::Map,
    progress: ascalon_asset::content::Progress,
    sector: ascalon_asset::content::Sector,
    skill: ascalon_asset::content::Skill,
    skin: ascalon_asset::content::Skin,
    species: ascalon_asset::content::Species,
    r#trait: ascalon_asset::content::Trait,
    ;
    color_palette: ascalon_asset::content::ColorPalette,
    configuration: ascalon_asset::content::Configuration,
    effect: ascalon_asset::content::Effect,
    marker: ascalon_asset::content::Marker,
    table_int: ascalon_asset::content::TableInt,
    team: ascalon_asset::content::Team,
}
