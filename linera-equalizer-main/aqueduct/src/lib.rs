#![allow(non_camel_case_types)]
use linera_sdk::base::{ContractAbi, ServiceAbi, ChainId, Amount};
use linera_sdk::{OperationContext, MessageContext, ExecutionResult};
use serde::{Serialize, Deserialize};
use async_graphql::{scalar, SimpleObject, InputObject, Request, Response, Object};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct AqueductAbi;

impl ContractAbi for AqueductAbi {
    type InitializationArgument = ();
    type Parameters = Parameter;
    type Operation = Operation;
    type ApplicationCall = ();
    type Message = Message;
    type SessionCall = ();
    type Response = ();
    type SessionState = ();
}

impl ServiceAbi for AqueductAbi {
    type Query = Request;
    type QueryResponse = Response;
    type Parameters = ();
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
//#[graphql(input_name = "TokenId")]
pub struct TokenId {
    pub minted_chain: ChainId,
    pub index: u64,
}

scalar!(TokenId);

#[derive(Debug, Deserialize, Serialize, Clone, SimpleObject, PartialEq, Eq)]
pub struct TokenMetadata {
    pub name: String,
    pub description: String,
    pub image: String,  //can be link to any asset, just following ERC721 Metadata JSON Schema for now
}

#[derive(Debug, Deserialize, Serialize, Clone, SimpleObject, PartialEq, Eq)]
pub struct Token {
    pub id: TokenId,
    pub metadata: TokenMetadata,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Parameter {
    pub logger_application_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Segment {
    pub period: u64,   //in milliseconds
    pub constant: i64,  //1 is 10^18
    pub factor: i64,    //1 is 10^18
    pub exponent: i64,  //1 is 10^18
    pub milestone: u64,//in milliseconds
}

scalar!(Segment);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StreamId {
    pub company: ChainId,
    pub id: String,     //from uuid_v4
}

scalar!(StreamId);

/*#[Object]
impl StreamId {
    async fn company(&self) -> ChainId { self.company }
    async fn id(&self) -> String { self.id.clone() }
}*/

#[derive(Debug, Deserialize, Serialize, Clone, SimpleObject)]
pub struct Stream {
    pub keywords: Vec<String>,  //so you can have different streams like selling one for selling
                                //product a and one for selling product b
    pub segments: Vec<Segment>,
    pub created: u64,           //number of non-leap milliseconds since 1 1 1970 UTC
    pub milestones_received: u64,
    pub periods_received: u64,  //number of periods received this current milestone
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Operation {
    Mint {
        name: String,
        description: String,
        keywords: Vec<String>,
        segments: Vec<Segment>,
    },
    Burn {
        token_id: TokenId,
    },
    List {
        token_id: TokenId,
        amount: Amount,
    },
    Cancel {
        token_id: TokenId,
    },
    Buy {
        token_id: TokenId,
    },
    Receive {
        token_id: TokenId,
    },
    Income {
        amount: Amount,
        keyword: String,
    },
}


//m4 M4 start

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct M4 {
    pub original_chain: ChainId,
    pub original_operation: Option<Operation>,
    pub original_ope_context: Option<OperationContext>,
    pub original_message: Option<ActualMessage>,
    pub original_msg_context: Option<MessageContext>,
}

//m4 M4 end

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub enum Message {
    #[default]
    Default,
    SafeTransferFrom {
        from: ChainId,
        to: ChainId,
        token_id: TokenId,
        data: Vec<u8>,
    },
//m4 Message start
    Result {
        m4: M4,
        origin: ChainId,
        result: String,
    },
    Buy_GetOwner {
        m4: M4,
    },
    Buy_GetPrice {
        m4: M4,
        cur_owner: ChainId,
    },
    Buy_BackToPay {
        m4: M4,
        cur_owner: ChainId,
        price: Amount,
    },
    Buy_ReceivePayment {
        m4: M4,
        cur_owner: ChainId,
        price: Amount,
    },
    Buy_ReceiveRollbackPayment {
        m4: M4,
        cur_owner: ChainId,
        price: Amount,
    },
    Receive_Company {
        m4: M4,
        metadata: TokenMetadata,
        stream_id: StreamId,
    },
    SafeTransferFrom_CheckOwn {
        m4: M4,
        sender: ChainId,
    },
    SafeTransferFrom_TransferFrom {
        m4: M4,
        sender: ChainId,
        own: bool,
    },
    SafeTransferFrom_TransferTo {
        m4: M4,
        sender: ChainId,
        own: bool,
        token: Token,
    },
    SafeTransferFrom_UpdateMintedChain {
        m4: M4,
        sender: ChainId,
        own: bool,
        token: Token,
    },
    SafeTransferFrom_OnERC721Received {
        m4: M4,
        sender: ChainId,
        own: bool,
        token: Token,
    },
    Result_BackToPay {
        m4: M4,
        from: ChainId,
        to: ChainId,
        price: Amount,
    },
    Result_ReceivePayment {
        m4: M4,
        from: ChainId,
        to: ChainId,
        price: Amount,
    },
//m4 Message end
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ActualMessage{
    SafeTransferFrom {
        from: ChainId,
        to: ChainId,
        token_id: TokenId,
        data: Vec<u8>,
    },
    Result {
        origin: ChainId,
        result: String,
    },
}
