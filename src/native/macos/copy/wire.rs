//! Fixed one-byte requests with exactly two transferred descriptors per copy.
use super::*;
use rustix::net::{
    RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, SendAncillaryBuffer,
    SendAncillaryMessage, SendFlags,
};
use std::io::{IoSlice, IoSliceMut};
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

pub(super) fn send(stream: &UnixStream, mode: u8, descriptors: &[BorrowedFd<'_>]) -> Result<()> {
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(2))];
    let mut control = SendAncillaryBuffer::new(&mut space);
    if !descriptors.is_empty() && !control.push(SendAncillaryMessage::ScmRights(descriptors)) {
        return Err(failed());
    }
    let sent = rustix::net::sendmsg(
        stream,
        &[IoSlice::new(&[mode])],
        &mut control,
        SendFlags::empty(),
    )
    .map_err(|_| failed())?;
    if sent != 1 {
        return Err(failed());
    }
    Ok(())
}

pub(super) fn receive(stream: &UnixStream) -> Result<(u8, Vec<OwnedFd>)> {
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(3))];
    let mut control = RecvAncillaryBuffer::new(&mut space);
    let mut byte = [0_u8];
    let result = rustix::net::recvmsg(
        stream,
        &mut [IoSliceMut::new(&mut byte)],
        &mut control,
        RecvFlags::empty(),
    )
    .map_err(|_| failed())?;
    let mut descriptors = Vec::new();
    for message in control.drain() {
        match message {
            RecvAncillaryMessage::ScmRights(fds) => descriptors.extend(fds),
            _ => return Err(failed()),
        }
    }
    if result.bytes != 1 || !result.flags.is_empty() {
        return Err(failed());
    }
    for fd in &descriptors {
        rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::CLOEXEC).map_err(|_| failed())?;
    }
    match (byte[0], descriptors.len()) {
        (0, 0) | (1 | 2, 2) => Ok((byte[0], descriptors)),
        _ => Err(failed()),
    }
}

pub(super) fn validate(
    source: BorrowedFd<'_>,
    destination: BorrowedFd<'_>,
    data: bool,
) -> Result<()> {
    let src = rustix::fs::fstat(source).map_err(|_| failed())?;
    let dst = rustix::fs::fstat(destination).map_err(|_| failed())?;
    let kind = src.st_mode & libc::S_IFMT;
    if (src.st_dev == dst.st_dev && src.st_ino == dst.st_ino)
        || kind != dst.st_mode & libc::S_IFMT
        || dst.st_uid != rustix::process::geteuid().as_raw()
        || if data {
            kind != libc::S_IFREG || dst.st_nlink != 1 || dst.st_size != 0
        } else {
            kind != libc::S_IFDIR && kind != libc::S_IFLNK
        }
    {
        return Err(failed());
    }
    // The kernel duplicates open descriptions, including seek offsets. Every
    // file request must begin at offset zero; metadata requests do not seek.
    if data {
        for fd in [source, destination] {
            if rustix::fs::seek(fd.as_fd(), rustix::fs::SeekFrom::Current(0))
                .map_err(|_| failed())?
                != 0
            {
                return Err(failed());
            }
        }
    }
    Ok(())
}
