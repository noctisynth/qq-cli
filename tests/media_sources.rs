use qq_cli::messaging::media::source::image_identity;

#[test]
fn fixed_image_resources_bind_identity_without_persisting_rotating_keys() {
    let base =
        "https://multimedia.nt.qq.com.cn/download?appid=1407&fileid=synthetic-resource&rkey=unused";
    let identity = image_identity(base).unwrap();
    let rotated = "https://multimedia.nt.qq.com.cn/download?rkey=rotated-key&fileid=synthetic-resource&appid=1407";
    assert_eq!(image_identity(rotated).unwrap(), identity);
    assert_eq!(identity.len(), 64);
    for changed in [
        base.replace("1407", "1406"),
        base.replace("synthetic-resource", "other-resource"),
        base.replace("multimedia.nt.qq.com.cn", "gchat.qpic.cn"),
    ] {
        assert_ne!(image_identity(&changed).unwrap(), identity);
    }
    assert!(
        image_identity(
            "https://gchat.qpic.cn/download?appid=1406&fileid=resource%2Fpart&rkey=key&spec=0"
        )
        .is_some()
    );
}

#[test]
fn fixed_legacy_and_market_image_shapes_are_distinct_and_bounded() {
    let md5 = "0123456789abcdef0123456789abcdef";
    let source = format!("https://gchat.qpic.cn/gchatpic_new/0/0-0-{md5}/0");
    assert_eq!(
        image_identity(&source),
        image_identity(&source.replace(md5, &md5.to_uppercase()))
    );
    let market = format!("https://gxh.vip.qq.com/club/item/parcel/item/01/{md5}/raw300.gif");
    assert!(image_identity(&market).is_some());
    assert_ne!(image_identity(&market), image_identity(&source));
    assert!(image_identity(&market.replace("/01/", "/ff/")).is_none());
    assert!(image_identity(&source.replace(md5, "short")).is_none());
}

#[test]
fn non_audited_image_hosts_paths_credentials_and_queries_are_rejected() {
    assert!(qq_cli::messaging::media::source::can_resolve_image(None));
    for local in [
        "",
        "synthetic-cache/image.png",
        "C:\\synthetic-cache\\image.png",
    ] {
        assert!(qq_cli::messaging::media::source::can_resolve_image(Some(
            local
        )));
    }
    for forbidden in [
        "javascript:ignored",
        "data:image/png;base64,ignored",
        "//invalid.invalid/path",
        "\\\\invalid.invalid\\path",
        "relative\npath",
    ] {
        assert!(!qq_cli::messaging::media::source::can_resolve_image(Some(
            forbidden
        )));
    }
    let base = "https://multimedia.nt.qq.com.cn/download?appid=1407&fileid=resource&rkey=key";
    let mut invalid = vec![
        base.replace("https:", "http:"),
        base.replace(".com.cn/", ".com.cn.evil.invalid/"),
        base.replace(".com.cn/", ".com.cn:443/"),
        base.replace("https://", "https://user:pass@"),
        base.replace("/download", "/other"),
        base.replace("/download", "/other/../download"),
        base.replace("/download", "/%2e%2e/download"),
        base.replace("1407", "1408"),
        base.replace("&rkey=key", ""),
        base.replace("fileid=resource", "fileid="),
        format!("{base}&appid=1407"),
        format!("{base}&%61ppid=1407"),
        format!("{base}&redirect=target"),
        format!("{base}&spec=0"),
        format!("{base}#fragment"),
        format!("{base}\n"),
        base.replace("/download", "\\download"),
        base.replace("fileid=resource", &format!("fileid={}", "a".repeat(4097))),
        "https://127.0.0.1/download?appid=1407&fileid=resource&rkey=key".into(),
        "file:///image.bin".into(),
        "https://gxh.vip.qq.com/club/item/parcel/item/aa/aa-resource/raw300.gif?token=key".into(),
    ];
    invalid.push(base.replace("rkey=key", "rkey=%00"));
    for source in invalid {
        assert!(
            image_identity(&source).is_none(),
            "An unapproved source shape was accepted"
        );
    }
}
