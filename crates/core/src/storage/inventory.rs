use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use super::{INVENTORY_SCHEMA_VERSION, SchemaError};
use crate::domain::{AbsolutePath, DesiredResource, ResourceGraph, ResourceId, Scope};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "InventoryWire")]
pub struct InventoryDocument {
    projects: BTreeSet<AbsolutePath>,
    resources: BTreeMap<ResourceId, DesiredResource>,
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
                    resource: resource.id().clone(),
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

    pub const fn resources(&self) -> &BTreeMap<ResourceId, DesiredResource> {
        &self.resources
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
