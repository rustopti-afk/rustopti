use winreg::RegKey;
use winreg::HKEY;

/// Sets a DWORD value in the Windows Registry.
pub fn set_dword(hkey: HKEY, subkey: &str, name: &str, value: u32) -> Result<(), String> {
    let key = RegKey::predef(hkey);
    let (reg_key, _) = key
        .create_subkey(subkey)
        .map_err(|e| format!("Failed to open/create key '{}': {}", subkey, e))?;
    reg_key
        .set_value(name, &value)
        .map_err(|e| format!("Failed to set '{}' = {}: {}", name, value, e))?;
    Ok(())
}

/// Gets a DWORD value from the Windows Registry.
pub fn get_dword(hkey: HKEY, subkey: &str, name: &str) -> Result<u32, String> {
    let key = RegKey::predef(hkey);
    let reg_key = key
        .open_subkey(subkey)
        .map_err(|e| format!("Failed to open key '{}': {}", subkey, e))?;
    reg_key
        .get_value::<u32, _>(name)
        .map_err(|e| format!("Failed to read '{}': {}", name, e))
}

/// Sets a String value in the Windows Registry.
pub fn set_string(hkey: HKEY, subkey: &str, name: &str, value: &str) -> Result<(), String> {
    let key = RegKey::predef(hkey);
    let (reg_key, _) = key
        .create_subkey(subkey)
        .map_err(|e| format!("Failed to open/create key '{}': {}", subkey, e))?;
    reg_key
        .set_value(name, &value)
        .map_err(|e| format!("Failed to set '{}' = '{}': {}", name, value, e))?;
    Ok(())
}

/// Gets a String value from the Windows Registry.
pub fn get_string(hkey: HKEY, subkey: &str, name: &str) -> Result<String, String> {
    let key = RegKey::predef(hkey);
    let reg_key = key
        .open_subkey(subkey)
        .map_err(|e| format!("Failed to open key '{}': {}", subkey, e))?;
    reg_key
        .get_value::<String, _>(name)
        .map_err(|e| format!("Failed to read '{}': {}", name, e))
}

/// Exports a registry key to a backup string representation.
pub fn export_key_values(hkey: HKEY, subkey: &str) -> Result<Vec<(String, String)>, String> {
    let key = RegKey::predef(hkey);
    let reg_key = key
        .open_subkey(subkey)
        .map_err(|e| format!("Failed to open key '{}': {}", subkey, e))?;

    let mut values = Vec::new();
    for value_result in reg_key.enum_values() {
        match value_result {
            Ok((name, value)) => {
                values.push((name, format!("{:?}", value)));
            }
            Err(e) => {
                values.push(("ERROR".to_string(), e.to_string()));
            }
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dword_nonexistent() {
        let result = get_dword(
            HKEY_CURRENT_USER,
            r"Software\RustOpti\TestNonExistent",
            "TestValue",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_set_and_get_dword() {
        let subkey = r"Software\RustOpti\UnitTest";
        let name = "TestDword";
        let value: u32 = 42;

        // Set
        let set_result = set_dword(HKEY_CURRENT_USER, subkey, name, value);
        assert!(set_result.is_ok(), "Failed to set: {:?}", set_result);

        // Get
        let get_result = get_dword(HKEY_CURRENT_USER, subkey, name);
        assert!(get_result.is_ok(), "Failed to get: {:?}", get_result);
        assert_eq!(get_result.unwrap(), value);

        // Cleanup
        let key = RegKey::predef(HKEY_CURRENT_USER);
        let _ = key.delete_subkey_all(r"Software\RustOpti\UnitTest");
    }

    #[test]
    fn test_set_and_get_string() {
        let subkey = r"Software\RustOpti\UnitTest";
        let name = "TestString";
        let value = "hello_rustopti";

        let set_result = set_string(HKEY_CURRENT_USER, subkey, name, value);
        assert!(set_result.is_ok(), "Failed to set string: {:?}", set_result);

        // Cleanup
        let key = RegKey::predef(HKEY_CURRENT_USER);
        let _ = key.delete_subkey_all(r"Software\RustOpti\UnitTest");
    }
}
