use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use super::{INVENTORY_SCHEMA_VERSION, SchemaError};
use crate::domain::{AbsolutePath, DesiredResource, ResourceGraph, ResourceKey, Scope};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "InventoryWire")]
pub struct InventoryDocument {
    projects: BTreeSet<AbsolutePath>,
    resources: BTreeMap<ResourceKey, DesiredResource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InventoryWire {
    schema: u32,
    projects: Vec<AbsolutePath>,
    resources: Vec<DesiredResource>,
}

impl InventoryDocument {
    pub const fn schema(&self) -> u32 {
        INVENTORY_SCHEMA_VERSION
    }

    pub fn new(
        schema: u32,
        projects: impl IntoIterator<Item = AbsolutePath>,
        resources: impl IntoIterator<Item = DesiredResource>,
    ) -> Result<Self, SchemaError> {
        if schema != INVENTORY_SCHEMA_VERSION {
            return Err(SchemaError::UnsupportedVersion {
                document: "inventory",
                version: schema,
            });
        }
        let mut project_set = BTreeSet::new();
        for path in projects {
            if !project_set.insert(path.clone()) {
                return Err(SchemaError::DuplicateProject { path });
            }
        }
        let graph = ResourceGraph::new(resources, [], [])?;
        for resource in graph.desired().values() {
            if let Scope::Project(path) = resource.scope()
                && !project_set.contains(path)
            {
                return Err(SchemaError::UndeclaredProject {
                    resource: resource.key().clone(),
                    path: path.clone(),
                });
            }
        }
        Ok(Self {
            projects: project_set,
            resources: graph.desired().clone(),
        })
    }

    pub const fn projects(&self) -> &BTreeSet<AbsolutePath> {
        &self.projects
    }

    pub const fn resources(&self) -> &BTreeMap<ResourceKey, DesiredResource> {
        &self.resources
    }

    /// Return a copy with one desired resource added idempotently.
    pub fn with_resource(&self, resource: DesiredResource) -> Result<Self, SchemaError> {
        if let Some(existing) = self.resources.get(resource.key()) {
            if existing == &resource {
                return Ok(self.clone());
            }
            return Err(SchemaError::InventoryResourceConflict {
                resource: resource.key().clone(),
            });
        }
        let mut resources = self.resources.values().cloned().collect::<Vec<_>>();
        resources.push(resource);
        let mut projects = self.projects.clone();
        if let crate::domain::Scope::Project(path) =
            resources.last().expect("resource was appended").scope()
        {
            projects.insert(path.clone());
        }
        Self::new(INVENTORY_SCHEMA_VERSION, projects, resources)
    }

    /// Return a copy without one desired resource. Removing an absent key is
    /// idempotent; recorded project scopes remain available for `--all-scopes`
    /// so a later adoption can still address them explicitly.
    pub fn without_resource(&self, key: &ResourceKey) -> Option<Self> {
        if !self.resources.contains_key(key) {
            return Some(self.clone());
        }
        let resources = self
            .resources
            .iter()
            .filter(|(resource_key, _)| *resource_key != key)
            .map(|(_, resource)| resource.clone())
            .collect::<Vec<_>>();
        Self::new(
            INVENTORY_SCHEMA_VERSION,
            self.projects.iter().cloned(),
            resources,
        )
        .ok()
    }

    /// Replace one desired resource while retaining all other inventory
    /// entries. This is useful when a target-scoped operation narrows the
    /// target projection of an existing resource.
    pub fn replace_resource(&self, resource: DesiredResource) -> Result<Self, SchemaError> {
        if self.resources.get(resource.key()) == Some(&resource) {
            return Ok(self.clone());
        }
        let key = resource.key().clone();
        let resources = self
            .resources
            .iter()
            .filter(|(resource_key, _)| *resource_key != &key)
            .map(|(_, value)| value.clone())
            .chain(std::iter::once(resource))
            .collect::<Vec<_>>();
        Self::new(
            INVENTORY_SCHEMA_VERSION,
            self.projects.iter().cloned(),
            resources,
        )
    }
}

impl From<InventoryDocument> for InventoryWire {
    fn from(value: InventoryDocument) -> Self {
        Self {
            schema: INVENTORY_SCHEMA_VERSION,
            projects: value.projects.into_iter().collect(),
            resources: value.resources.into_values().collect(),
        }
    }
}

impl TryFrom<InventoryWire> for InventoryDocument {
    type Error = SchemaError;

    fn try_from(value: InventoryWire) -> Result<Self, Self::Error> {
        Self::new(value.schema, value.projects, value.resources)
    }
}

impl<'de> Deserialize<'de> for InventoryDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        InventoryWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
