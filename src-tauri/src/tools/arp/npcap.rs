use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::net::Ipv4Addr;
use std::os::raw::{c_char, c_int, c_void};
use std::time::{Duration, Instant};

use crate::tools::model::{ArpInterfaceDto, ToolErrorDto, ToolResult};

#[cfg(windows)]
use super::native::format_mac;

#[cfg(windows)]
mod ffi {
    use super::{c_char, c_int, c_void};

    #[repr(C)]
    pub struct PcapIf {
        pub next: *mut PcapIf,
        pub name: *mut c_char,
        pub description: *mut c_char,
        pub addresses: *mut c_void,
        pub flags: u32,
    }

    #[repr(C)]
    pub struct PcapPkthdr {
        pub ts_sec: u32,
        pub ts_usec: u32,
        pub caplen: u32,
        pub len: u32,
    }

    pub type PcapHandle = *mut c_void;

    pub type FindAllDevs = unsafe extern "C" fn(*mut *mut PcapIf, *mut c_char) -> c_int;
    pub type FreeAllDevs = unsafe extern "C" fn(*mut PcapIf);
    pub type OpenLive =
        unsafe extern "C" fn(*const c_char, c_int, c_int, c_int, *mut c_char) -> PcapHandle;
    pub type Close = unsafe extern "C" fn(PcapHandle);
    pub type SendPacket = unsafe extern "C" fn(PcapHandle, *const u8, c_int) -> c_int;
    pub type NextEx =
        unsafe extern "C" fn(PcapHandle, *mut *mut PcapPkthdr, *mut *const u8) -> c_int;
    pub type GetErr = unsafe extern "C" fn(PcapHandle) -> *mut c_char;
}

#[cfg(windows)]
pub struct NpcapApi {
    _library: libloading::Library,
    find_all_devs: ffi::FindAllDevs,
    free_all_devs: ffi::FreeAllDevs,
    open_live: ffi::OpenLive,
    close: ffi::Close,
    send_packet: ffi::SendPacket,
    next_ex: ffi::NextEx,
    get_err: ffi::GetErr,
}

