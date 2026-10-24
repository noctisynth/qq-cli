//! Fixed public provenance and content digests. Never infer compatibility from matching loaders.
pub(super) struct Pin {
    pub version: &'static str,
    pub commit: &'static str,
    pub url: &'static str,
    pub bytes: u64,
    pub files: usize,
    pub sha256: &'static str,
    pub required: &'static [(&'static str, &'static str)],
}
pub(super) const V4_18_41: Pin = Pin {
    version: "v4.18.41",
    commit: "859eb530c5801577633c2c3ae871d23586edf281",
    url: "https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.41/NapCat.Framework.zip",
    bytes: 36_738_019,
    files: 680,
    sha256: "3c526f5789ff6cc56d78381eb10536c8273fe394d227187b27dea94b59965822",
    required: &V4_18_41_REQUIRED,
};
// Preserve the Windows members while explicitly pinning the Apple Silicon payloads.
// Artifact identity does not establish platform or API compatibility.
const V4_18_41_COMMON: [(&str, &str); 7] =
    required("cd9da7415cafcc93cde124e7b91c9242b0617c6b9a32710d4ab5d5c8bbf0b666");
const V4_18_41_REQUIRED: [(&str, &str); 11] = [
    V4_18_41_COMMON[0],
    V4_18_41_COMMON[1],
    V4_18_41_COMMON[2],
    V4_18_41_COMMON[3],
    V4_18_41_COMMON[4],
    V4_18_41_COMMON[5],
    V4_18_41_COMMON[6],
    (
        "package.json",
        "abce2924f10c57cbd55fed0b2e59f6ffde6fd9d27b1ea7ebac26e77fa973c4c7",
    ),
    (
        "native/napi2native/napi2native.darwin.arm64.node",
        "5a48995703afbb2889c73ee4624b1df41dc5fbc563e77d9224aa5f2aec048953",
    ),
    (
        "native/packet/MoeHoo.darwin.arm64.node",
        "72e9c8ca9ba48f5e3e8ffd5182d57e3f567dcd71331b04fd3aacdf1c4b36e7bf",
    ),
    (
        "native/ffmpeg/ffmpegAddon.darwin.arm64.node",
        "c578932c04b40e36b32601a86d3833dbd9892df13bd13dbe9355623da0144cbf",
    ),
];
pub(super) const V4_18_38: Pin = Pin {
    version: "v4.18.38",
    commit: "bbd0d2aca3d470c20b33b1d4f95123335633bf26",
    url: "https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.38/NapCat.Framework.zip",
    bytes: 36_728_573,
    files: 680,
    sha256: "45ef7f46aa78588c636044b6226b4b1847e31287ab6ec36b2d48adaa9785097b",
    required: &required("f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a"),
};
pub(super) const V4_18_37: Pin = Pin {
    version: "v4.18.37",
    commit: "ff9a829e29038eeec3e23b062ec90c838fc9efa3",
    url: "https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.37/NapCat.Framework.zip",
    bytes: 36_723_625,
    files: 680,
    sha256: "59d9538c1f1b66ff1269c362069fcd7d1b1d6793c35255ae80e699b91e37401c",
    required: &required("dcc02d22e4ec2390c0bd1a082e44ac4bd37a02b69fddbc0846cdebcf3023f9ac"),
};
const fn required(mjs: &'static str) -> [(&'static str, &'static str); 7] {
    [
        (
            "napimain.exe",
            "116a7da5a0887244a4946472062a0cb0b866e353924808b659ff7d109b5ffe69",
        ),
        (
            "napiloader.dll",
            "f7dba30fc165ac013e4b9114df851d4c7dbb7853d05175df5050c7886e9d5311",
        ),
        (
            "nativeLoader.cjs",
            "98e12440f27444c686de8087bd69381bb3f3e7e567870a90507f5976e0b3e38f",
        ),
        (
            "napcat.cjs",
            "0bacde790aa4081b3a2d3b18faae3dee39f43b44d9fe1f622602fd2604a74887",
        ),
        ("napcat.mjs", mjs),
        (
            "native/napi2native/napi2native.win32.x64.node",
            "46780f9cc420a840318179b26cbb551357d2ae986dd8df960ef1a49656181fcc",
        ),
        (
            "native/packet/MoeHoo.win32.x64.node",
            "fb62bff4d093b892bbe5c5dc4197a226881689e43f6d438a66895fa7cdbb90f1",
        ),
    ]
}
