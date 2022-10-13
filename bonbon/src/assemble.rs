use {
    borsh::de::BorshDeserialize,
    mpl_token_metadata::{
        instruction::MetadataInstruction, pda::find_metadata_account,
        state::Collection as MplCollection,
        state::Creator as MplCreator,
    },
    solana_sdk::{instruction::CompiledInstruction, program_option::COption, pubkey::Pubkey},
    spl_token_2022::instruction::{AuthorityType, TokenInstruction},
    std::collections::HashMap,
};

#[cfg(feature = "serde-feature")]
use {
    serde_with::{As, DisplayFromStr},
    serde::{Deserialize, Serialize},
};

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum EditionStatus {
    // Edition has not been created. This state is used temporarily for every NFT we encounter
    // since the metadata must be created before the edition, but it could also be an...
    // - SFT
    // - NFT where mint auth is held by e.g cardinal
    None,

    Master,

    Limited,
}

impl Default for EditionStatus {
    fn default() -> Self {
        Self::None
    }
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct LimitedEdition {
    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub master_key: Pubkey,

    // TODO: this is kind of a PITA to track correctly with the old method. Punt for now
    // (Option)
    pub edition_num: Option<i64>,

    pub instruction_index: InstructionIndex,
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Creator {
    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub address: Pubkey,

    pub verified: bool,

    pub share: i16,
}

impl From<MplCreator> for Creator {
    fn from(creator: MplCreator) -> Self {
        Self {
            address: creator.address,
            verified: creator.verified,
            share: creator.share.into(),
        }
    }
}

fn from_creators(creators: Option<Vec<MplCreator>>) -> Vec<Creator> {
    creators
        .unwrap_or(vec![])
        .into_iter()
        .map(Creator::from)
        .collect()
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Collection {
    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub address: Pubkey,

    pub verified: bool,
}

impl From<MplCollection> for Collection {
    fn from(creator: MplCollection) -> Self {
        Self {
            address: creator.key,
            verified: creator.verified,
        }
    }
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(PartialOrd, Ord, PartialEq, Eq, Default, Debug, Clone)]
pub struct InstructionIndex {
    pub slot: i64,

    pub block_index: i64,

    pub outer_index: i64,

    pub inner_index: Option<i64>,
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone)]
pub struct Ownership {
    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub owner: Pubkey,

    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub account: Pubkey,
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone)]
pub struct Transfer {
    pub slot: i64,
    pub start: Option<Ownership>, // first transfer starts for None
    pub end: Option<Ownership>,   // end can be None after burn
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone)]
pub struct Glazing {
    pub name: String,

    pub symbol: String,

    pub uri: String,

    pub creators: Vec<Creator>,

    pub collection: Option<Collection>,

    pub instruction_index: InstructionIndex,
}

#[cfg_attr(feature = "serde-feature", derive(Serialize, Deserialize))]
#[derive(Default, Debug)]
pub struct Bonbon {
    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub mint_key: Pubkey, // could be pubkey::default

    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub metadata_key: Pubkey, // could be pubkey::default

    #[cfg_attr(
        feature = "serde-feature",
        serde(with = "As::<DisplayFromStr>")
    )]
    pub mint_authority: Pubkey, // could be pubkey::default

    pub transfers: Vec<Transfer>,

    pub current_owner: Option<Ownership>,

    pub edition_status: EditionStatus,

    pub limited_edition: Option<LimitedEdition>,

    // we add a record of updates so that we can join up values at the end by slot/block/indexes.
    // track creator / collection verification and override those with the new values for the
    // limited edition
    pub glazings: Vec<Glazing>,

    // mapping of token account to owner
    // TODO: this is a bit of a hack. We need to track the owner of the token account and normally
    // rely on pre/postTokenBalances but those aren't available around block ~80M so...
    #[cfg_attr(
        feature = "serde-feature",
        serde(skip)
    )]
    ownerships: HashMap<Pubkey, Pubkey>,
}