#[cfg(windows)]
impl NpcapApi {
    pub fn load() -> ToolResult<Self> {
        let candidates = [
            "wpcap.dll",
            "C:\\Windows\\System32\\Npcap\\wpcap.dll",
            "C:\\Windows\\System32\\wpcap.dll",
        ];
        let mut last_error = None;
        for candidate in candidates {
            // SAFETY: loading a system capture library by name. The handle is retained for the
            // lifetime of NpcapApi and all resolved symbols are used only while it is alive.
            let library = unsafe { libloading::Library::new(candidate) };
            match library {
                Ok(library) => {
                    // SAFETY: every symbol is resolved before the library is stored, and the
                    // signatures match the public wpcap C API.
                    let api =
                        unsafe {
                            let find_all_devs = *library
                                .get::<ffi::FindAllDevs>(b"pcap_findalldevs\0")
                                .map_err(|error| {
                                    ToolErrorDto::new("npcapUnavailable", error.to_string())
                                })?;
                            let free_all_devs = *library
                                .get::<ffi::FreeAllDevs>(b"pcap_freealldevs\0")
                                .map_err(|error| {
                                    ToolErrorDto::new("npcapUnavailable", error.to_string())
                                })?;
                            let open_live = *library
                                .get::<ffi::OpenLive>(b"pcap_open_live\0")
                                .map_err(|error| {
                                    ToolErrorDto::new("npcapUnavailable", error.to_string())
                                })?;
                            let close =
                                *library
                                    .get::<ffi::Close>(b"pcap_close\0")
                                    .map_err(|error| {
                                        ToolErrorDto::new("npcapUnavailable", error.to_string())
                                    })?;
                            let send_packet = *library
                                .get::<ffi::SendPacket>(b"pcap_sendpacket\0")
                                .map_err(|error| {
                                    ToolErrorDto::new("npcapUnavailable", error.to_string())
                                })?;
                            let next_ex = *library.get::<ffi::NextEx>(b"pcap_next_ex\0").map_err(
                                |error| ToolErrorDto::new("npcapUnavailable", error.to_string()),
                            )?;
                            let get_err =
                                *library
                                    .get::<ffi::GetErr>(b"pcap_geterr\0")
                                    .map_err(|error| {
                                        ToolErrorDto::new("npcapUnavailable", error.to_string())
                                    })?;
                            NpcapApi {
                                _library: library,
                                find_all_devs,
                                free_all_devs,
                                open_live,
                                close,
                                send_packet,
                                next_ex,
                                get_err,
                            }
                        };
                    return Ok(api);
                }
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        Err(ToolErrorDto::new(
            "npcapUnavailable",
            last_error.unwrap_or_else(|| "wpcap.dll was not found".to_owned()),
        ))
    }

    pub fn interfaces(&self) -> ToolResult<Vec<ArpInterfaceDto>> {
        let mut devices: *mut ffi::PcapIf = std::ptr::null_mut();
        let mut errbuf = [0 as c_char; 256];
        // SAFETY: `devices` and `errbuf` are valid pointers of the documented size.
        let result = unsafe { (self.find_all_devs)(&mut devices, errbuf.as_mut_ptr()) };
        if result != 0 || devices.is_null() {
            return Err(ToolErrorDto::new(
                "npcapError",
                "Could not list Npcap interfaces",
            ));
        }
        let mut interfaces = Vec::new();
        let mut current = devices;
        // SAFETY: pcap_findalldevs returns a null-terminated linked list owned by the library.
        unsafe {
            while !current.is_null() {
                let device = &*current;
                if !device.name.is_null() {
                    let name = CStr::from_ptr(device.name).to_string_lossy().into_owned();
                    let description = if device.description.is_null() {
                        name.clone()
                    } else {
                        CStr::from_ptr(device.description)
                            .to_string_lossy()
                            .into_owned()
                    };
                    interfaces.push(ArpInterfaceDto { name, description });
                }
                current = device.next;
            }
            (self.free_all_devs)(devices);
        }
        Ok(interfaces)
    }

    pub fn scan(
        &self,
        interface_name: &str,
        local_ip: Ipv4Addr,
        local_mac: &[u8],
        hosts: &[Ipv4Addr],
        timeout_ms: u64,
    ) -> ToolResult<HashMap<Ipv4Addr, Vec<(String, u32)>>> {
        let interface_name = CString::new(interface_name)
            .map_err(|_| ToolErrorDto::new("invalidInterface", "Invalid interface name"))?;
        let mut errbuf = [0 as c_char; 256];
        // SAFETY: strings and buffers outlive the call. Promiscuous mode is disabled (0).
        let handle = unsafe {
            (self.open_live)(interface_name.as_ptr(), 65_535, 0, 500, errbuf.as_mut_ptr())
        };
        if handle.is_null() {
            // SAFETY: pcap_open_live NUL-terminates the error buffer on failure.
            let message = unsafe { CStr::from_ptr(errbuf.as_ptr()) }.to_string_lossy();
            return Err(ToolErrorDto::new("npcapOpenFailed", message.into_owned()));
        }
        let guard = PcapGuard {
            handle,
            close: self.close,
        };

        let targets: HashMap<Ipv4Addr, ()> = hosts.iter().copied().map(|ip| (ip, ())).collect();
        let source_mac = local_mac
            .get(..6)
            .ok_or_else(|| ToolErrorDto::new("invalidAdapter", "Adapter has no MAC address"))?;
        for host in hosts {
            let frame = arp_request(local_mac, local_ip, *host);
            // SAFETY: the frame buffer is valid for the duration of the call.
            let sent =
                unsafe { (self.send_packet)(guard.handle, frame.as_ptr(), frame.len() as c_int) };
            if sent != 0 {
                return Err(ToolErrorDto::new(
                    "npcapSendFailed",
                    pcap_error(self, guard.handle),
                ));
            }
            std::thread::sleep(Duration::from_micros(500));
        }

        let mut replies: HashMap<Ipv4Addr, Vec<(String, u32)>> = HashMap::new();
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.clamp(500, 10_000));
        while Instant::now() < deadline {
            let mut header: *mut ffi::PcapPkthdr = std::ptr::null_mut();
            let mut data: *const u8 = std::ptr::null();
            // SAFETY: pcap_next_ex writes the header/data pointers; they remain valid until the
            // next capture call.
            let result = unsafe { (self.next_ex)(guard.handle, &mut header, &mut data) };
            if result == 0 {
                continue;
            }
            if result < 0 {
                if result == -2 {
                    break;
                }
                return Err(ToolErrorDto::new(
                    "npcapReceiveFailed",
                    pcap_error(self, guard.handle),
                ));
            }
            if header.is_null() || data.is_null() {
                continue;
            }
            // SAFETY: captured data is valid for the reported capture length.
            let packet = unsafe { std::slice::from_raw_parts(data, (*header).caplen as usize) };
            if let Some((sender_ip, sender_mac)) = parse_arp_reply(packet)
                && targets.contains_key(&sender_ip)
            {
                let entry = replies.entry(sender_ip).or_default();
                let mac = format_mac(&sender_mac);
                if let Some((_, count)) = entry.iter_mut().find(|(known, _)| *known == mac) {
                    *count += 1;
                } else {
                    entry.push((mac, 1));
                }
            }
        }
        let _ = source_mac;
        Ok(replies)
    }
}

#[cfg(windows)]
struct PcapGuard {
    handle: ffi::PcapHandle,
    close: ffi::Close,
}

#[cfg(windows)]
impl Drop for PcapGuard {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by pcap_open_live and is closed exactly once.
        unsafe { (self.close)(self.handle) };
    }
}

