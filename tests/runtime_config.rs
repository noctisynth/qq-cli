use qq_cli::napcat::runtime::runtime_config::{Bundle, Ports};
use serde_json::Value;
use std::collections::BTreeMap;

fn ports() -> Ports {
    Ports {
        http: 19001,
        events: 19002,
        control: 19003,
    }
}
fn render(bundle: &Bundle) -> BTreeMap<&'static str, Value> {
    bundle
        .files()
        .map(|file| {
            let mut bytes = Vec::new();
            file.write_to(&mut bytes).unwrap();
            (file.name(), serde_json::from_slice(&bytes).unwrap())
        })
        .collect()
}
#[test]
fn startup_templates_separate_trust_domains_and_need_no_account_preset() {
    for version in ["v4.18.38", "v4.18.37"] {
        let bundle = Bundle::generate(version, ports()).unwrap();
        assert_eq!(bundle.version(), version);
        assert_eq!(bundle.ports(), ports());
        let files = render(&bundle);
        assert_eq!(
            files.keys().copied().collect::<Vec<_>>(),
            ["napcat.json", "onebot11.json", "webui.json"]
        );
        let web = &files["webui.json"];
        assert_eq!(web["host"], "127.0.0.1");
        assert_eq!(web["port"], 19003);
        assert_eq!(web["autoLoginAccount"], "");
        assert_eq!(web["enableXForwardedFor"], false);
        assert_eq!(web["accessControlMode"], "whitelist");
        assert_eq!(web["ipWhitelist"], serde_json::json!(["127.0.0.1"]));
        let onebot = &files["onebot11.json"];
        let net = &onebot["network"];
        let http = &net["httpServers"][0];
        let ws = &net["websocketServers"][0];
        assert_eq!(net["httpServers"].as_array().unwrap().len(), 1);
        assert_eq!(net["websocketServers"].as_array().unwrap().len(), 1);
        for server in [http, ws] {
            assert_eq!(server["host"], "127.0.0.1");
            assert_eq!(server["enable"], true);
            assert_eq!(server["messagePostFormat"], "array");
            assert_eq!(server["debug"], false);
            assert!(server["token"].as_str().unwrap() == bundle.api_token());
        }
        assert_eq!(http["port"], 19001);
        assert_eq!(http["enableCors"], false);
        assert_eq!(http["enableWebsocket"], false);
        assert_eq!(ws["port"], 19002);
        assert_eq!(ws["reportSelfMessage"], true);
        assert_eq!(ws["enableForcePushEvent"], true);
        for kind in [
            "httpSseServers",
            "httpClients",
            "websocketClients",
            "plugins",
        ] {
            assert!(net[kind].as_array().unwrap().is_empty());
        }
        assert_eq!(onebot["enableLocalFile2Url"], true);
        assert_eq!(onebot["parseMultMsg"], false);
        assert_eq!(onebot["imageDownloadProxy"], "");
        assert_eq!(files["napcat.json"]["fileLog"], false);
        assert_eq!(files["napcat.json"]["consoleLog"], false);
        assert!(web["token"].as_str().unwrap() == bundle.control_token());
        assert!(bundle.api_token() != bundle.control_token());
        bundle
            .visit_environment(|key, value| {
                assert_eq!(key, "NAPCAT_WEBUI_JWT_SECRET_KEY");
                assert!(value != bundle.api_token() && value != bundle.control_token());
                assert!(files.values().all(|file| !file.to_string().contains(value)));
                assert_eq!(value.len(), 64);
                Ok(())
            })
            .unwrap();
    }
}

#[test]
fn independent_bundles_use_new_credentials_and_sanitize_writer_failures() {
    let first = Bundle::generate("4.18.38", ports()).unwrap();
    let second = Bundle::generate("v4.18.38", ports()).unwrap();
    assert!(first.api_token() != second.api_token());
    assert!(first.control_token() != second.control_token());
    for token in [
        first.api_token(),
        first.control_token(),
        second.api_token(),
        second.control_token(),
    ] {
        assert_eq!(token.len(), 64);
        assert!(
            token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
    }
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("untrusted synthetic writer detail"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = first.files().next().unwrap().write_to(Broken).unwrap_err();
    assert_eq!(error.code, "RUNTIME_CONFIG_IO");
    let output = serde_json::to_string(&error).unwrap();
    assert!(!output.contains("untrusted synthetic"));
    assert!(!output.contains(first.api_token()));
    assert!(!output.contains(first.control_token()));
}

#[test]
fn unsupported_versions_privileged_or_colliding_ports_are_rejected() {
    for ports in [
        Ports { http: 0, ..ports() },
        Ports {
            events: 1023,
            ..ports()
        },
        Ports {
            control: 443,
            ..ports()
        },
        Ports {
            events: 19001,
            ..ports()
        },
        Ports {
            control: 19002,
            ..ports()
        },
        Ports {
            control: 19001,
            ..ports()
        },
    ] {
        assert_eq!(
            Bundle::generate("v4.18.38", ports).err().unwrap().code,
            "RUNTIME_CONFIG_INVALID"
        );
    }
    assert_eq!(
        Bundle::generate("latest", ports()).err().unwrap().code,
        "VERSION_UNSUPPORTED"
    );
    assert_eq!(
        Bundle::generate("v4.18.39", ports()).err().unwrap().code,
        "VERSION_UNSUPPORTED"
    );
}
