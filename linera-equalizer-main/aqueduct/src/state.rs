use linera_sdk::views::{ViewStorageContext, RegisterView, MapView, SetView};
use linera_views::views::{GraphQLView, RootView};
use linera_sdk::base::{ChainId, Amount};
use aqueduct::{TokenId, Token, StreamId, Stream};

#[derive(RootView, GraphQLView)]
#[view(context = "ViewStorageContext")]
pub struct Aqueduct {
    pub nfts: MapView<StreamId, Stream>,
    pub listings: MapView<TokenId, Amount>,
    pub number_minted: RegisterView<u64>,
    pub current_owner_minted: MapView<TokenId, ChainId>,
    pub owned_tokens: SetView<Token>,
    pub balance: RegisterView<Amount>,
}

impl Aqueduct {
    pub(crate) async fn balance(&self) -> Amount {
        *self.balance
            .get()
    }

}