#[cfg(windows)]
fn pcap_error(api: &NpcapApi, handle: ffi::PcapHandle) -> String {
    // SAFETY: the error string is owned by the pcap handle and valid until the next call.
    unsafe {
        let pointer = (api.get_err)(handle);
        if pointer.is_null() {
            "Unknown Npcap error".to_owned()
        } else {
            CStr::from_ptr(pointer).to_string_lossy().into_owned()
        }
    }
}

pub fn arp_request(local_mac: &[u8], local_ip: Ipv4Addr, target_ip: Ipv4Addr) -> Vec<u8> {
    let mut frame = Vec::with_capacity(42);
    frame.extend_from_slice(&[0xff; 6]);
    frame.extend_from_slice(local_mac.get(..6).unwrap_or(&[0; 6]));
    frame.extend_from_slice(&[0x08, 0x06]);
    frame.extend_from_slice(&[0x00, 0x01]);
    frame.extend_from_slice(&[0x08, 0x00]);
    frame.push(6);
    frame.push(4);
    frame.extend_from_slice(&[0x00, 0x01]);
    frame.extend_from_slice(local_mac.get(..6).unwrap_or(&[0; 6]));
    frame.extend_from_slice(&local_ip.octets());
    frame.extend_from_slice(&[0x00; 6]);
    frame.extend_from_slice(&target_ip.octets());
    frame
}

pub fn parse_arp_reply(packet: &[u8]) -> Option<(Ipv4Addr, [u8; 6])> {
    if packet.len() < 42 || packet[12] != 0x08 || packet[13] != 0x06 {
        return None;
    }
    let opcode = u16::from_be_bytes([packet[20], packet[21]]);
    if opcode != 2 {
        return None;
    }
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&packet[22..28]);
    let ip = Ipv4Addr::new(packet[28], packet[29], packet[30], packet[31]);
    Some((ip, mac))
}

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem::size_of;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: current process pseudo-handle is valid; token is closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut returned = 0u32;
    // SAFETY: the output buffer matches TOKEN_ELEVATION and the size is exact.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut c_void,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    };
    // SAFETY: token was opened by OpenProcessToken.
    unsafe { CloseHandle(token) };
    ok != 0 && elevation.TokenIsElevated != 0
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(windows)]
pub fn capabilities() -> (bool, bool, Option<String>, Vec<ArpInterfaceDto>) {
    let elevated = is_elevated();
    match NpcapApi::load() {
        Ok(api) => match api.interfaces() {
            Ok(interfaces) => (true, elevated, None, interfaces),
            Err(error) => (false, elevated, Some(error.detail), Vec::new()),
        },
        Err(error) => (false, elevated, Some(error.detail), Vec::new()),
    }
}

#[cfg(not(windows))]
pub fn capabilities() -> (bool, bool, Option<String>, Vec<ArpInterfaceDto>) {
    (
        false,
        false,
        Some("Npcap is only supported on Windows".to_owned()),
        Vec::new(),
    )
}

#[cfg(not(windows))]
pub fn scan(
    _interface_name: &str,
    _local_ip: Ipv4Addr,
    _local_mac: &[u8],
    _hosts: &[Ipv4Addr],
    _timeout_ms: u64,
) -> ToolResult<HashMap<Ipv4Addr, Vec<(String, u32)>>> {
    Err(ToolErrorDto::new(
        "unsupportedPlatform",
        "Enhanced ARP is only supported on Windows",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_parses_arp_packets() {
        let frame = arp_request(
            &[0x02, 0, 0, 0, 0, 1],
            Ipv4Addr::new(192, 168, 1, 10),
            Ipv4Addr::new(192, 168, 1, 20),
        );
        assert_eq!(frame.len(), 42);
        assert_eq!(&frame[0..6], &[0xff; 6]);
        assert_eq!(&frame[38..42], &[192, 168, 1, 20]);

        let mut reply = frame.clone();
        reply[20] = 0;
        reply[21] = 2;
        reply[22..28].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        reply[28..32].copy_from_slice(&[192, 168, 1, 20]);
        let (ip, mac) = parse_arp_reply(&reply).expect("arp reply");
        assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 20));
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn ignores_non_arp_packets() {
        assert!(parse_arp_reply(&[0u8; 42]).is_none());
    }
}
