use crate::{
    external_client_config::ExternalClientConfig,
    group::{framing::MLSMessage, ExternalGroup, GroupError},
};
use thiserror::Error;

pub use crate::external_client_builder::{
    BaseConfig, ExternalClientBuilder, Missing, MlsConfig, WithIdentityValidator, WithKeychain,
    WithProposalFilter,
};

#[derive(Debug, Error)]
pub enum ExternalClientError {
    #[error(transparent)]
    GroupError(#[from] GroupError),
}

pub struct ExternalClient<C> {
    config: C,
}

impl ExternalClient<()> {
    pub fn builder() -> ExternalClientBuilder<BaseConfig> {
        ExternalClientBuilder::new()
    }
}

impl<C> ExternalClient<C>
where
    C: ExternalClientConfig + Clone,
{
    pub(crate) fn new(config: C) -> Self {
        Self { config }
    }

    pub fn observe_group(
        &self,
        group_info: MLSMessage,
        tree_data: Option<&[u8]>,
    ) -> Result<ExternalGroup<C>, ExternalClientError> {
        ExternalGroup::join(self.config.clone(), group_info, tree_data).map_err(Into::into)
    }
}
