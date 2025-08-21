mod test_wildcards {
    #![cfg(test)]

    use fin::wildcards::*;

    #[test]
    fn test_wildcard_substring() {
        assert_eq!(wildcard_substring("", "something", b""), None);
        assert_eq!(wildcard_substring("something", "", b""), None);
        assert_eq!(wildcard_substring("some random text", "potato", b""), None);
        assert_eq!(
            wildcard_substring("surrounding (test) characters", "(*)", b""),
            Some("(test)")
        );
        assert_eq!(
            wildcard_substring("the beginning of the string", "*ng", b""),
            Some("the beginning")
        );
        assert_eq!(
            wildcard_substring("the end of the string", "end*", b""),
            Some("end of the string")
        );
        assert_eq!(
            wildcard_substring("match from start to hat ^ but no further", "^*^", b""),
            Some("match from start to hat ^")
        );
        assert_eq!(
            wildcard_substring("match from beginning to end of string", "^match*ing$", b""),
            Some("match from beginning to end of string")
        );
        assert_eq!(
            wildcard_substring("match early end of string", "*end$", b""),
            None
        );
        assert_eq!(
            wildcard_substring("zero-length match star", "zero-length match *star", b""),
            Some("zero-length match star")
        );
        assert_eq!(
            wildcard_substring("zero-length match star at the end", "*end*", b""),
            Some("zero-length match star at the end")
        );
        assert_eq!(
            wildcard_substring(
                "disallowed_underscore disallowed underscore",
                "disallowed*underscore",
                &[b'_']
            ),
            Some("disallowed underscore")
        );
    }

    #[test]
    fn test_match_wildcard() {
        if match_wildcard("partially matching patern", "*matching") {
            panic!("Failed: \"partially matching pattern\": expected non-match")
        }
        if !match_wildcard("match all", "*") {
            panic!("Failed: \"match all\": expected match");
        }
        if !match_wildcard("surrounding (test) characters", "*(*)*") {
            panic!("Failed: \"surrounding characters\": expected match");
        }
        if !match_wildcard("beginning of the string", "*ng") {
            println!("Failed: \"beginning of the string\": expected match");
        }
        if !match_wildcard("end of the string", "end*") {
            panic!("Failed: \"end of the string\": expected match");
        }
        if !match_wildcard("zero-length match star", "*match *star") {
            panic!("Failed: \"zero-length match star\": expected match");
        }
        if !match_wildcard("zero-length match star at the end", "*end*") {
            panic!("Failed: \"zero-length match star at the end\": expected match");
        }
        if match_wildcard("some random text", "potato") {
            panic!("Failed: \"non-match\": expected non-match");
        }
        if match_wildcard("", "something") {
            panic!("Failed: \"empty input\": expected non-match");
        }
        if match_wildcard("something", "") {
            panic!("Failed: \"no patterns\": expected non-match");
        }
        if match_wildcard("something", "") {
            panic!("Failed: \"empty pattern\": expected non-match");
        }
    }

    #[test]
    fn test_match_any_wildcard() {
        if match_any_wildcard("partially matching patern", &[String::from("*matching")]) {
            panic!("Failed: \"partially matching pattern\": (expected non-match)");
        }
        if !match_any_wildcard("match all", &[String::from("*")]) {
            panic!("Failed: \"match all\": (expected match)");
        }
        if !match_any_wildcard("surrounding (test) characters", &[String::from("*(*)*")]) {
            panic!("Failed: \"surrounding characters\": (expected match)");
        }
        if !match_any_wildcard("beginning of the string", &[String::from("*ng")]) {
            panic!("Failed: \"beginning of the string\": (expected match)");
        }
        if !match_any_wildcard("end of the string", &[String::from("end*")]) {
            panic!("Failed: \"end of the string\": (expected match)");
        }
        if !match_any_wildcard("zero-length match star", &[String::from("*match *star")]) {
            panic!("Failed: \"zero-length match star\": (expected match)");
        }
        if !match_any_wildcard(
            "zero-length match star at the end",
            &[String::from("*end*")],
        ) {
            panic!("Failed: \"zero-length match star at the end\": (expected match)");
        }
        if match_any_wildcard("some random text", &[String::from("potato")]) {
            panic!("Failed: \"non-match\": (expected non-match)");
        }
        if match_any_wildcard("", &[String::from("something")]) {
            panic!("Failed: \"empty input\": (expected non-match)");
        }
        if match_any_wildcard("something", &[]) {
            panic!("Failed: \"no patterns\": (expected non-match)");
        }
        if match_any_wildcard("something", &[String::from("")]) {
            panic!("Failed: \"empty pattern\": (expected non-match)");
        }
    }

    #[test]
    fn test_match_any_wildcard_mt() {
        if match_any_wildcard_mt(
            "partially matching patern".into(),
            [String::from("*matching")].into(),
        ) {
            panic!("Failed: \"partially matching pattern\": (expected non-match)");
        }
        if !match_any_wildcard_mt("match all".into(), [String::from("*")].into()) {
            panic!("Failed: \"match all\": (expected match)");
        }
        if !match_any_wildcard_mt(
            "surrounding (test) characters".into(),
            [String::from("*(*)*")].into(),
        ) {
            panic!("Failed: \"surrounding characters\": (expected match)");
        }
        if !match_any_wildcard_mt(
            "beginning of the string".into(),
            [String::from("*ng")].into(),
        ) {
            panic!("Failed: \"beginning of the string\": (expected match)");
        }
        if !match_any_wildcard_mt("end of the string".into(), [String::from("end*")].into()) {
            panic!("Failed: \"end of the string\": (expected match)");
        }
        if !match_any_wildcard_mt(
            "zero-length match star".into(),
            [String::from("*match *star")].into(),
        ) {
            panic!("Failed: \"zero-length match star\": (expected match)");
        }
        if !match_any_wildcard_mt(
            "zero-length match star at the end".into(),
            [String::from("*end*")].into(),
        ) {
            panic!("Failed: \"zero-length match star at the end\": (expected match)");
        }
        if match_any_wildcard_mt("some random text".into(), [String::from("potato")].into()) {
            panic!("Failed: \"non-match\": (expected non-match)");
        }
        if match_any_wildcard_mt("".into(), [String::from("something")].into()) {
            panic!("Failed: \"empty input\": (expected non-match)");
        }
        if match_any_wildcard_mt("something".into(), [].into()) {
            panic!("Failed: \"no patterns\": (expected non-match)");
        }
        if match_any_wildcard_mt("something".into(), [String::from("")].into()) {
            panic!("Failed: \"empty pattern\": (expected non-match)");
        }
    }
}
