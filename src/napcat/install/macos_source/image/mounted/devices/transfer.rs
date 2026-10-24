//! Metadata is only a description: inherited original raw FDs and live media must agree.
use super::*;
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::napcat::install::macos_source::image) struct Transfer {
    selected: report::Selected,
    generation: media::Generation,
    whole: (u32, u32),
    volume: Option<(u32, u32)>,
}
impl Transfer {
    pub(in crate::napcat::install::macos_source::image) fn has_volume(&self) -> bool {
        self.volume.is_some()
    }
}
impl Devices {
    pub(in crate::napcat::install::macos_source::image) fn into_transfer(
        self,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<(Transfer, File, Option<File>)> {
        self.revalidate(deadline, guard)?;
        let (whole, numbers) = self.whole.into_parts();
        let (volume, volume_numbers) = match self.volume {
            Some(volume) => {
                let (file, numbers) = volume.into_parts();
                (Some(file), Some(numbers))
            }
            None => (None, None),
        };
        Ok((
            Transfer {
                selected: self.selected,
                generation: self.generation,
                whole: numbers,
                volume: volume_numbers,
            },
            whole,
            volume,
        ))
    }
    pub(in crate::napcat::install::macos_source::image) fn from_transfer(
        transfer: Transfer,
        whole: File,
        volume: Option<File>,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        guard()?;
        let selected = media::inspect(&transfer.selected, deadline, guard)?;
        if selected.generation != transfer.generation
            || selected.whole != transfer.whole
            || transfer.volume.is_some() != (transfer.selected.device != transfer.selected.whole)
            || transfer
                .volume
                .is_some_and(|numbers| numbers != selected.volume)
        {
            return Err(invalid());
        }
        let whole = Disk::from_file(whole, transfer.whole.0, transfer.whole.1)?;
        let volume = match (transfer.volume, volume) {
            (Some(numbers), Some(file)) => Some(Disk::from_file(file, numbers.0, numbers.1)?),
            (None, None) => None,
            _ => return Err(invalid()),
        };
        let devices = Self {
            selected: transfer.selected,
            generation: transfer.generation,
            whole,
            volume,
        };
        devices.revalidate(deadline, guard)?;
        Ok(devices)
    }
}
