pub mod json;
pub mod yaml;

pub trait FormatSupport<Format>: Sized {
    type SaveError;
    type LoadError;

    fn save(&self) -> Result<Format, Self::SaveError>;
    fn load(data: &Format) -> Result<Self, Self::LoadError>;
}
