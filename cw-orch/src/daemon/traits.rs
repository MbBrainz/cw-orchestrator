use crate::daemon::queriers::CosmWasm;
use crate::environment::TxResponse;
use crate::error::CwOrchError;
use crate::prelude::*;

use super::sync::core::Daemon;

/// Helper methods for conditional uploading of a contract.
pub trait ConditionalUpload: CwOrcUpload<Daemon> {
    /// Only upload the contract if it is not uploaded yet (checksum does not match)
    fn upload_if_needed(&self) -> Result<Option<TxResponse<Daemon>>, CwOrchError> {
        if self.latest_is_uploaded()? {
            Ok(None)
        } else {
            Some(self.upload()).transpose()
        }
    }

    /// Returns whether the checksum of the WASM file matches the checksum of the latest uploaded code for this contract.
    fn latest_is_uploaded(&self) -> Result<bool, CwOrchError> {
        let Some(latest_uploaded_code_id) = self.code_id().ok() else {
            return Ok(false);
        };

        let chain = self.get_chain();
        let on_chain_hash = chain.rt_handle.block_on(
            chain
                .query_client::<CosmWasm>()
                .code_id_hash(latest_uploaded_code_id),
        )?;
        let local_hash = self.wasm().checksum()?;

        Ok(local_hash == on_chain_hash)
    }

    /// Returns whether the contract is running the latest uploaded code for it
    fn is_running_latest(&self) -> Result<bool, CwOrchError> {
        let Some(latest_uploaded_code_id) = self.code_id().ok() else {
            return Ok(false);
        };
        let chain = self.get_chain();
        let info = chain.rt_handle.block_on(
            chain
                .query_client::<CosmWasm>()
                .contract_info(self.address()?),
        )?;
        Ok(latest_uploaded_code_id == info.code_id)
    }
}

impl<T> ConditionalUpload for T where T: CwOrcUpload<Daemon> {}

/// Helper methods for conditional migration of a contract.
pub trait ConditionalMigrate: CwOrcMigrate<Daemon> + ConditionalUpload {
    /// Only migrate the contract if it is not on the latest code-id yet
    fn migrate_if_needed(
        &self,
        migrate_msg: &Self::MigrateMsg,
    ) -> Result<Option<TxResponse<Daemon>>, CwOrchError> {
        if self.is_running_latest()? {
            log::info!("{} is already running the latest code", self.id());
            Ok(None)
        } else {
            Some(self.migrate(migrate_msg, self.code_id()?)).transpose()
        }
    }
}

impl<T> ConditionalMigrate for T where T: CwOrcMigrate<Daemon> + CwOrcUpload<Daemon> {}
