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

// The logging level usage is governed by following convention: https://stackoverflow.com/questions/2031163/when-to-use-the-different-log-levels
//    Trace - Only when I would be "tracing" the code and trying to find one part of a function specifically.
//    Debug - Information that is diagnostically helpful to people more than just developers (IT, sysadmins, etc.).
//    Info - Generally useful information to log (service start/stop, configuration assumptions, etc). Info I want to always have available but usually don't care about under normal circumstances. This is my out-of-the-box config level.
//    Warn - Anything that can potentially cause application oddities, but for which I am automatically recovering. (Such as switching from a primary to backup server, retrying an operation, missing secondary data, etc.)
//    Error - Any error which is fatal to the operation, but not the service or application (can't open a required file, missing data, etc.). These errors will force user (administrator, or direct user) intervention. These are usually reserved (in my apps) for incorrect connection strings, missing services, etc.
//    Fatal - Any error that is forcing a shutdown of the service or application to prevent data loss (or further data loss). I reserve these only for the most heinous errors and situations where there is guaranteed to have been data corruption or loss.

#![deny(missing_docs)]
#![deny(unsafe_code)]
// Documentation
//! This is the core library for dcompass. It implements configuration parsing scheme, DNS query routing rules, and upstream managements.
pub mod error;
#[doc(hidden)]
pub mod mock;
mod router;

use std::collections::HashSet;

/// All the builders
pub mod builders {
    pub use super::router::{
        table::{
            rule::{actions::builder::*, builder::*, matchers::builder::*},
            TableBuilder,
        },
        upstreams::{UpstreamBuilder, UpstreamsBuilder},
        RouterBuilder,
    };
}

// All the major components
pub use self::router::{
    table::{
        rule::{actions, matchers, Rule},
        Table,
    },
    upstreams::{Upstream, Upstreams},
    Router,
};

use std::sync::Arc;

// Maximum TTL as defined in https://tools.ietf.org/html/rfc2181, 2147483647
//   Setting this to a value of 1 day, in seconds
const MAX_TTL: u32 = 86400_u32;

/// The type used for tag names in upstreams and routing tables.
pub type Label = Arc<str>;

/// A object that can be validated
pub trait Validatable {
    /// The possible errors from the validation.
    type Error;
    /// Validate oneself.
    /// `used`: some of the tags used by other parts, which should be existed.
    fn validate(&self, used: Option<&HashSet<Label>>) -> std::result::Result<(), Self::Error>;
}

// A cell used for bucket for validations
struct ValidateCell {
    pub used: bool,
    pub value: i32,
}

impl Default for ValidateCell {
    fn default() -> Self {
        Self {
            used: false,
            value: 0,
        }
    }
}

impl ValidateCell {
    pub fn used(&self) -> bool {
        self.used
    }

    pub fn val(&self) -> &i32 {
        &self.value
    }

    pub fn add(&mut self, rhs: i32) {
        self.used = true;
        self.value += rhs;
    }

    pub fn sub(&mut self, rhs: i32) {
        self.used = true;
        self.value -= rhs;
    }
}
