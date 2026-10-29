//! Reviewed file fields are bound privately; native paths never define logical identity.
use crate::{
    core::error::{AppError, Result},
    messaging::attachments::Snapshot,
    requests::ledger::Intent,
    storage::references::References,
};
use serde_json::{Value, json};
use std::time::Instant;
pub(super) struct Files {
    snapshots: Vec<Snapshot>,
    many: bool,
    field: &'static str,
    persistent: bool,
}
impl Files {
    pub(super) fn new(action: &str, snapshots: Vec<Snapshot>) -> Result<Self> {
        let (many, persistent) = match action {
            "add_custom_face" => (false, true),
            "create_flash_task" => (true, true),
            "send_online_file" | "send_online_folder" => (false, true),
            "upload_private_file"
            | "upload_group_file"
            | "upload_image_to_qun_album"
            | "upload_video_to_qun_album" => (false, false),
            "upload_images_to_qun_album" => (true, false),
            _ => return Err(AppError::invalid()),
        };
        if snapshots.iter().any(Snapshot::is_directory)
            && !matches!(action, "create_flash_task" | "send_online_folder")
        {
            return Err(AppError::invalid());
        }
        if action == "send_online_folder" && snapshots.iter().any(|s| !s.is_directory()) {
            return Err(AppError::invalid());
        }
        if snapshots.is_empty() || snapshots.len() > 32 || (!many && snapshots.len() != 1) {
            return Err(AppError::invalid());
        }
        Ok(Self {
            snapshots,
            field: match action {
                "send_online_file" => "file_path",
                "send_online_folder" => "folder_path",
                _ if many => "files",
                _ => "file",
            },
            many,
            persistent,
        })
    }
    pub(super) fn field(&self) -> &'static str {
        self.field
    }
    pub(super) fn fingerprint(&self) -> Result<Value> {
        if self.many {
            self.snapshots
                .iter()
                .map(Snapshot::logical_fingerprint)
                .collect::<Result<Vec<_>>>()
                .map(|v| json!(v))
        } else {
            self.snapshots[0].logical_fingerprint()
        }
    }
    pub(super) fn retain(
        self,
        intent: &Intent<'_>,
        references: &References,
        deadline: Instant,
    ) -> Result<Value> {
        let paths = self
            .snapshots
            .iter()
            .map(|file| {
                std::path::absolute(file.path())
                    .map_err(|_| AppError::invalid())?
                    .to_str()
                    .map(str::to_owned)
                    .ok_or_else(AppError::invalid)
            })
            .collect::<Result<Vec<_>>>()?;
        for file in self.snapshots {
            if self.persistent {
                let _retained = file.retain_persistent_for_intent(intent, references, deadline)?;
            } else {
                let _retained = file.retain_for_intent(intent, references, deadline)?;
            }
        }
        Ok(if self.many {
            json!(paths)
        } else {
            json!(paths[0])
        })
    }
}
