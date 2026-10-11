use super::{ContentType, Currency, Guid, GuildUpgrade, Item, Name, Progress, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct CraftingRecipe {
    pub contentGuid:        Guid,
    pub contentType:        u32,
    pub contentUid:         u32,
    pub contentName:        Ptr<Name>,
    pub contentFullName:    Ptr<Name>,
    pub dataId:             u32,
    pub _2c:                u32,
    pub ingredients:        Ptr<[CraftingRecipeIngredient]>,
    pub outputItem:         Ptr<Item>,
    pub outputItemCount:    u32,
    pub rating:             u32,
    pub textName:           u32,
    pub time:               u32,
    pub _58:                u32,
    _5c:                    u32,
    pub _60:                Ptr<Progress>,
    pub outputGuildUpgrade: Ptr<GuildUpgrade>,
    _70:                    u32,
    _74:                    u32,
    pub guildIngredients:   Ptr<[CraftingRecipeGuildIngredient]>,
    pub item:               Ptr<Item>,
    pub _90:                Ptr<u32>,
    _98:                    u32,
    _9c:                    u32,
    _a0:                    u32,
    _a4:                    u32,
    pub _a8:                Ptr<u32>,
}

impl ContentType for CraftingRecipe {
    const ID: u32 = 12;
}

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct CraftingRecipeIngredient {
    pub count:    u32,
    pub r#type:   u32,
    pub item:     Ptr<Item>,
    pub currency: Ptr<Currency>,
}

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct CraftingRecipeGuildIngredient {
    pub count:   u32,
    _04:         u32,
    pub upgrade: Ptr<GuildUpgrade>,
}
