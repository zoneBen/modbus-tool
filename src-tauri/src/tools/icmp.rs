use std::net::Ipv4Addr;

use super::model::ToolErrorDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpStatus {
    Success,
    TtlExpired,
    Unreachable,
    Timeout,
    Error,
}

impl IcmpStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            IcmpStatus::Success => "ok",
            IcmpStatus::TtlExpired => "ttlExpired",
            IcmpStatus::Unreachable => "unreachable",
            IcmpStatus::Timeout => "timeout",
            IcmpStatus::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IcmpReply {
    pub address: Option<Ipv4Addr>,
    pub rtt_ms: Option<f64>,
    pub status: IcmpStatus,
}

#[cfg(windows)]
pub fn ping_once(
    address: Ipv4Addr,
    ttl: u8,
    timeout_ms: u32,
    payload_size: usize,
) -> Result<IcmpReply, ToolErrorDto> {
    use std::ffi::c_void;
    use std::mem::size_of;

    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        ICMP_ECHO_REPLY, IP_DEST_HOST_UNREACHABLE, IP_DEST_NET_UNREACHABLE, IP_OPTION_INFORMATION,
        IP_REQ_TIMED_OUT, IP_SUCCESS, IP_TTL_EXPIRED_TRANSIT, IcmpCloseHandle, IcmpCreateFile,
        IcmpSendEcho,
    };

    struct IcmpHandle(HANDLE);

    impl Drop for IcmpHandle {
        fn drop(&mut self) {
            if self.0 != INVALID_HANDLE_VALUE {
                // SAFETY: the handle was created by IcmpCreateFile and is owned by this guard.
                unsafe { IcmpCloseHandle(self.0) };
            }
        }
    }

    let handle = IcmpHandle(unsafe { IcmpCreateFile() });
    if handle.0 == INVALID_HANDLE_VALUE {
        return Err(ToolErrorDto::new(
            "icmpUnavailable",
            "Could not create an ICMP handle",
        ));
    }

    let payload_size = payload_size.clamp(8, 1400);
    let payload = vec![0x4d_u8; payload_size];
    let options = IP_OPTION_INFORMATION {
        Ttl: ttl.max(1),
        Tos: 0,
        Flags: 0,
        OptionsSize: 0,
        OptionsData: std::ptr::null_mut(),
    };
    let reply_size = (size_of::<ICMP_ECHO_REPLY>() + payload_size + 8) as u32;
    let words = (reply_size as usize).div_ceil(size_of::<u64>());
    let mut reply_buffer = vec![0u64; words];
    let destination = u32::from_ne_bytes(address.octets());

    // SAFETY: the payload, reply buffer and options outlive this synchronous call. The reply
    // buffer is aligned to u64 and large enough for the ICMP header plus requested payload.
    let reply_count = unsafe {
        IcmpSendEcho(
            handle.0,
            destination,
            payload.as_ptr() as *const c_void,
            payload_size as u16,
            &options,
            reply_buffer.as_mut_ptr() as *mut c_void,
            reply_size,
            timeout_ms.clamp(50, 30_000),
        )
    };

    // SAFETY: IcmpSendEcho writes an ICMP_ECHO_REPLY at the start of the aligned buffer.
    let reply = unsafe { &*(reply_buffer.as_ptr() as *const ICMP_ECHO_REPLY) };
    let status = match reply.Status {
        IP_SUCCESS => IcmpStatus::Success,
        IP_TTL_EXPIRED_TRANSIT => IcmpStatus::TtlExpired,
        IP_DEST_HOST_UNREACHABLE | IP_DEST_NET_UNREACHABLE => IcmpStatus::Unreachable,
        IP_REQ_TIMED_OUT => IcmpStatus::Timeout,
        _ => IcmpStatus::Error,
    };
    if reply_count == 0 && reply.Status == 0 {
        return Err(ToolErrorDto::new(
            "icmpSendFailed",
            std::io::Error::last_os_error().to_string(),
        ));
    }

    let reply_address = (reply.Address != 0).then(|| Ipv4Addr::from(reply.Address.to_ne_bytes()));
    let rtt_ms = (reply_count > 0).then_some(f64::from(reply.RoundTripTime));
    Ok(IcmpReply {
        address: reply_address,
        rtt_ms,
        status,
    })
}

#[cfg(not(windows))]
pub fn ping_once(
    _address: Ipv4Addr,
    _ttl: u8,
    _timeout_ms: u32,
    _payload_size: usize,
) -> Result<IcmpReply, ToolErrorDto> {
    Err(ToolErrorDto::new(
        "unsupportedPlatform",
        "ICMP ping is currently supported on Windows only",
    ))
}