impl std::fmt::Display for Bonbon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bonbon")
            .field("mint_key", &self.mint_key)
            .field("metadata_key", &self.metadata_key)
            .field("mint_authority", &self.mint_authority)
            .field("transfers", &self.transfers)
            .field("current_owner", &self.current_owner)
            .field("edition_status", &self.edition_status)
            .field("limited_edition", &self.limited_edition)
            .field("glazings", &self.glazings)
            .finish()
    }
}

impl Bonbon {
    pub fn apply_creator_verification(
        &mut self,
        creator_key: Pubkey,
        verified: bool,
        instruction_index: InstructionIndex,
    ) {
        if let Some(last) = self.glazings.last() {
            let mut next: Glazing = last.clone();
            for creator in &mut next.creators {
                if creator.address == creator_key {
                    creator.verified = verified;
                    break;
                }
            }
            next.instruction_index = instruction_index;
            self.glazings.push(next);
        } else {
            self.glazings.push(Glazing {
                creators: vec![Creator {
                    address: creator_key,
                    verified,
                    share: 0,
                }],
                instruction_index,
                ..Glazing::default()
            });
        }
    }

    pub fn apply_collection_verification(
        &mut self,
        collection_key: Pubkey,
        verified: bool,
        instruction_index: InstructionIndex,
    ) {
        let prev = self
            .glazings
            .last()
            .map(|v| v.clone())
            .unwrap_or(Glazing::default());
        self.glazings.push(Glazing {
            collection: Some(Collection {
                address: collection_key,
                verified,
            }),
            instruction_index,
            ..prev
        })
    }

    pub fn apply_ownership(&mut self, new_owner: Option<Ownership>, slot: i64) {
        if let Some(current_owner) = &self.current_owner {
            let t = Transfer {
                slot,
                start: Some(current_owner.clone()),
                end: new_owner.clone(),
            };
            self.transfers.push(t);
            self.current_owner = new_owner;
        } else {
            // first transfer
            let o = new_owner;
            let t = Transfer {
                slot,
                start: None,
                end: o.clone(),
            };
            self.transfers.push(t);
            self.current_owner = o;
        }
    }
}

#[derive(Debug)]
pub enum ErrorCode {
    BadAccountKeyIndex,

    FailedInstructionDeserialization,

    InvalidMetadataCreate,

    InvalidMetadataUpdate,

    InvalidMasterEditionCreate,

    // includes unverify creator/collection
    InvalidMetadataVerifyOperation,

    CouldNotFindTokenAccountOwner,
}

#[derive(Clone)]
pub struct TransactionTokenOwnerMeta {
    pub account_index: u8,

    pub owner_key: Option<Pubkey>,
}

pub trait Cocoa {
    fn program_key(&self, account_keys: &[Pubkey]) -> Result<Pubkey, ErrorCode>;
    fn account_index(&self, index: usize) -> Result<u8, ErrorCode>;
    fn roast(&self) -> Result<MetadataInstruction, ErrorCode>;
    fn bake(&self) -> Result<TokenInstruction, ErrorCode>;

    fn account(&self, index: usize, account_keys: &[Pubkey]) -> Result<Pubkey, ErrorCode> {
        account_keys
            .get(usize::from(self.account_index(index)?))
            .map(|k| *k)
            .ok_or(ErrorCode::BadAccountKeyIndex)
    }
}

impl Cocoa for CompiledInstruction {
    fn program_key(&self, account_keys: &[Pubkey]) -> Result<Pubkey, ErrorCode> {
        account_keys
            .get(usize::from(self.program_id_index))
            .map(|k| *k)
            .ok_or(ErrorCode::BadAccountKeyIndex)
    }

    fn account_index(&self, index: usize) -> Result<u8, ErrorCode> {
        self.accounts
            .get(index)
            .map(|v| *v)
            .ok_or(ErrorCode::BadAccountKeyIndex)
    }

