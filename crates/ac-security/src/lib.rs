use std::collections::{BTreeMap, BTreeSet};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

include!("capability.rs");
include!("extensions.rs");
include!("hooks.rs");
include!("mcp.rs");
include!("findings.rs");
include!("scanner.rs");
include!("active.rs");
include!("ai.rs");
include!("hardening.rs");
include!("tests.rs");
