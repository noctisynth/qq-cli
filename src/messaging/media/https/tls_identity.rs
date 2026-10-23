//! Ephemeral test CA: trusted only by the dedicated synthetic client, never the OS.
use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::OnceLock;

struct Identity {
    root: CertificateDer<'static>,
    leaf: CertificateDer<'static>,
    key: Vec<u8>,
}
fn identity() -> &'static Identity {
    static IDENTITY: OnceLock<Identity> = OnceLock::new();
    IDENTITY.get_or_init(|| {
        // Short validity satisfies native TLS policy; wall clock does not age a checked-in fixture.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let date = |offset| {
            use chrono::Datelike;
            let day = chrono::DateTime::from_timestamp(now + offset, 0).unwrap();
            rcgen::date_time_ymd(day.year(), day.month() as u8, day.day() as u8)
        };
        let mut ca = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca.not_before = date(-86400);
        ca.not_after = date(7 * 86400);
        ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
        let ca_key = KeyPair::generate().unwrap();
        let root = ca.self_signed(&ca_key).unwrap().der().clone();
        let issuer = Issuer::new(ca, ca_key);
        let mut leaf = CertificateParams::new(vec!["gchat.qpic.cn".into()]).unwrap();
        leaf.not_before = date(-86400);
        leaf.not_after = date(86400);
        leaf.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
        leaf.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
        let key = KeyPair::generate().unwrap();
        let leaf = leaf.signed_by(&key, &issuer).unwrap().der().clone();
        Identity {
            root,
            leaf,
            key: key.serialize_der(),
        }
    })
}
pub(super) fn root() -> &'static [u8] {
    identity().root.as_ref()
}
pub(super) fn server() -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    let identity = identity();
    (
        vec![identity.leaf.clone(), identity.root.clone()],
        PrivatePkcs8KeyDer::from(identity.key.clone()).into(),
    )
}
