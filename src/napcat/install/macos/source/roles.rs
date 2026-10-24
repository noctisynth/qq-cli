//! Reviewed website QQ 7.0.2-53644 bundle roles. Never infer roles at runtime.
use super::*;
use crate::napcat::install::macos::signing::Policy;
use sha2::{Digest, Sha256};

pub(super) fn digest() -> String {
    let mut hash = Sha256::new();
    hash.update(b"qq-cli-macos-source-roles-v1\0");
    for (path, id, executable, policy) in ROLES {
        let role = match policy {
            Policy::Main => "main",
            Policy::Electron => "electron",
            Policy::PluginHost => "plugin-host",
            Policy::Native => "native",
        };
        for value in [*path, *id, *executable, role, &policy.entitlements()] {
            hash.update((value.len() as u64).to_le_bytes());
            hash.update(value.as_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

pub(super) fn roles() -> Result<Vec<BundleRole>> {
    ROLES
        .iter()
        .map(|(path, id, executable, policy)| {
            BundleRole::new(Path::new(path), id, executable, *policy)
        })
        .collect()
}

const ROLES: &[(&str, &str, &str, Policy)] = &[
    ("", "com.tencent.qq", "QQ", Policy::Main),
    (
        "Contents/PlugIns/share.appex",
        "com.tencent.qq.share",
        "share",
        Policy::Native,
    ),
    (
        "Contents/Frameworks/QQ Helper (GPU).app",
        "com.tencent.qq.helper.GPU",
        "QQ Helper (GPU)",
        Policy::Electron,
    ),
    (
        "Contents/Frameworks/QQ Helper (Plugin).app",
        "com.tencent.qq.helper.Plugin",
        "QQ Helper (Plugin)",
        Policy::Electron,
    ),
    (
        "Contents/Frameworks/QQ Helper (Renderer).app",
        "com.tencent.qq.helper.Renderer",
        "QQ Helper (Renderer)",
        Policy::Electron,
    ),
    (
        "Contents/Frameworks/QQ Helper.app",
        "com.tencent.qq.helper",
        "QQ Helper",
        Policy::Electron,
    ),
    (
        "Contents/Frameworks/QQNT.framework",
        "com.github.Electron.framework",
        "QQNT",
        Policy::Native,
    ),
    (
        "Contents/Library/LoginItems/QQ Login Helper.app",
        "com.tencent.qq.loginhelper",
        "QQ Login Helper",
        Policy::Native,
    ),
    (
        "Contents/MacOS/QQEXDOC.app",
        "com.tencent.qqexdoc",
        "QQEXDOC",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDOC.app/Contents/Frameworks/QQEXDOC Helper (GPU).app",
        "com.tencent.qqexdoc.helper.GPU",
        "QQEXDOC Helper (GPU)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDOC.app/Contents/Frameworks/QQEXDOC Helper (Plugin).app",
        "com.tencent.qqexdoc.helper.Plugin",
        "QQEXDOC Helper (Plugin)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDOC.app/Contents/Frameworks/QQEXDOC Helper (Renderer).app",
        "com.tencent.qqexdoc.helper.Renderer",
        "QQEXDOC Helper (Renderer)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDOC.app/Contents/Frameworks/QQEXDOC Helper.app",
        "com.tencent.qqexdoc.helper",
        "QQEXDOC Helper",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDesktopPet.app",
        "com.tencent.qqexdesktoppet",
        "QQEXDesktopPet",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDesktopPet.app/Contents/Frameworks/QQEXDesktopPet Helper (GPU).app",
        "com.tencent.qqexdoc.helper.GPU",
        "QQEXDesktopPet Helper (GPU)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDesktopPet.app/Contents/Frameworks/QQEXDesktopPet Helper (Plugin).app",
        "com.tencent.qqexdoc.helper.Plugin",
        "QQEXDesktopPet Helper (Plugin)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDesktopPet.app/Contents/Frameworks/QQEXDesktopPet Helper (Renderer).app",
        "com.tencent.qqexdoc.helper.Renderer",
        "QQEXDesktopPet Helper (Renderer)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXDesktopPet.app/Contents/Frameworks/QQEXDesktopPet Helper.app",
        "com.tencent.qqexdoc.helper",
        "QQEXDesktopPet Helper",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXGuild.app",
        "com.tencent.qqexguild",
        "QQEXGuild",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXGuild.app/Contents/Frameworks/QQEXGuild Helper (GPU).app",
        "com.tencent.qqexdoc.helper.GPU",
        "QQEXGuild Helper (GPU)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXGuild.app/Contents/Frameworks/QQEXGuild Helper (Plugin).app",
        "com.tencent.qqexdoc.helper.Plugin",
        "QQEXGuild Helper (Plugin)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXGuild.app/Contents/Frameworks/QQEXGuild Helper (Renderer).app",
        "com.tencent.qqexdoc.helper.Renderer",
        "QQEXGuild Helper (Renderer)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXGuild.app/Contents/Frameworks/QQEXGuild Helper.app",
        "com.tencent.qqexdoc.helper",
        "QQEXGuild Helper",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXMiniProgram.app",
        "com.tencent.qqexminiprogram",
        "QQEXMiniProgram",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXMiniProgram.app/Contents/Frameworks/QQEXMiniProgram Helper (GPU).app",
        "com.tencent.qqexdoc.helper.GPU",
        "QQEXMiniProgram Helper (GPU)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXMiniProgram.app/Contents/Frameworks/QQEXMiniProgram Helper (Plugin).app",
        "com.tencent.qqexdoc.helper.Plugin",
        "QQEXMiniProgram Helper (Plugin)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXMiniProgram.app/Contents/Frameworks/QQEXMiniProgram Helper (Renderer).app",
        "com.tencent.qqexdoc.helper.Renderer",
        "QQEXMiniProgram Helper (Renderer)",
        Policy::Electron,
    ),
    (
        "Contents/MacOS/QQEXMiniProgram.app/Contents/Frameworks/QQEXMiniProgram Helper.app",
        "com.tencent.qqexdoc.helper",
        "QQEXMiniProgram Helper",
        Policy::Electron,
    ),
    (
        "Contents/Resources/app/ClipBoard.framework",
        "com.tencent.qq.ClipBoard",
        "ClipBoard",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/CrdRemoteDesktopSDK.framework",
        "org.chromium.CrdRemoteDesktopSDK",
        "CrdRemoteDesktopSDK",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/JTRecordSDK.framework",
        "com.Tencent.JTRecordSDK",
        "JTRecordSDK",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/LightQuic.framework",
        "com.tencent.LightQuic",
        "LightQuic",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/LongCaptureFramework.framework",
        "com.Tencent.LongCaptureFramework",
        "LongCaptureFramework",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/QQ ScreenCapture plugin.app",
        "FN2V63AD2J.com.tencent.ScreenCapture3",
        "QQ ScreenCapture plugin",
        Policy::PluginHost,
    ),
    (
        "Contents/Resources/app/QQ ScreenCapture plugin.app/Contents/Frameworks/AFNetworking.framework",
        "com.alamofire.AFNetworking",
        "AFNetworking",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/QQ ScreenCapture plugin.app/Contents/Frameworks/CocoaLumberjack.framework",
        "com.deusty.CocoaLumberjack",
        "CocoaLumberjack",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/QQ ScreenCapture plugin.app/Contents/Frameworks/JTRecordSDK.framework",
        "com.Tencent.JTRecordSDK",
        "JTRecordSDK",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/QQ ScreenCapture plugin.app/Contents/Frameworks/JietuFramework.framework",
        "com.Tencent.JietuFramework",
        "JietuFramework",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/QQ ScreenCapture plugin.app/Contents/Frameworks/MASShortcut.framework",
        "com.github.shpakovski.MASShortcut",
        "MASShortcut",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/QQHelper.app",
        "com.tencent.qq.qqhelper",
        "QQHelper",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/ScreenCaptureAgent.framework",
        "com.tencent.qq.ScreenCaptureAgent",
        "ScreenCaptureAgent",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/avsdk/TXFFmpeg.framework",
        "com.tencent.liteav.FFMPEG",
        "TXFFmpeg",
        Policy::Native,
    ),
    (
        "Contents/Resources/app/avsdk/TXSoundTouch.framework",
        "com.tencent.liteav.SoundTouch",
        "TXSoundTouch",
        Policy::Native,
    ),
];
