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

use super::{super::super::State, Matcher, Result};
use log::info;
use maxminddb::{geoip2::Country, Reader};
use std::{collections::HashSet, net::IpAddr};
use trust_dns_proto::rr::record_data::RData::{A, AAAA};

/// A matcher that matches if IP address in the record of the first A/AAAA response is in the list of countries.
pub struct GeoIp {
    db: Reader<Vec<u8>>,
    list: HashSet<String>,
}

impl GeoIp {
    /// Create a new `Geoip` matcher from a set of ISO country codes like `CN`, `AU`.
    pub fn new(list: HashSet<String>, buf: Vec<u8>) -> Result<Self> {
        Ok(Self {
            list,
            db: Reader::from_source(buf)?,
        })
    }
}

impl Matcher for GeoIp {
    fn matches(&self, state: &State) -> bool {
        if let Some(ip) = state
            .resp
            .answers()
            .iter()
            .find(|&r| matches!(r.rdata(), A(_) | AAAA(_)))
            .map(|r| match *r.rdata() {
                A(addr) => IpAddr::V4(addr),
                AAAA(addr) => IpAddr::V6(addr),
                _ => unreachable!(),
            })
        {
            let r = if let Ok(r) = self.db.lookup::<Country>(ip) {
                r
            } else {
                return false;
            };

            r.country
                .and_then(|c| {
                    c.iso_code.map(|n| {
                        info!("The IP `{}` has ISO country code `{}`", ip, n);
                        self.list.contains(n)
                    })
                })
                .unwrap_or(false)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::Matcher, GeoIp, State};
    use once_cell::sync::Lazy;
    use std::str::FromStr;
    use trust_dns_proto::{
        op::Message,
        rr::{resource::Record, Name, RData},
    };

    // Starting from droute's crate root
    static PATH: Lazy<Vec<u8>> =
        Lazy::new(|| include_bytes!("../../../../../../data/full.mmdb").to_vec());
    static RECORD_NOT_CHINA: Lazy<Record> = Lazy::new(|| {
        Record::from_rdata(
            Name::from_str("apple.com").unwrap(),
            10,
            RData::A("1.1.1.1".parse().unwrap()),
        )
    });
    static RECORD_CHINA: Lazy<Record> = Lazy::new(|| {
        Record::from_rdata(
            Name::from_str("baidu.com").unwrap(),
            10,
            RData::A("36.152.44.95".parse().unwrap()),
        )
    });

    fn create_state(v: Vec<Record>) -> State {
        State {
            resp: {
                let mut m = Message::new();
                m.insert_answers(v);
                m
            },
            ..Default::default()
        }
    }

    #[test]
    fn builtin_db_not_china() {
        assert_eq!(
            GeoIp::new(
                vec!["CN".to_string()].into_iter().collect(),
                include_bytes!("../../../../../../data/full.mmdb").to_vec()
            )
            .unwrap()
            .matches(&create_state(vec![(RECORD_NOT_CHINA).clone()])),
            false
        )
    }

    #[test]
    fn not_china() {
        assert_eq!(
            GeoIp::new(vec!["CN".to_string()].into_iter().collect(), (PATH).clone(),)
                .unwrap()
                .matches(&create_state(vec![(RECORD_NOT_CHINA).clone()])),
            false
        )
    }

    #[test]
    fn mixed() {
        let geoip = GeoIp::new(
            vec!["CN".to_string(), "AU".to_string()]
                .into_iter()
                .collect(),
            (PATH).clone(),
        )
        .unwrap();
        assert_eq!(
            geoip.matches(&create_state(vec![(RECORD_CHINA).clone()])),
            true
        );
        assert_eq!(
            geoip.matches(&create_state(vec![(RECORD_NOT_CHINA).clone()])),
            true
        )
    }

    #[test]
    fn empty_records() {
        assert_eq!(
            GeoIp::new(vec!["CN".to_string()].into_iter().collect(), (PATH).clone(),)
                .unwrap()
                .matches(&State::default()),
            false,
        )
    }

    #[test]
    fn is_china() {
        assert_eq!(
            GeoIp::new(vec!["CN".to_string()].into_iter().collect(), (PATH).clone(),)
                .unwrap()
                .matches(&create_state(vec![(RECORD_CHINA).clone()])),
            true
        )
    }

    #[test]
    fn unordered_is_china() {
        assert_eq!(
            GeoIp::new(vec!["CN".to_string()].into_iter().collect(), (PATH).clone(),)
                .unwrap()
                .matches(&create_state(vec![
                    (RECORD_CHINA).clone(),
                    (RECORD_NOT_CHINA).clone(),
                    Record::from_rdata(
                        Name::from_str("baidu.com").unwrap(),
                        10,
                        RData::NS(Name::from_str("baidu.com").unwrap()),
                    ),
                ])),
            true
        )
    }
}
