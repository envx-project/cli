use clap::Parser;
#[test]
fn auth_namespace_and_legacy_status_parse() {
    for args in [vec!["envx","auth"], vec!["envx","auth","--verbose"],vec!["envx","auth","status"],vec!["envx","auth","link"],vec!["envx","auth","login","https://api.envx.sh/#envx-pair-v1=00000000-0000-4000-8000-000000000000"],vec!["envx","auth","gen","--no-upload"]] {
        assert!(crate::Args::try_parse_from(args.clone()).is_ok(),"{args:?}");
    }
}