    fn roast(&self) -> Result<MetadataInstruction, ErrorCode> {
        MetadataInstruction::try_from_slice(&self.data)
            .map_err(|_| ErrorCode::FailedInstructionDeserialization)
    }

    fn bake(&self) -> Result<TokenInstruction, ErrorCode> {
        TokenInstruction::unpack(&self.data)
            .map_err(|_| ErrorCode::FailedInstructionDeserialization)
    }
}

pub struct InstructionContext<'a, T: Cocoa> {
    pub instruction: &'a T,

    pub account_keys: &'a [Pubkey],

    pub owners: &'a [TransactionTokenOwnerMeta],

    pub instruction_index: InstructionIndex,

    pub transient_metas: &'a mut Vec<TransactionTokenOwnerMeta>,
}

trait IntoGlazing {
    fn into_glazing(self, instruction_index: InstructionIndex) -> Glazing;
}

impl IntoGlazing for mpl_token_metadata::state::Data {
    fn into_glazing(self, instruction_index: InstructionIndex) -> Glazing {
        Glazing {
            name: self.name,
            symbol: self.symbol,
            uri: self.uri,
            creators: from_creators(self.creators),
            collection: None,
            instruction_index,
        }
    }
}

impl IntoGlazing for mpl_token_metadata::state::DataV2 {
    fn into_glazing(self, instruction_index: InstructionIndex) -> Glazing {
        Glazing {
            name: self.name,
            symbol: self.symbol,
            uri: self.uri,
            creators: from_creators(self.creators),
            collection: self.collection.map(Collection::from),
            instruction_index,
        }
    }
}

