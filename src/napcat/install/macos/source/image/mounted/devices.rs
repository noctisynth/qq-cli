//! Raw-device pins prevent normal device-slot reuse; ownership remains a journal responsibility.
use super::*;
use crate::native::macos_disk::Disk;
mod transfer;
pub(in crate::napcat::install::macos::source::image) use transfer::Transfer;

pub struct Devices {
    selected: report::Selected,
    generation: media::Generation,
    whole: Disk,
    volume: Option<Disk>,
}
impl Observation<'_> {
    /// Keep this observation's root alive while opening both raw-device handles.
    /// Returns no authority to unmount; the caller must own a durable release intent.
    pub fn pin_devices(
        &self,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<Devices> {
        self.revalidate(deadline, guard)?;
        let observed = media::inspect(&self.selected, deadline, guard)?;
        if observed.generation != self.media {
            return Err(invalid());
        }
        let whole = Disk::open(&self.selected.whole, observed.whole.0, observed.whole.1)?;
        let volume = if self.selected.device == self.selected.whole {
            None
        } else {
            Some(Disk::open(
                &self.selected.device,
                observed.volume.0,
                observed.volume.1,
            )?)
        };
        // The still-held filesystem root and raw handles overlap. Never drop
        // all original pins and reopen only a reusable device name.
        self.revalidate(deadline, guard)?;
        if media::inspect(&self.selected, deadline, guard)? != observed {
            return Err(invalid());
        }
        let devices = Devices {
            selected: self.selected.clone(),
            generation: self.media.clone(),
            whole,
            volume,
        };
        devices.revalidate(deadline, guard)?;
        Ok(devices)
    }
}
impl Devices {
    /// Fixed volume name for a later supervised ordinary unmount. Pins must
    /// survive that command; no force or unrelated mount-point fallback.
    pub fn volume_name(&self) -> &str {
        &self.selected.device
    }
    pub fn revalidate(&self, deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<()> {
        guard()?;
        self.whole.verify()?;
        if let Some(volume) = &self.volume {
            volume.verify()?;
        }
        if media::observe(&self.selected, deadline, guard)? != self.generation {
            return Err(invalid());
        }
        guard()
    }
    /// Caller has persisted a release intent and completed ordinary unmount.
    /// Consumes all raw handles even on failure; result is not disappearance proof.
    pub fn eject_once(self, deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<()> {
        self.revalidate(deadline, guard)?;
        let Self { whole, volume, .. } = self;
        drop(volume);
        whole.eject_once(deadline, guard)
    }
}
