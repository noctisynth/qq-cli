//! Read-only Security Framework signing information; no private key access.
use crate::core::error::{AppError, Result};
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::CFURL;
use security_framework::os::macos::code_signing::SecStaticCode;

unsafe extern "C" {
    fn SecCodeCopySigningInformation(
        code: *const std::ffi::c_void,
        flags: u32,
        information: *mut CFDictionaryRef,
    ) -> i32;
    static kSecCodeInfoFlags: CFStringRef;
    static kSecCodeInfoEntitlementsDict: CFStringRef;
    static kSecCodeInfoEntitlements: CFStringRef;
    static kSecCodeInfoMainExecutable: CFStringRef;
    static kSecCodeInfoPList: CFStringRef;
}

fn information(code: &SecStaticCode, flags: u32) -> Result<CFDictionary<CFString, CFType>> {
    let mut output = std::ptr::null();
    // SAFETY: live SecStaticCode, fixed signing-information selector, initialized
    // out-pointer. Successful API output follows the CF Create ownership rule.
    let status = unsafe { SecCodeCopySigningInformation(code.as_CFTypeRef(), flags, &mut output) };
    if status != 0 || output.is_null() {
        return Err(invalid());
    }
    // SAFETY: successful API guarantees a retained CFDictionary. SDK keys are
    // CFString and values CF objects; wrappers release our one retained result.
    Ok(unsafe { CFDictionary::wrap_under_create_rule(output) })
}

fn dictionary(value: &CFType) -> Result<CFDictionary<CFString, CFType>> {
    let raw = value.downcast::<CFDictionary>().ok_or_else(invalid)?;
    // SAFETY: runtime dictionary type checked; SDK values are CF objects.
    // Only our own CFString keys are used for lookup, never unchecked keys.
    Ok(unsafe { CFDictionary::wrap_under_get_rule(raw.as_concrete_TypeRef()) })
}

/// Metadata as understood by codesign, even before an edited bundle is signed.
/// Does not validate provenance, entitlements, or authorize modifying the path.
pub(crate) fn bundle(
    code: &SecStaticCode,
    metadata: &[u8],
) -> Result<(std::path::PathBuf, String, String)> {
    if metadata.len() > 1024 * 1024 {
        return Err(invalid());
    }
    let info = information(code, 0)?;
    // SAFETY: immutable process-lifetime SDK constants, retained by wrappers.
    let executable_key = unsafe { CFString::wrap_under_get_rule(kSecCodeInfoMainExecutable) };
    let plist_key = unsafe { CFString::wrap_under_get_rule(kSecCodeInfoPList) };
    let executable = info
        .find(&executable_key)
        .and_then(|v| v.downcast::<CFURL>())
        .and_then(|v| v.to_path())
        .ok_or_else(invalid)?;
    let (raw, _) = core_foundation::propertylist::create_with_data(
        CFData::from_buffer(metadata),
        core_foundation::propertylist::kCFPropertyListImmutable,
    )
    .map_err(|_| invalid())?;
    // SAFETY: successful CF property-list parser returns a retained CF object;
    // release follows the Create rule, and dictionary type is checked below.
    let parsed = unsafe { CFType::wrap_under_create_rule(raw) };
    let identity = plist_identity(&parsed)?;
    if let Some(secured) = info.find(&plist_key)
        && plist_identity(&secured)? != identity
    {
        return Err(invalid());
    }
    Ok((executable, identity.0, identity.1))
}

fn plist_identity(value: &CFType) -> Result<(String, String)> {
    let plist = dictionary(value)?;
    let value = |key| {
        plist
            .find(CFString::new(key))
            .and_then(|v| v.downcast::<CFString>())
            .map(|v| v.to_string())
            .ok_or_else(invalid)
    };
    Ok((value("CFBundleIdentifier")?, value("CFBundleExecutable")?))
}

/// Exact native-architecture policy. Validity and tree identity are separate.
pub(crate) fn verify(code: &SecStaticCode, keys: &[&str]) -> Result<()> {
    let info = information(code, 1 << 1)?;
    // SAFETY: immutable process-lifetime SDK constants, borrowed with retain.
    let flags_key = unsafe { CFString::wrap_under_get_rule(kSecCodeInfoFlags) };
    let entitlements_key = unsafe { CFString::wrap_under_get_rule(kSecCodeInfoEntitlementsDict) };
    // SAFETY: same SDK constant ownership as above.
    let raw_entitlements_key = unsafe { CFString::wrap_under_get_rule(kSecCodeInfoEntitlements) };
    let flags = info
        .find(&flags_key)
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|v| v.to_i64())
        .ok_or_else(invalid)?;
    // CS_ADHOC | CS_RUNTIME only. Do not accept linker-only signatures or
    // silently preserve source signing options beyond the reviewed policy.
    if flags != 0x10002 {
        return Err(invalid());
    }
    let Some(value) = info.find(&entitlements_key) else {
        // An absent dictionary can also mean a non-dictionary entitlement blob.
        // Never interpret that case as an entitlement-free native executable.
        return if keys.is_empty() && info.find(&raw_entitlements_key).is_none() {
            Ok(())
        } else {
            Err(invalid())
        };
    };
    let dict = dictionary(&value)?;
    if dict.len() != keys.len() {
        return Err(invalid());
    }
    for key in keys {
        let enabled = dict
            .find(CFString::new(key))
            .and_then(|v| v.downcast::<CFBoolean>())
            .is_some_and(bool::from);
        if !enabled {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(crate) fn invalid() -> AppError {
    AppError::new(
        "MACOS_SIGNING_POLICY",
        "Private application signature or entitlements do not match the required policy.",
        4,
    )
}
