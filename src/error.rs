use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, WinedroidError>;

#[derive(Debug)]
pub struct WinedroidError {
    message: String,
    missing_context: bool,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl WinedroidError {
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        crate::missing::record_from_error_message(&message);

        Self {
            message,
            missing_context: false,
            source: None,
        }
    }

    pub(crate) fn with_context(mut self, context: &str) -> Self {
        self.missing_context = true;
        self.message.push('\n');
        self.message.push_str(context);
        self
    }

    pub(crate) fn has_missing_context(&self) -> bool {
        self.missing_context
            || self
                .source
                .as_ref()
                .and_then(|source| source.downcast_ref::<Self>())
                .is_some_and(Self::has_missing_context)
    }

    pub(crate) fn missing_primitive(&self) -> Option<&str> {
        if let Some(primitive) = crate::missing::primitive_from_message(&self.message) {
            return Some(primitive);
        }
        self.source
            .as_ref()?
            .downcast_ref::<Self>()?
            .missing_primitive()
    }

    pub fn with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        let message = message.into();
        crate::missing::record_from_error_message(&message);

        Self {
            message,
            missing_context: false,
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for WinedroidError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)?;

        if let Some(source) = &self.source {
            write!(formatter, ": {source}")?;
        }

        Ok(())
    }
}

impl Error for WinedroidError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn Error + 'static))
    }
}

impl From<std::io::Error> for WinedroidError {
    fn from(error: std::io::Error) -> Self {
        Self::with_source("I/O error", error)
    }
}

impl From<zip::result::ZipError> for WinedroidError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::with_source("ZIP archive error", error)
    }
}
