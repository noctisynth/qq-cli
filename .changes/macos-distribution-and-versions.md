---
qq-cli: "patch:feat"
---

Add native Apple Silicon CI configuration and reproducible allowlisted development archives with checksums. Provide a macOS user installer with validated archives, retained version slots, atomic command switching and non-destructive uninstall.

Support reviewed Framework v4.18.41 and v4.18.38 profiles, authenticated update and rollback while preserving instance data and recovery intent. Bound source finalization within startup deadlines and preserve the original failure during cleanup.

Include both Windows and macOS packages in GitHub Release assets. Collect packages from the same quality run, verify their versions and original checksums, and generate a shared checksum manifest for automatic installation on both platforms.
