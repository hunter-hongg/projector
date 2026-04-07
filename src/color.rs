pub fn red(text: &str) -> String {
  format!("\x1b[31m{}\x1b[0m", text)
}

pub fn green(text: &str) -> String {
  format!("\x1b[32m{}\x1b[0m", text)
}

pub fn yellow(text: &str) -> String {
  format!("\x1b[33m{}\x1b[0m", text)
}

pub fn blue(text: &str) -> String {
  format!("\x1b[34m{}\x1b[0m", text)
}

pub fn magenta(text: &str) -> String {
  format!("\x1b[35m{}\x1b[0m", text)
}

pub fn cyan(text: &str) -> String {
  format!("\x1b[36m{}\x1b[0m", text)
}

pub fn white(text: &str) -> String {
  format!("\x1b[37m{}\x1b[0m", text)
}

pub fn error(text: &str) -> String {
  format!("{}{}", red("Error "), text)
}

pub fn info(text: &str) -> String {
  format!("{}{}", green("Info "), text)
}
