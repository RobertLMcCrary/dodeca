use super::*;
use fs_err as fs;

const HOME_PAGE: &str = r#"+++
title = "Home"
+++

# Welcome

Visit the [Guide](/guide/).
"#;

const GUIDE_PAGE: &str = r#"+++
title = "Guide"
+++

## Usage

Back to the [home page](/).
"#;

fn protocol_site() -> InlineSite {
    let site = InlineSite::new(&[("_index.md", HOME_PAGE), ("guide.md", GUIDE_PAGE)]);
    fs::write(
        site.fixture_dir.join(".config/dodeca.styx"),
        r#"content content
output public
protocols {
    gemini true
    gopher true
    gopher_header "Welcome to Gopher"
}
"#,
    )
    .expect("write protocol config");
    site
}

pub fn gemini_output_generated_for_enabled_protocol() {
    let site = protocol_site();
    let result = site.build();
    result.assert_success();

    let output_dir = result.output_dir();
    let http_index = output_dir.join("http/index.html");
    let gemini_index = output_dir.join("gemini/index.gmi");
    let gemini_guide = output_dir.join("gemini/guide/index.gmi");

    assert!(http_index.exists(), "expected HTTP output at {}", http_index.display());
    assert!(
        gemini_index.exists(),
        "expected Gemini output at {}",
        gemini_index.display()
    );
    assert!(
        gemini_guide.exists(),
        "expected Gemini page output at {}",
        gemini_guide.display()
    );

    let root = fs::read_to_string(&gemini_index).expect("read gemini index");
    assert!(
        root.contains("# Welcome"),
        "gemini root should preserve heading, got:\n{}",
        root
    );
    assert!(
        root.contains("=> /guide/ Guide"),
        "gemini root should emit a Gemini link, got:\n{}",
        root
    );
}

pub fn gopher_output_generated_for_enabled_protocol() {
    let site = protocol_site();
    let result = site.build();
    result.assert_success();

    let output_dir = result.output_dir();
    let gopher_index = output_dir.join("gopher/gophermap");
    let gopher_guide = output_dir.join("gopher/guide/gophermap");

    assert!(
        gopher_index.exists(),
        "expected Gopher output at {}",
        gopher_index.display()
    );
    assert!(
        gopher_guide.exists(),
        "expected nested Gopher output at {}",
        gopher_guide.display()
    );

    let root = fs::read_to_string(&gopher_index).expect("read gopher index");
    assert!(
        root.contains("iWelcome to Gopher\tfake\tnull\t0\r\n"),
        "gopher root should include configured header, got:\n{}",
        root
    );
    assert!(
        root.contains("1Guide\t/guide/\t\t70\r\n"),
        "gopher root should emit a menu link, got:\n{}",
        root
    );
    assert!(
        root.ends_with(".\r\n"),
        "gopher output should terminate with dot line, got:\n{}",
        root
    );
}

pub fn link_check_can_be_disabled_in_config() {
    let site = InlineSite::new(&[(
        "_index.md",
        r#"+++
title = "Broken"
+++

See the [missing page](/missing/).
"#,
    )]);
    fs::write(
        site.fixture_dir.join(".config/dodeca.styx"),
        r#"content content
output public
link_check {
    enabled false
}
"#,
    )
    .expect("write config with link checking disabled");

    let result = site.build();
    result
        .assert_success()
        .assert_output_not_contains("Warning:")
        .assert_output_not_contains("/missing/")
        .assert_output_not_contains("Links");
}

pub fn link_check_can_be_disabled_via_cli_flag() {
    let site = InlineSite::new(&[(
        "_index.md",
        r#"+++
title = "Broken"
+++

See the [missing page](/missing/).
"#,
    )]);

    let with_link_check = site.build();
    with_link_check
        .assert_success()
        .assert_output_contains("Warning:")
        .assert_output_contains("/missing/");

    let disabled = site.build_with_args(&["--no-link-check"]);
    disabled
        .assert_success()
        .assert_output_not_contains("Warning:")
        .assert_output_not_contains("/missing/")
        .assert_output_not_contains("Links");
}
