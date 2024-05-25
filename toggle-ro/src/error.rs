#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<&str> for Error {
    fn from(message: &str) -> Error {
        Error {
            message: String::from(message),
        }
    }
}

impl From<&String> for Error {
    fn from(message: &String) -> Error {
        Error {
            message: message.clone(),
        }
    }
}

impl From<String> for Error {
    fn from(message: String) -> Error {
        Error { message: message }
    }
}

impl From<&std::io::Error> for Error {
    fn from(err: &std::io::Error) -> Error {
        Error {
            message: format!("{:?}", err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error {
            message: format!("{:?}", err),
        }
    }
}

impl From<&gptman::Error> for Error {
    fn from(err: &gptman::Error) -> Error {
        Error {
            message: format!("{:?}", err),
        }
    }
}

impl From<gptman::Error> for Error {
    fn from(err: gptman::Error) -> Error {
        Error {
            message: format!("{:?}", err),
        }
    }
}
