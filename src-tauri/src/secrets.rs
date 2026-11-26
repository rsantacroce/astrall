//! Chaves de API: cofre do sistema (Keychain, Credential Manager, Secret Service).

use rusqlite::Connection;

const SERVICE: &str = "xyz.santacroce.astrall";

#[cfg(not(target_os = "android"))]
mod store {
    use super::SERVICE;

    pub fn get(account: &str) -> Result<Option<String>, String> {
        match keyring::Entry::new(SERVICE, account).and_then(|e| e.get_password()) {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(account: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, account)
            .and_then(|e| e.set_password(value))
            .map_err(|e| e.to_string())
    }

    pub fn delete(account: &str) -> Result<(), String> {
        match keyring::Entry::new(SERVICE, account).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn fallback_key(account: &str) -> String {
    format!("secret:{account}")
}

/// Lê a chave. Se o cofre do sistema não estiver disponível (Linux sem Secret Service, por
/// exemplo), usa a cópia guardada no banco local.
pub fn get(conn: &Connection, account: &str) -> Option<String> {
    #[cfg(not(target_os = "android"))]
    if let Ok(Some(v)) = store::get(account) {
        return Some(v);
    }
    crate::db::get_setting(conn, &fallback_key(account)).ok().flatten()
}

/// Grava a chave; devolve `false` quando precisou cair no banco local (sem cofre).
pub fn set(conn: &Connection, account: &str, value: &str) -> Result<bool, String> {
    #[cfg(not(target_os = "android"))]
    if store::set(account, value).is_ok() {
        let _ = crate::db::delete_setting(conn, &fallback_key(account));
        return Ok(true);
    }
    crate::db::set_setting(conn, &fallback_key(account), value).map_err(|e| e.to_string())?;
    Ok(false)
}

pub fn delete(conn: &Connection, account: &str) -> Result<(), String> {
    #[cfg(not(target_os = "android"))]
    store::delete(account)?;
    crate::db::delete_setting(conn, &fallback_key(account)).map_err(|e| e.to_string())
}
