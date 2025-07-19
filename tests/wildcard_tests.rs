mod wildcard_tests {
    #![cfg(test)]
    use fin::wildcards::*;

    #[test]
    fn test_wildcard_substring() {
        assert_eq!(wildcard_substring("", "something", b""), None);
        assert_eq!(wildcard_substring("something", "", b""), None);
        assert_eq!(wildcard_substring("some random text", "potato", b""), None);
        assert_eq!(
            wildcard_substring("<+cohad ocnA_IEr (test) _rgyodah_ ictes h", "(*)", b""),
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
        assert!(!match_wildcard("partially matching patern", "*matching"));
        println!("Passed: \"non-match: partially matching pattern\"");
        assert!(match_wildcard("match all", "*"));
        println!("Passed: \"match all\"");
        assert!(match_wildcard(
            "<+cohad ocnA_IEr (test) _rgyodah_ ictes h",
            "*(*)*"
        ));
        println!("Passed: \"surrounding characters\"");
        assert!(match_wildcard("beginning of the string", "*ng"));
        println!("Passed: \"beginning of the string\"");
        assert!(match_wildcard("end of the string", "end*"));
        println!("Passed: \"end of the string\"");
        assert!(match_wildcard("zero-length match star", "*match *star"));
        println!("Passed: \"zero-length match star\"");
        assert!(match_wildcard("zero-length match star at the end", "*end*"));
        println!("Passed: \"zero-length match star at the end\"");
        assert!(!match_wildcard("some random text", "potato"));
        println!("Passed: \"non-match\"");
        assert!(!match_wildcard("", "something"));
        println!("Passed: \"non-match: empty input\"");
        assert!(!match_wildcard("something", ""));
        println!("Passed: \"non-match: no patterns\"");
        assert!(!match_wildcard("something", ""));
        println!("Passed: \"non-match: empty pattern\"");
    }

    #[test]
    fn test_match_any_wildcard() {
        assert!(!match_any_wildcard(
            "partially matching patern",
            &[String::from("*matching")]
        ));
        println!("Passed: \"non-match: partially matching pattern\"");
        assert!(match_any_wildcard("match all", &[String::from("*")]));
        println!("Passed: \"match all\"");
        assert!(match_any_wildcard(
            "<+cohad ocnA_IEr (test) _rgyodah_ ictes h",
            &[String::from("*(*)*")]
        ));
        println!("Passed: \"surrounding characters\"");
        assert!(match_any_wildcard(
            "beginning of the string",
            &[String::from("*ng")]
        ));
        println!("Passed: \"beginning of the string\"");
        assert!(match_any_wildcard(
            "end of the string",
            &[String::from("end*")]
        ));
        println!("Passed: \"end of the string\"");
        assert!(match_any_wildcard(
            "zero-length match star",
            &[String::from("*match *star")]
        ));
        println!("Passed: \"zero-length match star\"");
        assert!(match_any_wildcard(
            "zero-length match star at the end",
            &[String::from("*end*")]
        ));
        println!("Passed: \"zero-length match star at the end\"");
        assert!(!match_any_wildcard(
            "some random text",
            &[String::from("potato")]
        ));
        println!("Passed: \"non-match\"");
        assert!(!match_any_wildcard("", &[String::from("something")]));
        println!("Passed: \"non-match: empty input\"");
        assert!(!match_any_wildcard("something", &[]));
        println!("Passed: \"non-match: no patterns\"");
        assert!(!match_any_wildcard("something", &[String::from("")]));
        println!("Passed: \"non-match: empty pattern\"");
    }

    #[test]
    fn test_match_any_wildcard_new() {
        assert!(!match_any_wildcard_new(
            "partially matching patern",
            &[String::from("*matching")]
        ));
        println!("Passed: \"non-match: partially matching pattern\"");
        assert!(match_any_wildcard_new("match all", &[String::from("*")]));
        println!("Passed: \"match all\"");
        assert!(match_any_wildcard_new(
            "<+cohad ocnA_IEr (test) _rgyodah_ ictes h",
            &[String::from("*(*)*")]
        ));
        println!("Passed: \"surrounding characters\"");
        assert!(match_any_wildcard_new(
            "beginning of the string",
            &[String::from("*ng")]
        ));
        println!("Passed: \"beginning of the string\"");
        assert!(match_any_wildcard_new(
            "end of the string",
            &[String::from("end*")]
        ));
        println!("Passed: \"end of the string\"");
        assert!(match_any_wildcard_new(
            "zero-length match star",
            &[String::from("*match *star")]
        ));
        println!("Passed: \"zero-length match star\"");
        assert!(match_any_wildcard_new(
            "zero-length match star at the end",
            &[String::from("*end*")]
        ));
        println!("Passed: \"zero-length match star at the end\"");
        assert!(!match_any_wildcard_new(
            "some random text",
            &[String::from("potato")]
        ));
        println!("Passed: \"non-match\"");
        assert!(!match_any_wildcard_new("", &[String::from("something")]));
        println!("Passed: \"non-match: empty input\"");
        assert!(!match_any_wildcard_new("something", &[]));
        println!("Passed: \"non-match: no patterns\"");
        assert!(!match_any_wildcard_new("something", &[String::from("")]));
        println!("Passed: \"non-match: empty pattern\"");
    }
}