pub fn update_metadata_instruction<T: Cocoa>(
    bonbon: &mut Bonbon,
    InstructionContext {
        instruction,
        account_keys,
        owners: _,
        instruction_index,
        transient_metas: _,
    }: InstructionContext<T>,
) -> Result<(), ErrorCode> {
    let get_account_key = |index: usize| instruction.account(index, account_keys);

    let metadata_instruction = instruction.roast()?;

    match metadata_instruction {
        MetadataInstruction::CreateMetadataAccount(args) => {
            // OG create metadata
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = metadata_key;
            bonbon.glazings.push(args.data.into_glazing(instruction_index));
        }
        MetadataInstruction::CreateMetadataAccountV2(args) => {
            // create metadata with datav2 (adds collection info, etc)
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = metadata_key;
            bonbon.glazings.push(args.data.into_glazing(instruction_index));
        }
        MetadataInstruction::UpdateMetadataAccount(args) => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataUpdate);
            }

            if let Some(data) = args.data {
                bonbon.glazings.push(data.into_glazing(instruction_index));
            }
        }
        MetadataInstruction::UpdateMetadataAccountV2(args) => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataUpdate);
            }

            if let Some(data) = args.data {
                bonbon.glazings.push(data.into_glazing(instruction_index));
            }
        }
        MetadataInstruction::DeprecatedCreateMasterEdition(_) => {
            // master edition with printing tokens (and reservation list?)
            let metadata_key = get_account_key(7)?;
            if bonbon.metadata_key != metadata_key || bonbon.edition_status != EditionStatus::None {
                return Err(ErrorCode::InvalidMasterEditionCreate);
            }

            bonbon.edition_status = EditionStatus::Master;
        }
        MetadataInstruction::CreateMasterEdition(_) => {
            // edition v2 w/ bitvec directly
            let metadata_key = get_account_key(5)?;
            if bonbon.metadata_key != metadata_key || bonbon.edition_status != EditionStatus::None {
                return Err(ErrorCode::InvalidMasterEditionCreate);
            }

            bonbon.edition_status = EditionStatus::Master;
        }
        MetadataInstruction::CreateMasterEditionV3(_) => {
            // not sure why this exists
            let metadata_key = get_account_key(5)?;
            if bonbon.metadata_key != metadata_key || bonbon.edition_status != EditionStatus::None {
                return Err(ErrorCode::InvalidMasterEditionCreate);
            }

            bonbon.edition_status = EditionStatus::Master;
        }
        MetadataInstruction::DeprecatedMintNewEditionFromMasterEditionViaPrintingToken => {
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = metadata_key;
            bonbon.edition_status = EditionStatus::Limited;
            bonbon.limited_edition = Some(LimitedEdition {
                master_key: get_account_key(11)?,
                edition_num: None,
                instruction_index,
            });
        }
        MetadataInstruction::MintNewEditionFromMasterEditionViaToken(args) => {
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = metadata_key;
            bonbon.edition_status = EditionStatus::Limited;
            bonbon.limited_edition = Some(LimitedEdition {
                master_key: get_account_key(10)?,
                edition_num: Some(args.edition as i64),
                instruction_index,
            });
        }
        MetadataInstruction::MintNewEditionFromMasterEditionViaVaultProxy(args) => {
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = metadata_key;
            bonbon.edition_status = EditionStatus::Limited;
            bonbon.limited_edition = Some(LimitedEdition {
                master_key: get_account_key(12)?,
                edition_num: Some(args.edition as i64),
                instruction_index,
            });
        }
        MetadataInstruction::SignMetadata => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let creator_key = get_account_key(1)?;
            bonbon.apply_creator_verification(creator_key, true, instruction_index);
        }
        MetadataInstruction::RemoveCreatorVerification => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let creator_key = get_account_key(1)?;
            bonbon.apply_creator_verification(creator_key, false, instruction_index);
        }
        MetadataInstruction::VerifyCollection => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let collection_key = get_account_key(3)?;
            bonbon.apply_collection_verification(collection_key, true, instruction_index);
        }
        MetadataInstruction::SetAndVerifyCollection => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let collection_key = get_account_key(4)?;
            bonbon.apply_collection_verification(collection_key, true, instruction_index);
        }
        MetadataInstruction::UnverifyCollection => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let collection_key = get_account_key(3)?;
            bonbon.apply_collection_verification(collection_key, false, instruction_index);
        }
        MetadataInstruction::BurnNft => {
            bonbon.apply_ownership(None, instruction_index.slot);
        }
        MetadataInstruction::VerifySizedCollectionItem => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let collection_key = get_account_key(3)?;
            bonbon.apply_collection_verification(collection_key, true, instruction_index);
        }
        MetadataInstruction::UnverifySizedCollectionItem => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let collection_key = get_account_key(3)?;
            bonbon.apply_collection_verification(collection_key, false, instruction_index);
        }
        MetadataInstruction::SetAndVerifySizedCollectionItem => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let collection_key = get_account_key(4)?;
            bonbon.apply_collection_verification(collection_key, true, instruction_index);
        }
        MetadataInstruction::CreateMetadataAccountV3(args) => {
            // with collection details if parent collection NFT
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            // ignore collection details
            // bonbon.collection_details = args.collection_details.map(|cd| cd.into());
            bonbon.metadata_key = metadata_key;
            bonbon.glazings.push(args.data.into_glazing(instruction_index));
        }
        MetadataInstruction::UpdatePrimarySaleHappenedViaToken => {}
        MetadataInstruction::DeprecatedSetReservationList(_) => {}
        MetadataInstruction::DeprecatedCreateReservationList => {}
        MetadataInstruction::DeprecatedMintPrintingTokensViaToken(_) => {}
        MetadataInstruction::DeprecatedMintPrintingTokens(_) => {}
        MetadataInstruction::ConvertMasterEditionV1ToV2 => {}
        MetadataInstruction::PuffMetadata => {}
        MetadataInstruction::Utilize(_) => {}
        MetadataInstruction::ApproveUseAuthority(_) => {}
        MetadataInstruction::RevokeUseAuthority => {}
        MetadataInstruction::ApproveCollectionAuthority => {}
        MetadataInstruction::RevokeCollectionAuthority => {}
        MetadataInstruction::FreezeDelegatedAccount => {}
        MetadataInstruction::ThawDelegatedAccount => {}
        MetadataInstruction::SetCollectionSize(_) => {}
        MetadataInstruction::SetTokenStandard => {} // TODO?
    }

    Ok(())
}

