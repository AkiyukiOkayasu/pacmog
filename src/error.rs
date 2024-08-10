use embedded_error_chain::ErrorCategory;

#[derive(Clone, Copy, ErrorCategory)]
#[repr(u8)]
pub enum DecodingError {
    UnknownFormat,
    UnsupportedBitDepth,
    UnsupportedFormat
}


#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(DecodingError))]
#[repr(u8)]
pub enum ReaderError {
    InvalidChannel,
    InvalidSample,
    DecodingError
}

#[derive(Clone, Copy, ErrorCategory)]
#[repr(u8)]
pub enum PlayerError {
    InvalidOutputBufferLength,
    InvalidData,
    FinishedPlaying
}