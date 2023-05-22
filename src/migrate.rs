use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdError, StdResult, Storage};
use cw_storage_plus::Item;
use semver::Version;

#[cw_serde]
pub struct ContractVersion {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.7.0", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,
}

pub const CONTRACT: Item<ContractVersion> = Item::new("contract_info");

/// get_contract_version can be use in migrate to read the previous version of this contract
pub fn get_contract_version(store: &dyn Storage) -> StdResult<ContractVersion> {
    CONTRACT.load(store)
}

/// set_contract_version should be used in instantiate to store the original version, and after a successful
/// migrate to update it
pub fn set_contract_version<T: Into<String>, U: Into<String>>(
    store: &mut dyn Storage,
    name: T,
    version: U,
) -> StdResult<()> {
    let val = ContractVersion {
        contract: name.into(),
        version: version.into(),
    };
    CONTRACT.save(store, &val)
}

/// This function not only validates that the right contract and version can be migrated, but also
/// updates the contract version from the original (stored) version to the new version.
/// It returns the original version for the convenience of doing external checks.
pub fn ensure_from_older_version(
    storage: &mut dyn Storage,
    name: &str,
    new_version: &str,
) -> StdResult<Version> {
    let version: Version = new_version.parse().map_err(from_semver)?;
    let stored = get_contract_version(storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    if name != stored.contract {
        let msg = format!("Cannot migrate from {} to {}", stored.contract, name);
        return Err(StdError::generic_err(msg));
    }

    if storage_version > version {
        let msg = format!(
            "Cannot migrate from newer version ({}) to older ({})",
            stored.version, new_version
        );
        return Err(StdError::generic_err(msg));
    }
    if storage_version < version {
        // we don't need to save anything if migrating from the same version
        set_contract_version(storage, name, new_version)?;
    }

    Ok(storage_version)
}

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn accepts_identical_version() {
        let mut storage = MockStorage::new();
        set_contract_version(&mut storage, "demo", "0.1.2").unwrap();
        // ensure this matches
        ensure_from_older_version(&mut storage, "demo", "0.1.2").unwrap();
    }

    #[test]
    fn accepts_and_updates_on_newer_version() {
        let mut storage = MockStorage::new();
        set_contract_version(&mut storage, "demo", "0.4.0").unwrap();
        // ensure this matches
        let original_version = ensure_from_older_version(&mut storage, "demo", "0.4.2").unwrap();

        // check the original version is returned
        assert_eq!(original_version.to_string(), "0.4.0".to_string());

        // check the version is updated
        let stored = get_contract_version(&storage).unwrap();
        assert_eq!(stored.contract, "demo".to_string());
        assert_eq!(stored.version, "0.4.2".to_string());
    }

    #[test]
    fn errors_on_name_mismatch() {
        let mut storage = MockStorage::new();
        set_contract_version(&mut storage, "demo", "0.1.2").unwrap();
        // ensure this matches
        let err = ensure_from_older_version(&mut storage, "cw20-base", "0.1.2").unwrap_err();
        assert!(err.to_string().contains("cw20-base"), "{}", err);
        assert!(err.to_string().contains("demo"), "{}", err);
    }

    #[test]
    fn errors_on_older_version() {
        let mut storage = MockStorage::new();
        set_contract_version(&mut storage, "demo", "0.10.2").unwrap();
        // ensure this matches
        let err = ensure_from_older_version(&mut storage, "demo", "0.9.7").unwrap_err();
        assert!(err.to_string().contains("0.10.2"), "{}", err);
        assert!(err.to_string().contains("0.9.7"), "{}", err);
    }

    #[test]
    fn errors_on_broken_version() {
        let mut storage = MockStorage::new();
        let err = ensure_from_older_version(&mut storage, "demo", "0.a.7").unwrap_err();
        assert!(
            err.to_string().contains("unexpected character 'a'"),
            "{}",
            err
        );
    }
}