pub fn update_token_instruction<T: Cocoa>(
    bonbon: &mut Bonbon,
    InstructionContext {
        instruction,
        account_keys,
        owners,
        instruction_index,
        transient_metas,
    }: InstructionContext<T>,
) -> Result<(), ErrorCode> {
    let get_account_key = |index: usize| instruction.account(index, account_keys);

    let get_token_meta_for = |index: usize| {
        let index = instruction.account_index(index)?;
        if let Some(v) = owners.iter().find(|m| m.account_index == index) {
            Ok(v)
        } else {
            transient_metas.iter().find(|m| m.account_index == index)
                .ok_or(ErrorCode::BadAccountKeyIndex)
        }
    };

    let token_instruction = instruction.bake()?;

    match token_instruction {
        TokenInstruction::InitializeMint { .. } => {
            bonbon.mint_key = get_account_key(0)?;
        }
        // initializing an account doesn't change who currently owns it
        TokenInstruction::InitializeAccount { .. } => {
            let account_key = get_account_key(0)?;
            let owner_key = get_account_key(2)?;
            bonbon.ownerships.insert(
                account_key,
                owner_key,
            );
            if let Err(_) = get_token_meta_for(0) {
                transient_metas.push(TransactionTokenOwnerMeta {
                    account_index: instruction.account_index(0)?,
                    owner_key: Some(owner_key),
                });
            }
        }
        TokenInstruction::InitializeAccount2 { .. } => {
            let account_key = get_account_key(0)?;
            let owner_key = get_account_key(2)?;
            bonbon.ownerships.insert(
                account_key,
                owner_key,
            );
            if let Err(_) = get_token_meta_for(0) {
                transient_metas.push(TransactionTokenOwnerMeta {
                    account_index: instruction.account_index(0)?,
                    owner_key: Some(owner_key),
                });
            }
        }
        #[allow(deprecated)]
        TokenInstruction::Transfer { .. } => {
            let new_owner = get_token_meta_for(1)?.owner_key;
            let new_account = get_account_key(1)?;
            bonbon.apply_ownership(
                Some(Ownership {
                    owner: new_owner.or_else(
                        || bonbon.ownerships.get(&new_account).cloned())
                        .ok_or(ErrorCode::CouldNotFindTokenAccountOwner)?,
                    account: new_account,
                }),
                instruction_index.slot,
            );
        }
        TokenInstruction::SetAuthority {
            authority_type,
            new_authority,
        } => {
            match authority_type {
                AuthorityType::AccountOwner => {
                    // no account change. owner changes though possibly
                    if let COption::Some(new_authority) = new_authority {
                        bonbon.apply_ownership(
                            Some(Ownership {
                                owner: new_authority,
                                account: get_account_key(0)?,
                            }),
                            instruction_index.slot,
                        );
                    }
                }
                _ => {}
            }
        }
        TokenInstruction::MintTo { .. } => {
            let new_owner = get_token_meta_for(1)?.owner_key;
            let new_account = get_account_key(1)?;
            bonbon.apply_ownership(
                Some(Ownership {
                    owner: new_owner.or_else(
                        || bonbon.ownerships.get(&new_account).cloned())
                        .ok_or(ErrorCode::CouldNotFindTokenAccountOwner)?,
                    account: new_account,
                }),
                instruction_index.slot,
            );
            bonbon.mint_authority = get_account_key(2)?;
        }
        TokenInstruction::Burn { .. } => {
            bonbon.apply_ownership(None, instruction_index.slot);
        }
        TokenInstruction::TransferChecked { .. } => {
            let new_owner = get_token_meta_for(2)?.owner_key;
            let new_account = get_account_key(2)?;
            bonbon.apply_ownership(
                Some(Ownership {
                    owner: new_owner.or_else(
                        || bonbon.ownerships.get(&new_account).cloned())
                        .ok_or(ErrorCode::CouldNotFindTokenAccountOwner)?,
                    account: new_account,
                }),
                instruction_index.slot,
            );
        }
        TokenInstruction::MintToChecked { .. } => {
            let new_owner = get_token_meta_for(1)?.owner_key;
            let new_account = get_account_key(1)?;
            bonbon.apply_ownership(
                Some(Ownership {
                    owner: new_owner.or_else(
                        || bonbon.ownerships.get(&new_account).cloned())
                        .ok_or(ErrorCode::CouldNotFindTokenAccountOwner)?,
                    account: new_account,
                }),
                instruction_index.slot,
            );
            bonbon.mint_authority = get_account_key(2)?;
        }
        TokenInstruction::BurnChecked { .. } => {
            bonbon.apply_ownership(None, instruction_index.slot);
        }
        TokenInstruction::InitializeMultisig { .. } => {}
        TokenInstruction::Approve { .. } => {}
        TokenInstruction::Revoke => {}
        TokenInstruction::CloseAccount => {
            // mints can't be closed and a token account must have zero balance to be closed so...
            bonbon.ownerships.remove(&get_account_key(0)?);
            let account_index = instruction.account_index(0)?;
            if let Some(index) = transient_metas
                .iter()
                .position(|m| m.account_index == account_index)
            {
                transient_metas.swap_remove(index);
            }
        }
        TokenInstruction::FreezeAccount => {}
        TokenInstruction::ThawAccount => {}
        TokenInstruction::ApproveChecked { .. } => {}
        TokenInstruction::SyncNative => {}
        TokenInstruction::InitializeAccount3 { owner: owner_key } => {
            let account_key = get_account_key(0)?;
            bonbon.ownerships.insert(
                account_key,
                owner_key,
            );
            if let Err(_) = get_token_meta_for(0) {
                transient_metas.push(TransactionTokenOwnerMeta {
                    account_index: instruction.account_index(0)?,
                    owner_key: Some(owner_key),
                });
            }
        }
        TokenInstruction::InitializeMultisig2 { .. } => {}
        TokenInstruction::InitializeMint2 { .. } => {
            bonbon.mint_key = get_account_key(0)?;
        }

        // none of the token-2022 info gets passed out of partition
        TokenInstruction::GetAccountDataSize { .. } => {},
        TokenInstruction::AmountToUiAmount { .. } => {},
        TokenInstruction::UiAmountToAmount { .. } => {},
        TokenInstruction::TransferFeeExtension(..) => {},
        TokenInstruction::ConfidentialTransferExtension => {},
        TokenInstruction::DefaultAccountStateExtension => {},
        TokenInstruction::MemoTransferExtension => {},
        TokenInstruction::InterestBearingMintExtension => {},
        TokenInstruction::Reallocate { .. } => {},
        TokenInstruction::CreateNativeMint => {},
        TokenInstruction::InitializeImmutableOwner => {},
        TokenInstruction::InitializeMintCloseAuthority { .. } => {},
        TokenInstruction::InitializeNonTransferableMint => {},
    }

    Ok(())
}

pub struct BonbonUpdater<T: Cocoa> {
    pub program_id: Pubkey,

    pub update: fn(
        bonbon: &mut Bonbon,
        instruction_context: InstructionContext<T>,
    ) -> Result<(), ErrorCode>,
}

impl Bonbon {
    pub fn update<T: Cocoa>(
        &mut self,
        instruction_context @ InstructionContext {
            instruction,
            account_keys,
            ..
        }: InstructionContext<T>,
        updaters: &[BonbonUpdater<T>],
    ) -> Result<(), ErrorCode> {
        let program_id = instruction.program_key(account_keys)?;

        if let Some(BonbonUpdater { update, .. }) =
            updaters.iter().find(|u| u.program_id == program_id)
        {
            update(self, instruction_context)
        } else {
            Ok(())
        }
    }
}
