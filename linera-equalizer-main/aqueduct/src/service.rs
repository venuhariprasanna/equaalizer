
#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Aqueduct;
use async_trait::async_trait;
use linera_sdk::{base::{WithServiceAbi, Amount}, QueryContext, Service, ViewStateStorage};
use std::sync::Arc;
use thiserror::Error;
use aqueduct::*;
use async_graphql::{Object, Request, Response, Schema, EmptySubscription, EmptyMutation};

linera_sdk::service!(Aqueduct);

impl WithServiceAbi for Aqueduct {
    type Abi = aqueduct::AqueductAbi;
}

#[async_trait]
impl Service for Aqueduct {
    type Error = Error;
    type Storage = ViewStateStorage<Self>;

    async fn query_application(
        self: Arc<Self>,
        _context: &QueryContext,
        request: Request,
    ) -> Result<Response, Self::Error> {
        let schema = Schema::build(self.clone(), MutationRoot {}, EmptySubscription).finish();
        let response = schema.execute(request).await;
        Ok(response)
    }
}
struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn mint(
        &self,
        name: String,
        description: String,
        keywords: Vec<String>,
        segments: Vec<Segment>,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::Mint {
            name,
            description,
            keywords,
            segments,
        })
        .unwrap()
    }
    async fn burn(
        &self,
        token_id: TokenId,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::Burn {
            token_id,
        })
        .unwrap()
    }
    async fn list(
        &self,
        token_id: TokenId,
        amount: Amount,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::List {
            token_id,
            amount,
        })
        .unwrap()
    }
    async fn cancel(
        &self,
        token_id: TokenId,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::Cancel {
            token_id,
        })
        .unwrap()
    }
    async fn buy(
        &self,
        token_id: TokenId,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::Buy {
            token_id,
        })
        .unwrap()
    }
    async fn receive(
        &self,
        token_id: TokenId,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::Receive {
            token_id,
        })
        .unwrap()
    }
    async fn income(
        &self,
        amount: Amount,
        keyword: String,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::Income {
            amount,
            keyword,
        })
        .unwrap()
    }
}

/// An error that can occur while querying the service.
#[derive(Debug, Error)]
pub enum Error {
    /// Query not supported by the application.
    #[error("Queries not supported by application")]
    QueriesNotSupported,

    /// Invalid query argument; could not deserialize request.
    #[error("Invalid query argument; could not deserialize request")]
    InvalidQuery(#[from] serde_json::Error),

    // Add error variants here.
}
