// Copyright 2020 LEXUGE
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use super::{error::Result, upstream::UpstreamBuilder, Upstreams};
use crate::Label;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, num::NonZeroUsize};

fn default_cache_size() -> NonZeroUsize {
    NonZeroUsize::new(2048).unwrap()
}

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Clone)]
/// The Builder for upstreams
pub struct UpstreamsBuilder {
    upstreams: HashMap<Label, UpstreamBuilder>,
    #[serde(default = "default_cache_size")]
    cache_size: NonZeroUsize,
}

impl UpstreamsBuilder {
    /// Create an UpstreamsBuilder from a set of upstreams and the cache_size for all of them.
    pub fn new(
        upstreams: HashMap<impl Into<Label>, UpstreamBuilder>,
        cache_size: NonZeroUsize,
    ) -> Self {
        Self {
            upstreams: upstreams.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            cache_size,
        }
    }

    /// Build the Upstreams from an UpstreamsBuilder
    pub async fn build(self) -> Result<Upstreams> {
        let mut v = HashMap::new();
        for (tag, u) in self.upstreams {
            v.insert(tag, u.build().await?);
        }
        Upstreams::new(v, self.cache_size)
    }
}
