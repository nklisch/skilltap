//! Explicit marketplace/plugin identity values.

use std::fmt;

use crate::domain::{NativeId, ResourceId, ResourceKey, Scope, Source, ValidationError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginSelector {
    plugin: NativeId,
    marketplace: NativeId,
}

impl PluginSelector {
    pub fn parse(value: &str) -> Result<Self, MarketplaceIdentityError> {
        let Some((plugin, marketplace)) = value.split_once('@') else {
            return Err(MarketplaceIdentityError::InvalidSelector);
        };
        if plugin.is_empty() || marketplace.is_empty() || marketplace.contains('@') {
            return Err(MarketplaceIdentityError::InvalidSelector);
        }
        Ok(Self {
            plugin: NativeId::new(plugin).map_err(MarketplaceIdentityError::InvalidId)?,
            marketplace: NativeId::new(marketplace).map_err(MarketplaceIdentityError::InvalidId)?,
        })
    }

    pub const fn plugin(&self) -> &NativeId {
        &self.plugin
    }
    pub const fn marketplace(&self) -> &NativeId {
        &self.marketplace
    }
}

impl fmt::Display for PluginSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.plugin, self.marketplace)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketplaceIdentity {
    pub resource: ResourceKey,
    pub name: NativeId,
    pub source: Source,
}

impl MarketplaceIdentity {
    pub fn new(
        name: NativeId,
        source: Source,
        scope: Scope,
    ) -> Result<Self, MarketplaceIdentityError> {
        let resource = ResourceKey::new(
            ResourceId::new(format!("marketplace:{}", name.as_str()))
                .map_err(MarketplaceIdentityError::InvalidId)?,
            scope,
        );
        Ok(Self {
            resource,
            name,
            source,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketplaceIdentityError {
    InvalidSelector,
    InvalidId(ValidationError),
}

impl fmt::Display for MarketplaceIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelector => {
                formatter.write_str("expected an exact plugin@marketplace selector")
            }
            Self::InvalidId(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for MarketplaceIdentityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{SourceKind, SourceLocator};

    #[test]
    fn plugin_selectors_are_exact_and_scope_bearing() {
        let selector = PluginSelector::parse("formatter@team").unwrap();
        assert_eq!(selector.to_string(), "formatter@team");
        assert!(PluginSelector::parse("formatter").is_err());
        assert!(PluginSelector::parse("formatter@team@other").is_err());
        let source = Source::new(
            SourceKind::Git,
            SourceLocator::new("https://example.invalid/team.git").unwrap(),
            None,
        )
        .unwrap();
        let identity =
            MarketplaceIdentity::new(NativeId::new("team").unwrap(), source, Scope::Global)
                .unwrap();
        assert_eq!(identity.resource.id().as_str(), "marketplace:team");
    }
}
