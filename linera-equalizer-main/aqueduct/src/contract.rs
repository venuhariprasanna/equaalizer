#![cfg_attr(target_arch = "wasm32", no_main)]
mod state;

use self::state::Aqueduct;
use async_trait::async_trait;
use linera_sdk::{
    base::{SessionId, WithContractAbi, ChainId, Amount, Timestamp, ApplicationId},
    contract::system_api,
    ApplicationCallResult, CalleeContext, Contract, MessageContext,
    ExecutionResult, OperationContext, SessionCallResult, ViewStateStorage,
};
use thiserror::Error;
use serde::Serialize;
use log::info;
use bcs::{to_bytes, from_bytes};
//use keccak_hash::keccak256;
use aqueduct::*;
//use uuid::Uuid;
use std::str::Utf8Error;
use logger_macro::*;

linera_sdk::contract!(Aqueduct);

impl WithContractAbi for Aqueduct {
    type Abi = aqueduct::AqueductAbi;
}

#[async_trait]
impl Contract for Aqueduct {
    type Error = Error;
    type Storage = ViewStateStorage<Self>;

    #[initialize(Self::logger_id()?)]
    async fn initialize(
        &mut self,
        _context: &OperationContext,
        _argument: (),
    ) -> Result<ExecutionResult<Self::Message>, Self::Error> {
        Ok(ExecutionResult::default())
    }

    #[execute_operation(Self::logger_id()?)]
    async fn execute_operation(
        &mut self,
        context: &OperationContext,
        operation: Operation,
    ) -> Result<ExecutionResult<Self::Message>, Self::Error> {
        match operation.clone() {
            Operation::Mint {
                name,
                mut description,
                keywords,
                segments,
            } => {
                let stream_id = StreamId {
                    company: system_api::current_chain_id(),
                    //id: uuid::Uuid::new_v4().to_string(),
                    id: format!("{}", self.nfts.indices().await?.len()),
                };
                let stream = Stream {
                    keywords,
                    segments,
                    created: system_api::current_system_time().micros(),
                    milestones_received: 0,
                    periods_received: 0,
                };
                self.nfts.insert(&stream_id, stream);
                let s = serde_json::to_string(&stream_id)?;
                if description.is_empty() { description = s.clone(); }
                let metadata = TokenMetadata {
                    name,
                    description,
                    image: s,
                };
                let num = *self.number_minted.get();
                let id = TokenId { minted_chain: system_api::current_chain_id(), index: num };
                self.owned_tokens.insert(&Token { id: id.clone(), metadata })?;
                self.current_owner_minted.insert(&id, system_api::current_chain_id())?;
                self.number_minted.set(num + 1);
                Ok(ExecutionResult::default())
            },
            Operation::Burn {
                token_id,
            } => {
                let mut realtoken = Token {
                    id: TokenId {
                        minted_chain: system_api::current_chain_id(),
                        index: 0,
                    },
                    metadata: TokenMetadata {
                        name: "".to_string(),
                        description: "".to_string(),
                        image: "".to_string(),
                    },
                };
                self.owned_tokens.for_each_index(|key| {
                    if key.id == token_id {
                        realtoken = key;
                    }
                    Ok(())
                }).await?;
                self.owned_tokens.remove(&realtoken)?;
                self.listings.remove(&token_id)?;
                Ok(ExecutionResult::default())
            },
            Operation::List {
                token_id,
                amount,
            } => {
                self.listings.insert(&token_id, amount)?;
                Ok(ExecutionResult::default())
            },
            Operation::Cancel {
                token_id,
            } => {
                self.listings.remove(&token_id)?;
                Ok(ExecutionResult::default())
            },
            Operation::Income {
                amount,
                keyword,
            } => {
                self.credit(amount).await?;
                Ok(ExecutionResult::default())
            },
//m4 execute_operation start
        Operation::Buy { token_id } => {
            let m4 = M4 {
                original_chain: system_api::current_chain_id(),
                original_operation: Some(operation),
                original_ope_context: Some(context.clone()),
                original_message: None,
                original_msg_context: None,
            };
            let __message = Message::Buy_GetOwner { m4: m4.clone() };
            return Ok(ExecutionResult::default()
                .with_authenticated_message(token_id.minted_chain, __message));
        }
        Operation::Receive { token_id } => {
            let m4 = M4 {
                original_chain: system_api::current_chain_id(),
                original_operation: Some(operation),
                original_ope_context: Some(context.clone()),
                original_message: None,
                original_msg_context: None,
            };
            let metadata: TokenMetadata = self.get_token(token_id).await?;
            if let Ok(stream_id) = serde_json::from_str::<StreamId>(&metadata.image) {
                let __message = Message::Receive_Company {
                    m4: m4.clone(),
                    metadata: metadata.clone(),
                    stream_id: stream_id.clone(),
                };
                return Ok(ExecutionResult::default()
                    .with_authenticated_message(stream_id.company.clone(), __message));
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "Err(Error :: JsonError)".to_string(),
                };
                return Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message));
            }
            Ok(ExecutionResult::default())
        }
