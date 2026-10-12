use if_addrs::IfAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use tokio::net::{TcpSocket, TcpStream, UdpSocket};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanInterface {
    pub name: String,
    pub ip: Ipv4Addr,
    pub mask: Ipv4Addr,
    pub broadcast: Ipv4Addr,
    pub index: Option<u32>,
}
pub(crate) fn local_interfaces() -> Vec<LanInterface> {
    let mut interfaces = Vec::new();
    let Ok(items) = if_addrs::get_if_addrs() else {
        return interfaces;
    };
    for iface in items {
        if !iface.is_oper_up() || iface.is_p2p() || iface.is_loopback() {
            continue;
        }
        let IfAddr::V4(addr) = iface.addr else {
            continue;
        };
        if !(addr.ip.is_private() || addr.ip.is_link_local())
            || addr.prefixlen == 0
            || addr.prefixlen >= 31
        {
            continue;
        }
        let broadcast = addr
            .broadcast
            .unwrap_or_else(|| Ipv4Addr::from(u32::from(addr.ip) | !u32::from(addr.netmask)));
        interfaces.push(LanInterface {
            name: iface.name,
            ip: addr.ip,
            mask: addr.netmask,
            broadcast,
            index: iface.index,
        });
    }
    interfaces.sort_by_key(|i| i.ip);
    interfaces.dedup_by_key(|i| i.ip);
    interfaces.truncate(12);
    interfaces
}
fn matching<'a>(interfaces: &'a [LanInterface], target: Ipv4Addr) -> Option<&'a LanInterface> {
    interfaces
        .iter()
        .filter(|i| {
            (u32::from(i.ip) & u32::from(i.mask)) == (u32::from(target) & u32::from(i.mask))
        })
        .max_by_key(|i| u32::from(i.mask).count_ones())
}
#[cfg(windows)]
fn select_interface<T: std::os::windows::io::AsRawSocket + ?Sized>(
    socket: &T,
    index: Option<u32>,
) -> std::io::Result<()> {
    use windows_sys::Win32::Networking::WinSock::{
        IP_UNICAST_IF, IPPROTO_IP, SOCKET_ERROR, WSAGetLastError, setsockopt,
    };
    let Some(index) = index else {
        return Ok(());
    };
    let value = index.to_be_bytes();
    let handle = usize::try_from(socket.as_raw_socket()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "socket handle does not fit this platform",
        )
    })?;
    // Winsock consumes this stack buffer during the call; no system route is modified.
    let result = unsafe {
        setsockopt(
            handle,
            IPPROTO_IP,
            IP_UNICAST_IF,
            value.as_ptr(),
            value.len() as i32,
        )
    };
    if result == SOCKET_ERROR {
        return Err(std::io::Error::from_raw_os_error(unsafe {
            WSAGetLastError()
        }));
    }
    Ok(())
}
pub(crate) async fn udp_for(interface: &LanInterface) -> std::io::Result<UdpSocket> {
    let socket = UdpSocket::bind(SocketAddrV4::new(interface.ip, 0)).await?;
    socket.set_broadcast(true)?;
    #[cfg(windows)]
    select_interface(&socket, interface.index)?;
    Ok(socket)
}
pub(crate) async fn connect(target: SocketAddrV4) -> std::io::Result<TcpStream> {
    let interfaces = local_interfaces();
    if let Some(interface) = matching(&interfaces, *target.ip()) {
        let socket = TcpSocket::new_v4()?;
        socket.bind(SocketAddrV4::new(interface.ip, 0).into())?;
        #[cfg(windows)]
        select_interface(&socket, interface.index)?;
        socket.connect(target.into()).await
    } else {
        TcpStream::connect(target).await
    }
}
pub(crate) fn same_subnet(interface: &LanInterface, target: Ipv4Addr) -> bool {
    (u32::from(interface.ip) & u32::from(interface.mask))
        == (u32::from(target) & u32::from(interface.mask))
}
