//! Ordered deployment effects. Source/installation authentication and persistent
//! intent belong to Plan; this kernel also supports original synthetic fixtures.
use super::*;
use crate::napcat::install::macos_bundle::{BundleRole, Inventory};
use crate::storage::state::private_dir;

pub struct Assembly<'a> {
    pub source: &'a Inventory,
    pub roles: &'a [BundleRole],
    pub prepared: &'a Prepared,
    pub adapter: &'a AdapterPlan,
    pub package: &'a PackageEdit,
}
impl Assembly<'_> {
    /// Caller authenticates all inputs and holds durable deployment authority.
    /// Host must dispatch copy, signer-supervisor and policy private workers.
    /// Failure retains partial files and never permits replay or startup.
    pub fn execute(
        &self,
        root: &Path,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<Inventory> {
        crate::core::deadline::check(deadline)?;
        guard()?;
        let expected = self.prepared.adapter_plan()?;
        if root != self.prepared.current_directory().join("QQ.app")
            || self.adapter.digest() != expected.digest()
            || self.adapter.entry() != expected.entry()
            || self.package.destination() != root.join("Contents/Resources/app/package.json")
        {
            return Err(invalid());
        }
        private_dir(&self.prepared.current_directory().join("adapter"))?;
        guard()?;
        self.adapter.publish(&guard)?;
        let checked = || {
            crate::core::deadline::check(deadline)?;
            guard()?;
            self.adapter.verify()?;
            Ok(())
        };
        checked()?;
        let copy = self.source.copy_to_new_isolated(
            self.prepared.current_directory(),
            "QQ.app",
            deadline,
            checked,
        )?;
        let copy = copy.harden_copy(deadline, checked)?;
        let copy = self
            .prepared
            .deploy_package(copy, self.package, deadline, checked)?;
        let plan = copy.signing_plan(self.roles, deadline)?;
        checked()?;
        let copy = copy.sign_copy_supervised(&plan, deadline, checked)?;
        checked()?;
        Ok(copy)
    }
}