//m4 execute_operation end
        }
    }

    #[execute_message(Self::logger_id()?)]
    async fn execute_message(
        &mut self,
        context: &MessageContext,
        message: Message,
    ) -> Result<ExecutionResult<Self::Message>, Self::Error> {
        match message.clone() {
            Message::Default => Ok(ExecutionResult::default()),
//m4 execute_message start
        Message::Result { m4, origin, result } => {
            info!(
                "m4: {:?}
origin: {}res: {}",
                m4, origin, result
            );
            Ok(ExecutionResult::default())
        }

        Message::Buy_BackToPay {
            m4,
            cur_owner,
            price,
        } => {
            if let Some(Operation::Buy { token_id }) = m4.original_operation.clone() {
                if let Some(ope_context) = m4.original_ope_context.clone() {
                    match self.debit(price).await {
                        Ok(_) => {
                            let __message = Message::Buy_ReceivePayment {
                                m4: m4.clone(),
                                cur_owner: cur_owner.clone(),
                                price: price.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(cur_owner, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedOperationContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedOperationError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::Buy_GetOwner { m4 } => {
            if let Some(Operation::Buy { token_id }) = m4.original_operation.clone() {
                if let Some(ope_context) = m4.original_ope_context.clone() {
                    match self.get_owner(token_id).await {
                        Ok(cur_owner) => {
                            let __message = Message::Buy_GetPrice {
                                m4: m4.clone(),
                                cur_owner: cur_owner.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(cur_owner, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedOperationContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedOperationError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::Buy_GetPrice { m4, cur_owner } => {
            if let Some(Operation::Buy { token_id }) = m4.original_operation.clone() {
                if let Some(ope_context) = m4.original_ope_context.clone() {
                    match self.get_price(token_id).await {
                        Ok(price) => {
                            let __message = Message::Buy_BackToPay {
                                m4: m4.clone(),
                                cur_owner: cur_owner.clone(),
                                price: price.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedOperationContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedOperationError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::Buy_ReceivePayment {
            m4,
            cur_owner,
            price,
        } => {
            if let Some(Operation::Buy { token_id }) = m4.original_operation.clone() {
                if let Some(ope_context) = m4.original_ope_context.clone() {
                    match self.credit(price).await {
                        Ok(_) => {
                            if let Ok(data) = to_bytes(&price) {
                                let message: Message = Message::SafeTransferFrom {
                                    from: cur_owner,
                                    to: m4.original_chain,
                                    token_id,
                                    data,
                                };
                                let __message = Message :: Result { m4 : m4 . clone () , origin : system_api :: current_chain_id () , result : "Ok(ExecutionResult :: default () . with_authenticated_message (cur_owner , message))" . to_string () , } ;
                                return Ok(ExecutionResult::default()
                                    .with_authenticated_message(m4.original_chain, __message));
                            } else {
                                self.debit(price).await;
                                let __message = Message::Buy_ReceiveRollbackPayment {
                                    m4: m4.clone(),
                                    cur_owner: cur_owner.clone(),
                                    price: price.clone(),
                                };
                                return Ok(ExecutionResult::default()
                                    .with_authenticated_message(cur_owner, __message));
                            }
                            Ok(ExecutionResult::default())
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedOperationContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedOperationError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::Buy_ReceiveRollbackPayment {
            m4,
            cur_owner,
            price,
        } => {
            if let Some(Operation::Buy { token_id }) = m4.original_operation.clone() {
                if let Some(ope_context) = m4.original_ope_context.clone() {
                    match self.credit(price).await {
                        Ok(_) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: "Err(Error :: BcsPriceError)".to_string(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedOperationContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedOperationError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::Receive_Company {
            m4,
            metadata,
            stream_id,
        } => {
            if let Some(Operation::Receive { token_id }) = m4.original_operation.clone() {
                if let Some(ope_context) = m4.original_ope_context.clone() {
                    match self.handle_receive(stream_id, m4.original_chain).await {
                        Ok(_) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: "Ok(ExecutionResult :: default ())".to_string(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedOperationContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedOperationError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::SafeTransferFrom {
            from,
            to,
            token_id,
            data,
        } => {
            let m4 = M4 {
                original_chain: system_api::current_chain_id(),
                original_operation: None,
                original_ope_context: None,
                original_message: Some(ActualMessage::SafeTransferFrom {
                    from,
                    to,
                    token_id,
                    data,
                }),
                original_msg_context: Some(context.clone()),
            };
            let sender: ChainId = system_api::current_chain_id();
            if from != sender {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "Err(Error :: NotAnOperatorNorApproved)".to_string(),
                };
                return Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message));
            }
            let __message = Message::SafeTransferFrom_CheckOwn {
                m4: m4.clone(),
                sender: sender.clone(),
            };
            return Ok(ExecutionResult::default().with_authenticated_message(from, __message));
        }
        Message::SafeTransferFrom_CheckOwn { m4, sender } => {
            if let Some(ActualMessage::SafeTransferFrom {
                from,
                to,
                token_id,
                data,
            }) = m4.original_message.clone()
            {
                if let Some(msg_context) = m4.original_msg_context.clone() {
                    match self.check_own(token_id).await {
                        Ok(own) => {
                            if !own {
                                let __message = Message::Result {
                                    m4: m4.clone(),
                                    origin: system_api::current_chain_id(),
                                    result: "Err(Error :: DoesNotOwnToken)".to_string(),
                                };
                                return Ok(ExecutionResult::default()
                                    .with_authenticated_message(m4.original_chain, __message));
                            }
                            let __message = Message::SafeTransferFrom_TransferFrom {
                                m4: m4.clone(),
                                sender: sender.clone(),
                                own: own.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(from, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedMessageContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedMessageError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::SafeTransferFrom_OnERC721Received {
            m4,
            sender,
            own,
            token,
        } => {
            if let Some(ActualMessage::SafeTransferFrom {
                from,
                to,
                token_id,
                data,
            }) = m4.original_message.clone()
            {
                if let Some(msg_context) = m4.original_msg_context.clone() {
                    match self
                        .on_erc721_received(OnERC721Received {
                            from: from.clone(),
                            to: to.clone(),
                            token_id: token_id.clone(),
                            data: data.clone(),
                        })
                        .await
                    {
                        Ok(check) => {
                            if let Ok(b) = to_bytes(&OnERC721Received {
                                from: from.clone(),
                                to: to.clone(),
                                token_id: token_id.clone(),
                                data: data.clone(),
                            }) {
                                let mut bb = b;
                                //keccak256(&mut bb);
                                if check != bb {
                                    let __message = Message::Result {
                                        m4: m4.clone(),
                                        origin: system_api::current_chain_id(),
                                        result: "Err(Error :: AfterTransferCheckFailed)"
                                            .to_string(),
                                    };
                                    return Ok(ExecutionResult::default()
                                        .with_authenticated_message(m4.original_chain, __message));
                                }
                                let __message = Message::Result {
                                    m4: m4.clone(),
                                    origin: system_api::current_chain_id(),
                                    result: "Ok(ExecutionResult :: default ())".to_string(),
                                };
                                return Ok(ExecutionResult::default()
                                    .with_authenticated_message(m4.original_chain, __message));
                            } else {
                                let __message = Message::Result {
                                    m4: m4.clone(),
                                    origin: system_api::current_chain_id(),
                                    result: "Err(Error :: BcsError)".to_string(),
                                };
                                return Ok(ExecutionResult::default()
                                    .with_authenticated_message(m4.original_chain, __message));
                            }
                            Ok(ExecutionResult::default())
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedMessageContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedMessageError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::SafeTransferFrom_TransferFrom { m4, sender, own } => {
            if let Some(ActualMessage::SafeTransferFrom {
                from,
                to,
                token_id,
                data,
            }) = m4.original_message.clone()
            {
                if let Some(msg_context) = m4.original_msg_context.clone() {
                    match self.transfer_from_me(token_id).await {
                        Ok(token) => {
                            let __message = Message::SafeTransferFrom_TransferTo {
                                m4: m4.clone(),
                                sender: sender.clone(),
                                own: own.clone(),
                                token: token.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(to, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedMessageContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedMessageError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::SafeTransferFrom_TransferTo {
            m4,
            sender,
            own,
            token,
        } => {
            if let Some(ActualMessage::SafeTransferFrom {
                from,
                to,
                token_id,
                data,
            }) = m4.original_message.clone()
            {
                if let Some(msg_context) = m4.original_msg_context.clone() {
                    match self.transfer_to_me(&token).await {
                        Ok(_) => {
                            let __message = Message::SafeTransferFrom_UpdateMintedChain {
                                m4: m4.clone(),
                                sender: sender.clone(),
                                own: own.clone(),
                                token: token.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(token_id.minted_chain, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedMessageContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedMessageError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::SafeTransferFrom_UpdateMintedChain {
            m4,
            sender,
            own,
            token,
        } => {
            if let Some(ActualMessage::SafeTransferFrom {
                from,
                to,
                token_id,
                data,
            }) = m4.original_message.clone()
            {
                if let Some(msg_context) = m4.original_msg_context.clone() {
                    match self.transfer_update_minted(token_id, to).await {
                        Ok(_) => {
                            let __message = Message::SafeTransferFrom_OnERC721Received {
                                m4: m4.clone(),
                                sender: sender.clone(),
                                own: own.clone(),
                                token: token.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(to, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedMessageContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedMessageError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::Result { m4, origin, result } => {
            let m4 = M4 {
                original_chain: system_api::current_chain_id(),
                original_operation: None,
                original_ope_context: None,
                original_message: Some(ActualMessage::Result { /*m4,*/ origin: origin.clone(), result: result.clone() }),
                original_msg_context: Some(context.clone()),
            };
            info!("m4: {:?} origin: {} res: {}", m4, origin, result);
            if let Some(ActualMessage::SafeTransferFrom {
                from,
                to,
                token_id,
                data,
            }) = m4.original_message.clone()
            {
                if result.contains("Err") {
                    if let Ok(price) = from_bytes::<Amount>(&data) {
                        let __message = Message::Result_BackToPay {
                            m4: m4.clone(),
                            from: from.clone(),
                            to: to.clone(),
                            price: price.clone(),
                        };
                        return Ok(
                            ExecutionResult::default().with_authenticated_message(from, __message)
                        );
                    } else {
                        let __message = Message::Result {
                            m4: m4.clone(),
                            origin: system_api::current_chain_id(),
                            result: "Err(Error :: BcsError)".to_string(),
                        };
                        return Ok(ExecutionResult::default()
                            .with_authenticated_message(m4.original_chain, __message));
                    }
                }
            }
            let __message = Message::Result {
                m4: m4.clone(),
                origin: system_api::current_chain_id(),
                result: "Ok(ExecutionResult :: default ())".to_string(),
            };
            return Ok(
                ExecutionResult::default().with_authenticated_message(m4.original_chain, __message)
            );
        }
        Message::Result_BackToPay {
            m4,
            from,
            to,
            price,
        } => {
            if let Some(ActualMessage::Result { /*m4,*/ origin, result }) = m4.original_message.clone()
            {
                if let Some(msg_context) = m4.original_msg_context.clone() {
                    match self.debit(price).await {
                        Ok(_) => {
                            let __message = Message::Result_ReceivePayment {
                                m4: m4.clone(),
                                from: from.clone(),
                                to: to.clone(),
                                price: price.clone(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(to, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedMessageContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedMessageError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
        Message::Result_ReceivePayment {
            m4,
            from,
            to,
            price,
        } => {
            if let Some(ActualMessage::Result { /*m4,*/ origin, result }) = m4.original_message.clone()
            {
                if let Some(msg_context) = m4.original_msg_context.clone() {
                    match self.credit(price).await {
                        Ok(_) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: "Ok(ExecutionResult :: default ())".to_string(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: "Ok(ExecutionResult :: default ())".to_string(),
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                        Err(error) => {
                            let __message = Message::Result {
                                m4: m4.clone(),
                                origin: system_api::current_chain_id(),
                                result: format! { "{:?}" , error },
                            };
                            return Ok(ExecutionResult::default()
                                .with_authenticated_message(m4.original_chain, __message));
                        }
                    }
                } else {
                    let __message = Message::Result {
                        m4: m4.clone(),
                        origin: system_api::current_chain_id(),
                        result: "M4MismatchedMessageContextError".to_string(),
                    };
                    Ok(ExecutionResult::default()
                        .with_authenticated_message(m4.original_chain, __message))
                }
            } else {
                let __message = Message::Result {
                    m4: m4.clone(),
                    origin: system_api::current_chain_id(),
                    result: "M4MismatchedMessageError".to_string(),
                };
                Ok(ExecutionResult::default()
                    .with_authenticated_message(m4.original_chain, __message))
            }
        }
//m4 execute_message end
//rmb to change: original_message: Some(ActualMessage::Result { m4: serde_json::to_string(&m4) , origin, result }),
        }
    }

    async fn handle_application_call(
        &mut self,
        _context: &CalleeContext,
        _argument: (),
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult<Self::Message, Self::Response, Self::SessionState>, Self::Error> {
        Ok(ApplicationCallResult::default())
    }

    async fn handle_session_call(
        &mut self,
        _context: &CalleeContext,
        _session: (),
        _argument: (),
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult<Self::Message, Self::Response, Self::SessionState>, Self::Error> {
        Ok(SessionCallResult::default())
    }
}

impl Aqueduct {
    async fn get_owner(&mut self, token: TokenId) -> Result<ChainId, Error> {
        if let Some(owner) = self.current_owner_minted.get(&token).await? {
            Ok(owner)
        } else {
            Err(Error::NoOwnerInMintedError)
        }
    }
    async fn get_token(&mut self, token_id: TokenId) -> Result<TokenMetadata, Error> {
        let temptoken = Token {
            id: TokenId {
                minted_chain: system_api::current_chain_id(),
                index: 0,
            },
            metadata: TokenMetadata {
                name: "".to_string(),
                description: "".to_string(),
                image: "".to_string(),
            },
        };
        let mut realtoken = temptoken.clone();
        self.owned_tokens.for_each_index(|key| {
            if key.id == token_id {
                realtoken = key;
            }
            Ok(())
        }).await?;
        if realtoken == temptoken {
            Err(Error::ThisChainDoesNotOwnThisTokenError)
        } else {
            Ok(realtoken.metadata)
        }
    }
    async fn check_own(&mut self, token: TokenId) -> Result<bool, Error> {
        let mut b = false;
        self.owned_tokens.for_each_index(|key| {
            b = b || key.id == token;
            Ok(())
        }).await?;
        Ok(b)
    }
    async fn on_erc721_received(&mut self, oer: OnERC721Received) -> Result<Vec<u8>, Error> {
        let mut b = to_bytes(&oer)?;
        //keccak256(&mut b);
        Ok(b.to_vec())
    }
    async fn transfer_from_me(&mut self, token: TokenId) -> Result<Token, Error> {
        let mut realtoken = Token {
            id: TokenId {
                minted_chain: system_api::current_chain_id(),
                index: 0,
            },
            metadata: TokenMetadata {
                name: "".to_string(),
                description: "".to_string(),
                image: "".to_string(),
            },
        };
        self.owned_tokens.for_each_index(|key| {
            if key.id == token {
                realtoken = key;
            }
            Ok(())
        }).await?;
        self.owned_tokens.remove(&realtoken)?;
        //self.token_approvals.remove(&token)?;
        Ok(realtoken)
    }
    async fn transfer_to_me(&mut self, token: &Token) -> Result<(), Error> {
        self.owned_tokens.insert(token)?;
        Ok(())
    }
    async fn transfer_update_minted(&mut self, token: TokenId, new: ChainId) -> Result<(), Error> {
        self.current_owner_minted.insert(&token, new)?;
        Ok(())
    }
    async fn get_price(&mut self, token_id: TokenId) -> Result<Amount, Error> {
        match self.listings.get(&token_id).await? {
            Some(price) => Ok(price),
            None => Err(Error::TokenNotListedError),
        }
    }
    /*async fn pay(&mut self, from: ChainId, amount: Amount, to: ChainId) -> Result<(), Self::Error> {
        let call: logging_fungible::ApplicationCall = logging_fungible::ApplicationCall::Transfer {
            owner: self.get_owner_of_chain(m4.original_chain),
            amount: price,
            destination: self.get_owner_of_chain(cur_owner),
        };
        self.call_application(Self::logging_fungible_id()?, &call, vec![]).await?;
        Ok(())
    }*/
    async fn credit(&mut self, amount: Amount) -> Result<(), Error> {
        let mut balance = self.balance().await;
        balance.saturating_add_assign(amount);
        self.balance.set(balance);
        Ok(())
    }

    async fn debit(
        &mut self,
        amount: Amount,
    ) -> Result<(), Error> {
        let mut balance = self.balance().await;
        balance
            .try_sub_assign(amount)
            .map_err(|_| Error::InsufficientBalanceError)?;
        self.balance.set(balance);
        Ok(())
    }

    fn logger_id() -> Result<ApplicationId<logger::LoggerAbi>, Error> {
        Ok(bcs::from_bytes::<ApplicationId>(&hex::decode(Self::parameters()?.logger_application_id)?)?.with_abi::<logger::LoggerAbi>())
    }


    async fn handle_receive(&mut self, stream_id: StreamId, to: ChainId) -> Result<Amount, Error> {
        let stream = self.nfts.get(&stream_id).await?;
        let mut amount_to_give = Amount::zero();
        if let Some(stream) = stream {
            let mut cur_milestone = stream.milestones_received;
            let mut cur_period = stream.periods_received;
            let mut time = stream.created;
            if cur_milestone >= stream.segments.len() as u64 {
                return Ok(amount_to_give);
            }
            if cur_milestone > 0 { time += stream.segments[(cur_milestone - 1) as usize].milestone; }
            time += stream.segments[cur_milestone as usize].period * cur_period;
            let mut prev_milestone = 0;
            while time < system_api::current_system_time().micros() && cur_milestone < stream.segments.len() as u64 {
                let mut endtime = time + stream.segments[cur_milestone as usize].period;
                if endtime > stream.segments[cur_milestone as usize].milestone {
                    endtime = stream.segments[cur_milestone as usize].milestone;
                }
                for keyword in &stream.keywords {
                    let call = logger::ApplicationCall::Query {
                        log_type: Some(logger::LogType::OperationExecutionStart),
                        keyword: keyword.to_string(),
                        app: None,
                        app_name: None,
                        timestamp: Some((Timestamp::from(time), Timestamp::from(endtime))),
                        function_name: None,
                    };
                    if let Ok((log, _)) = self.call_application(true, Self::logger_id()?, &call, vec![]).await {
                        for log_statement in log {
                            if let Ok(Operation::Income { amount, keyword }) = serde_json::from_str::<Operation>(&log_statement.log) {
                                let cons: f64 = (stream.segments[cur_milestone as usize].constant as f64 ) / 1000000000000000000.;
                                let fact: f64 = (stream.segments[cur_milestone as usize].factor as f64 ) / 1000000000000000000.;
                                let expo: f64 = (stream.segments[cur_milestone as usize].exponent as f64 ) / 1000000000000000000.;
                                let var1=(((time - prev_milestone) as f64) / ((stream.segments[cur_milestone as usize].milestone - prev_milestone) as f64));
                                amount_to_give.saturating_add_assign(amount.saturating_mul((cons + fact * var1.powf(expo)) as u128));
                            }
                        }
                    }
                }
                time = endtime;
                cur_period += 1;
                if time == stream.segments[cur_milestone as usize].milestone {
                    prev_milestone = stream.segments[cur_milestone as usize].milestone;
                    cur_milestone += 1;
                    cur_period = 0;
                }
            }
            self.nfts.insert(&stream_id, Stream {
                keywords: stream.keywords,
                segments: stream.segments,
                created: stream.created,
                milestones_received: cur_milestone,
                periods_received: cur_period,
            });
            Ok(amount_to_give)
        } else {
            Err(Error::CompanyDoesntHaveStreamError)
        }
    }
}

#[derive(Serialize)]
struct OnERC721Received {
    from: ChainId,
    to: ChainId,
    token_id: TokenId,
    data: Vec<u8>,
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to deserialize BCS bytes
    #[error("Failed to deserialize BCS bytes {0}")]
    BcsError(#[from] bcs::Error),

    /// Failed to deserialize JSON string
    #[error("Failed to deserialize JSON string")]
    JsonError(#[from] serde_json::Error),

    #[error("view error {0}")]
    ViewError(#[from] linera_sdk::views::views::ViewError),

    #[error("token is not listed by owner")]
    TokenNotListedError,

    #[error("insufficient balance")]
    InsufficientBalanceError,

    #[error("company did not mint a nft with this streamid")]
    CompanyDoesntHaveStreamError,

    #[error("minted chain doesnt list owner of a token it minted")]
    NoOwnerInMintedError,

    #[error("this chain dopes not own this tojken")]
    ThisChainDoesNotOwnThisTokenError,
//m4 errors start

    #[error("original operation somehow got lost during messages")]
    M4MismatchedOperationError,
    #[error("original operation context somehow got lost during messages")]
    M4MismatchedOperationContextError,
    #[error("original message somehow got lost during messages")]
    M4MismatchedMessageError,
    #[error("original message context somehow got lost during messages")]
    M4MismatchedMessageContextError,

//m4 errors end

    
    #[error("how did u even get this utf8 error (parameter)")]
    Utf8Error(#[from] Utf8Error),

    #[error("ur crate weird {0}")]
    FindCrateError(#[from] find_crate::Error),

    #[error("hecks {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("cannot read ur toml {0}")]
    IoError(#[from] std::io::Error),

    #[error("cannot deserialize ur toml {0}")]
    TomlDeError(#[from] toml::de::Error),

    #[error("aaaaaaaaaaaaaaaaaaaaaaa")]
    NoRequiredIdsError,

    #[error("wheres ur manifest dir")]
    NotFoundManifestDir,

    #[error("wheres ur crate name in cargo.toml bruh")]
    NoNameInCargoToml,

}
