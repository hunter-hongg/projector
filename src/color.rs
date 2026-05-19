macro_rules! make_color {
    ($name:ident, $code:expr) => {
        pub fn $name(text: &str) -> String {
            format!("\x1b[{}m{}\x1b[0m", $code, text)
        }
    };
}

make_color!(red, 31);
make_color!(green, 32);
make_color!(yellow, 33);
make_color!(blue, 34);
make_color!(magenta, 35);
make_color!(cyan, 36);
make_color!(white, 37);

pub fn error(text: &str) -> String {
    format!("{}{}", red("Error: "), text)
}

pub fn info(text: &str) -> String {
    format!("{}{}", green("Info: "), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_red_wraps_ansi() {
        let result = red("hello");
        assert_eq!(result, "\x1b[31mhello\x1b[0m");
    }

    #[test]
    fn test_green_wraps_ansi() {
        let result = green("test");
        assert_eq!(result, "\x1b[32mtest\x1b[0m");
    }

    #[test]
    fn test_error_includes_red_prefix() {
        let result = error("something went wrong");
        assert!(result.starts_with("\x1b[31mError: \x1b[0m"));
        assert!(result.ends_with("something went wrong"));
    }

    #[test]
    fn test_info_includes_green_prefix() {
        let result = info("done");
        assert!(result.starts_with("\x1b[32mInfo: \x1b[0m"));
        assert!(result.ends_with("done"));
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(red(""), "\x1b[31m\x1b[0m");
        assert_eq!(blue(""), "\x1b[34m\x1b[0m");
    }
}
