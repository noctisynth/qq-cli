//! CLI names for the protocol format enum.
use crate::core::contracts::voice::Format;
impl clap::ValueEnum for Format {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Original,
            Self::Mp3,
            Self::Amr,
            Self::Wma,
            Self::M4a,
            Self::Spx,
            Self::Ogg,
            Self::Wav,
            Self::Flac,
        ]
    }
    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(
            if *self == Self::Original {
                "original"
            } else {
                self.wire()
            },
        ))
    }
}
