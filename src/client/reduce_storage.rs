use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use byte_unit::Byte;
use shadow_drive_user_staking::accounts as shdw_drive_accounts;
use shadow_drive_user_staking::instruction as shdw_drive_instructions;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::sysvar::rent;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signer::Signer, transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::ID as TokenProgramID;

use super::ShadowDriveClient;
use crate::constants::EMISSIONS;
use crate::{
    constants::{PROGRAM_ADDRESS, STORAGE_CONFIG_PDA, TOKEN_MINT},
    derived_addresses,
    error::Error,
    models::*,
};

impl<T> ShadowDriveClient<T>
where
    T: Signer + Send + Sync,
{
    /// Reduces the amount of total storage available for the given storage account.
    /// * `storage_account_key` - The public key of the [`StorageAccount`] whose storage will be reduced.
    /// * `size` - The amount of storage you want to remove.
    /// E.g if you have an existing [`StorageAccount`] with 3MB of storage
    /// but you want 2MB total, `size` should equal 1MB. 
    /// When specifying size, only KB, MB, and GB storage units are currently supported.
    /// 
    /// # Example
    ///
    /// ```
    /// # use byte_unit::Byte;
    /// # use shadow_drive_rust::{ShadowDriveClient, derived_addresses::storage_account};
    /// # use solana_client::rpc_client::RpcClient;
    /// # use solana_sdk::{
    /// # pubkey::Pubkey,
    /// # signature::Keypair,
    /// # signer::{keypair::read_keypair_file, Signer},
    /// # };
    /// #
    /// # let keypair = read_keypair_file(KEYPAIR_PATH).expect("failed to load keypair at path");
    /// # let user_pubkey = keypair.pubkey();
    /// # let rpc_client = RpcClient::new("https://ssc-dao.genesysgo.net");
    /// # let shdw_drive_client = ShadowDriveClient::new(keypair, rpc_client);
    /// # let (storage_account_key, _) = storage_account(&user_pubkey, 0);
    /// # let reduced_bytes = Byte::from_str("1MB").expect("invalid byte string");
    /// #
    /// let reduce_storage_response = shdw_drive_client
    ///     .reduce_storage(&storage_account_key, reduced_bytes)
    ///     .await?;
    /// ```
    pub async fn reduce_storage(
        &self,
        storage_account_key: &Pubkey,
        size: Byte,
    ) -> ShadowDriveResult<ShdwDriveResponse> {
        let size_as_bytes: u64 = size
            .get_bytes()
            .try_into()
            .map_err(|_| Error::InvalidStorage)?;

        let wallet_pubkey = self.wallet.pubkey();

        let selected_storage_acct = self.get_storage_account(storage_account_key).await?;
        let (unstake_account, _) = derived_addresses::unstake_account(&storage_account_key);
        let (unstake_info, _) = derived_addresses::unstake_info(&storage_account_key);

        let owner_ata = get_associated_token_address(&wallet_pubkey, &TOKEN_MINT);
        let (stake_account, _) = derived_addresses::stake_account(&storage_account_key);

        let emeissions_ata = get_associated_token_address(&EMISSIONS, &TOKEN_MINT);

        let accounts = shdw_drive_accounts::DecreaseStorage {
            storage_config: *STORAGE_CONFIG_PDA,
            storage_account: *storage_account_key,
            unstake_info,
            unstake_account,
            owner: selected_storage_acct.owner_1,
            owner_ata,
            stake_account,
            emissions_wallet: emeissions_ata,
            token_mint: TOKEN_MINT,
            system_program: system_program::ID,
            token_program: TokenProgramID,
            rent: rent::ID,
        };
        let args = shdw_drive_instructions::DecreaseStorage {
            remove_storage: Some(size_as_bytes),
        };

        let instruction = Instruction {
            program_id: PROGRAM_ADDRESS,
            accounts: accounts.to_account_metas(None),
            data: args.data(),
        };

        let txn = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&wallet_pubkey),
            &[&self.wallet],
            self.rpc_client.get_latest_blockhash()?,
        );

        let txn_result = self
            .rpc_client
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &txn,
                CommitmentConfig {
                    commitment: CommitmentLevel::Confirmed,
                },
            )?;

        Ok(ShdwDriveResponse {
            txid: txn_result.to_string(),
        })
    }
}
