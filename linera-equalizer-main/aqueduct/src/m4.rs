match operation {
    Operation::Buy {
        token_id,
    } => {
        let cur_owner: ChainId = #[message(GetOwner)](token_id.minted_chain, self.get_owner(token_id).await);
        let price: Amount = #[message(GetPrice)](cur_owner, self.get_price(token_id).await);
        #[message(BackToPay)](m4.original_chain, self.debit(price).await);
        #[message(ReceivePayment)](cur_owner, self.credit(price).await);
        if let Ok(data) = to_bytes(&price) {
            let message: Message = Message::SafeTransferFrom {
                from: cur_owner, 
                to: m4.original_chain,
                token_id,
                data,
            };
            //rollback moni if fail (implemented by hand after generating m4)
            return Ok(ExecutionResult::default().with_authenticated_message(cur_owner, message));
        } else {
            self.debit(price).await;
            #[message(ReceiveRollbackPayment)](cur_owner, self.credit(price).await);
            return Err(Error::BcsPriceError);
        }
    },
    Operation::Receive {
        token_id,
    } => {
        let metadata: TokenMetadata = self.get_token(token_id).await?;
        if let Ok(stream_id) = serde_json::from_str::<StreamId>(&metadata.image) {
            #[addvar(stream_id: StreamId)]
            #[message(Company)](stream_id.company.clone(), self.handle_receive(stream_id, m4.original_chain).await);
            return Ok(ExecutionResult::default());
        } else {
            return Err(Error::JsonError);
        }
    },
}

match message {
    Message::SafeTransferFrom {
        from,
        to,
        token_id,
        data,
    } => {
        let sender: ChainId = system_api::current_chain_id();
        if from != sender {
            return Err(Error::NotAnOperatorNorApproved);
        }
        let own: bool = #[message(CheckOwn)](from, self.check_own(token_id).await);
        if !own { return Err(Error::DoesNotOwnToken); }
        // throws if to is zero address
        // throws if tokenid is not a valid nft
        let token: Token = #[message(TransferFrom)](from, self.transfer_from_me(token_id).await);
        #[message(TransferTo)](to, self.transfer_to_me(&token).await);
        #[message(UpdateMintedChain)](token_id.minted_chain, self.transfer_update_minted(token_id, to).await);
        // rollback if something goes wrong or check fails?
        let check: Vec<u8> = #[message(OnERC721Received)](to, self.on_erc721_received(OnERC721Received {
            from: from.clone(), 
            to: to.clone(),
            token_id: token_id.clone(),
            data: data.clone(),
        }).await);
        if let Ok(b) = to_bytes(&OnERC721Received {
            from: from.clone(), 
            to: to.clone(),
            token_id: token_id.clone(),
            data: data.clone(),
        }) {
            let mut bb = b;
            keccak256(&mut bb);
            if check != bb { return Err(Error::AfterTransferCheckFailed); }
            return Ok(ExecutionResult::default());
        } else {
            return Err(Error::BcsError);
        }
    },
    Message::Result { m4, origin, result } => {
        info!("m4: {:?} origin: {} res: {}", m4, origin, result);
        if let Some(ActualMessage::SafeTransferFrom {
                from,
                to,
                token_id,
                data,
            }) = m4.original_message {
            #[addvar(from: ChainId)]
            #[addvar(to: ChainId)]
            if result.contains("Err") {
                if let Ok(price) = from_bytes::<Amount>(&data) {
                    #[addvar(price: Amount)]
                    #[message(BackToPay)](from, self.debit(price).await);
                    #[message(ReceivePayment)](to, self.credit(price).await);
                    return Ok(ExecutionResult::default());
                } else {
                    return Err(Error::BcsError);
                }
            }
        }
        return Ok(ExecutionResult::default())
    }
}
