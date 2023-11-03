use std::{collections::HashMap, path::Path, str::FromStr};

use dbus::{
    arg::{self, prop_cast, Append, Arg, ArgType, Get, PropMap},
    Signature,
};

pub trait LocalAppend: Append {}

pub trait FromPropmap: Sized {
    fn from_propmap(map: PropMap) -> Self;
}

pub trait Enum: Sized {
    fn from_u32(num: u32) -> Self;
    fn to_u32(&self) -> u32;
}

struct EnumWrapper<T>(T);

impl<T> Append for EnumWrapper<T>
where
    T: Enum,
{
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.0.to_u32());
        });
    }
}

impl<'a, T> Get<'a> for EnumWrapper<T>
where
    T: Enum,
{
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (num,) = <(u32,)>::get(i)?;
        Some(EnumWrapper(T::from_u32(num)))
    }
}

impl<T> Arg for EnumWrapper<T>
where
    T: Enum,
{
    const ARG_TYPE: arg::ArgType = ArgType::UInt32;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("u\0") }
    }
}

pub struct ConversionError {
    message: &'static str,
}

pub struct Connection {
    settings: ConnectionSettings,
    x802: X802Settings,
    device: TypeSettings,
    ipv4: IPV4Settings,
    ipv6: IPV6Settings,
}

#[derive(Clone, Copy, Default)]
pub enum Trust {
    HOME,
    WORK,
    PUBLIC,
    #[default]
    DEFAULT,
}

impl FromStr for Trust {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Home" => Ok(Trust::HOME),
            "Work" => Ok(Trust::WORK),
            "Public" => Ok(Trust::PUBLIC),
            _ => Ok(Trust::DEFAULT),
        }
    }
}

impl ToString for Trust {
    fn to_string(&self) -> String {
        match self {
            Trust::HOME => String::from("Home"),
            Trust::WORK => String::from("Work"),
            Trust::PUBLIC => String::from("Public"),
            Trust::DEFAULT => String::from("null"),
        }
    }
}

impl Enum for Trust {
    fn from_u32(num: u32) -> Self {
        match num {
            0 => Trust::HOME,
            1 => Trust::WORK,
            2 => Trust::PUBLIC,
            _ => Trust::DEFAULT,
        }
    }

    fn to_u32(&self) -> u32 {
        match self {
            Trust::HOME => 0,
            Trust::WORK => 1,
            Trust::PUBLIC => 2,
            Trust::DEFAULT => 3,
        }
    }
}

#[derive(Default)]
pub enum Mode {
    #[default]
    INFRASTRUCTURE,
    ADHOC,
    AP,
}

impl FromStr for Mode {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "adhoc" => Ok(Mode::ADHOC),
            "ap" => Ok(Mode::AP),
            _ => Ok(Mode::INFRASTRUCTURE),
        }
    }
}

impl ToString for Mode {
    fn to_string(&self) -> String {
        match self {
            Mode::ADHOC => String::from("adhoc"),
            Mode::AP => String::from("ap"),
            Mode::INFRASTRUCTURE => String::from("infrastructure"),
        }
    }
}

impl Enum for Mode {
    fn from_u32(num: u32) -> Self {
        match num {
            0 => Mode::INFRASTRUCTURE,
            1 => Mode::ADHOC,
            _ => Mode::AP,
        }
    }

    fn to_u32(&self) -> u32 {
        match self {
            Mode::INFRASTRUCTURE => 0,
            Mode::ADHOC => 1,
            Mode::AP => 2,
        }
    }
}

#[derive(Default)]
pub enum Band {
    _5GHZ,
    _24GHZ,
    #[default]
    DYN,
}

impl FromStr for Band {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "a" => Ok(Band::_5GHZ),
            "bg" => Ok(Band::_24GHZ),
            _ => Ok(Band::DYN),
        }
    }
}

impl ToString for Band {
    fn to_string(&self) -> String {
        match self {
            Band::_5GHZ => String::from("bg"),
            Band::_24GHZ => String::from("a"),
            Band::DYN => String::from(""),
        }
    }
}

impl Enum for Band {
    fn from_u32(num: u32) -> Self {
        match num {
            0 => Band::_5GHZ,
            1 => Band::_24GHZ,
            _ => Band::DYN,
        }
    }

    fn to_u32(&self) -> u32 {
        match self {
            Band::_5GHZ => 0,
            Band::_24GHZ => 1,
            Band::DYN => 2,
        }
    }
}

#[derive(Default)]
pub enum Duplex {
    HALF,
    #[default]
    FULL,
}

impl FromStr for Duplex {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "half" => Ok(Duplex::HALF),
            _ => Ok(Duplex::FULL),
        }
    }
}

impl ToString for Duplex {
    fn to_string(&self) -> String {
        match self {
            Duplex::HALF => String::from("half"),
            Duplex::FULL => String::from("full"),
        }
    }
}

impl Enum for Duplex {
    fn from_u32(num: u32) -> Self {
        match num {
            0 => Duplex::HALF,
            _ => Duplex::FULL,
        }
    }

    fn to_u32(&self) -> u32 {
        match self {
            Duplex::HALF => 0,
            Duplex::FULL => 1,
        }
    }
}

pub enum TypeSettings {
    WIFI(WifiSettings),
    ETHERNET(EthernetSettings),
    VPN(VPNSettings),
}

struct EthernetSettings {
    auto_negotiate: bool,
    duplex: EnumWrapper<Duplex>,
    mtu: u32,
    name: String,
    speed: u32,
}

impl FromPropmap for EthernetSettings {
    fn from_propmap(map: PropMap) -> Self {
        let auto_negotiate: Option<&bool> = prop_cast(&map, "auto-negotiate");
        let duplex: EnumWrapper<Duplex>;
        let duplex_opt: Option<&String> = prop_cast(&map, "mode");
        if duplex_opt.is_none() {
            duplex = EnumWrapper(Duplex::FULL);
        } else {
            duplex = EnumWrapper(Duplex::from_str(duplex_opt.unwrap().as_str()).ok().unwrap());
        }
        let mtu: Option<&u32> = prop_cast(&map, "mtu");
        let name: String;
        let name_opt: Option<&String> = prop_cast(&map, "name");
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = name_opt.unwrap().clone();
        }
        let speed: Option<&u32> = prop_cast(&map, "speed");
        Self {
            auto_negotiate: *auto_negotiate.unwrap_or_else(|| &true),
            duplex,
            mtu: *mtu.unwrap_or_else(|| &0),
            name,
            speed: *speed.unwrap_or_else(|| &0),
        }
    }
}

struct VPNSettings {
    data: HashMap<String, String>,
    name: String,
    persistent: bool,
    secrets: HashMap<String, String>,
    service_type: String,
    timeout: u32,
    user_name: String,
}

impl FromPropmap for VPNSettings {
    fn from_propmap(map: PropMap) -> Self {
        let data: HashMap<String, String>;
        let name: String;
        let secrets: HashMap<String, String>;
        let service_type: String;
        let user_name: String;

        let data_opt: Option<&HashMap<String, String>> = prop_cast(&map, "data");
        if data_opt.is_none() {
            data = HashMap::new();
        } else {
            data = data_opt.unwrap().clone();
        }
        let name_opt: Option<&String> = prop_cast(&map, "name");
        if name_opt.is_none() {
            name = String::from("vpn");
        } else {
            name = name_opt.unwrap().clone();
        }
        let persistent: Option<&bool> = prop_cast(&map, "persistent");
        let secrets_opt: Option<&HashMap<String, String>> = prop_cast(&map, "secrets");
        if secrets_opt.is_none() {
            secrets = HashMap::new();
        } else {
            secrets = secrets_opt.unwrap().clone();
        }
        let service_type_opt: Option<&String> = prop_cast(&map, "service-type");
        if service_type_opt.is_none() {
            service_type = String::from("");
        } else {
            service_type = service_type_opt.unwrap().clone();
        }
        let timeout: Option<&u32> = prop_cast(&map, "timeout");
        let user_name_opt: Option<&String> = prop_cast(&map, "user-name");
        if user_name_opt.is_none() {
            user_name = String::from("");
        } else {
            user_name = user_name_opt.unwrap().clone();
        }
        Self {
            data,
            name,
            persistent: *persistent.unwrap_or_else(|| &false),
            secrets,
            service_type,
            timeout: *timeout.unwrap_or_else(|| &0),
            user_name,
        }
    }
}

struct WifiSettings {
    band: EnumWrapper<Band>,
    channel: u32,
    mode: EnumWrapper<Mode>,
    mtu: u32,
    powersave: u32,
    rate: u32,
    ssid: Vec<u8>,
}

impl FromPropmap for WifiSettings {
    fn from_propmap(map: PropMap) -> Self {
        let mode: EnumWrapper<Mode>;
        let band: EnumWrapper<Band>;
        let mode_opt: Option<&String> = prop_cast(&map, "mode");
        if mode_opt.is_none() {
            mode = EnumWrapper(Mode::from_str("").ok().unwrap());
        } else {
            mode = EnumWrapper(Mode::from_str(mode_opt.unwrap().as_str()).ok().unwrap());
        }
        let channel = prop_cast(&map, "channel");
        let band_opt: Option<&String> = prop_cast(&map, "band");
        if band_opt.is_none() {
            band = EnumWrapper(Band::from_str("").ok().unwrap());
        } else {
            band = EnumWrapper(Band::from_str(band_opt.unwrap().as_str()).ok().unwrap());
        }
        let mtu = prop_cast(&map, "mtu");
        let powersave = prop_cast(&map, "powersave");
        let rate = prop_cast(&map, "rate");
        let ssid: Vec<u8>;
        let ssid_opt: Option<&Vec<u8>> = prop_cast(&map, "ssid");
        if ssid_opt.is_none() {
            ssid = Vec::new();
        } else {
            ssid = ssid_opt.unwrap().clone();
        }
        Self {
            band,
            channel: *channel.unwrap_or_else(|| &0),
            mode,
            mtu: *mtu.unwrap_or_else(|| &0),
            powersave: *powersave.unwrap_or_else(|| &0),
            rate: *rate.unwrap_or_else(|| &0),
            ssid,
        }
    }
}

struct X802Settings {
    ca_cert: Vec<u8>,
    ca_cert_string: String,
    client_cert: Vec<u8>,
    domain_suffix: String,
    eap: Vec<String>,
    identity: String,
    pac_file: String,
    password: String,
    password_flags: u32,
    password_raw_flags: Vec<u8>,
}

impl FromPropmap for X802Settings {
    fn from_propmap(map: PropMap) -> Self {
        let ca_cert: Vec<u8>;
        let ca_cert_string: String;
        let client_cert: Vec<u8>;
        let domain_suffix: String;
        let eap: Vec<String>;
        let identity: String;
        let pac_file: String;
        let password: String;
        let password_raw_flags: Vec<u8>;
        let password_flags = prop_cast(&map, "password-flags");
        let ca_cert_opt: Option<&Vec<u8>> = prop_cast(&map, "ca-cert");
        if ca_cert_opt.is_none() {
            ca_cert = Vec::new();
        } else {
            ca_cert = ca_cert_opt.unwrap().clone();
        }
        let ca_cert_string_opt: Option<&String> = prop_cast(&map, "ca-cert-string");
        if ca_cert_string_opt.is_none() {
            ca_cert_string = String::new();
        } else {
            ca_cert_string = ca_cert_string_opt.unwrap().clone();
        }
        let client_cert_opt: Option<&Vec<u8>> = prop_cast(&map, "client-cert");
        if client_cert_opt.is_none() {
            client_cert = Vec::new();
        } else {
            client_cert = client_cert_opt.unwrap().clone();
        }
        let domain_suffix_opt: Option<&String> = prop_cast(&map, "domain-suffix");
        if domain_suffix_opt.is_none() {
            domain_suffix = String::from("");
        } else {
            domain_suffix = domain_suffix_opt.unwrap().clone();
        }
        let eap_opt: Option<&Vec<String>> = prop_cast(&map, "eap");
        if eap_opt.is_none() {
            eap = Vec::new();
        } else {
            eap = eap_opt.unwrap().clone();
        }
        let identity_opt: Option<&String> = prop_cast(&map, "identity");
        if identity_opt.is_none() {
            identity = String::from("");
        } else {
            identity = identity_opt.unwrap().clone();
        }
        let pac_file_opt: Option<&String> = prop_cast(&map, "pac-file");
        if pac_file_opt.is_none() {
            pac_file = String::from("");
        } else {
            pac_file = pac_file_opt.unwrap().clone();
        }
        let password_opt: Option<&String> = prop_cast(&map, "password");
        if password_opt.is_none() {
            password = String::from("");
        } else {
            password = password_opt.unwrap().clone();
        }
        let password_raw_flags_opt: Option<&Vec<u8>> = prop_cast(&map, "password-raw-flags");
        if password_raw_flags_opt.is_none() {
            password_raw_flags = Vec::new();
        } else {
            password_raw_flags = password_raw_flags_opt.unwrap().clone();
        }
        Self {
            ca_cert,
            ca_cert_string,
            client_cert,
            domain_suffix,
            eap,
            identity,
            pac_file,
            password,
            password_flags: *password_flags.unwrap_or_else(|| &0),
            password_raw_flags,
        }
    }
}

struct IPV4Settings {}

impl FromPropmap for IPV4Settings {
    fn from_propmap(map: PropMap) -> Self {
        Self {}
    }
}

struct IPV6Settings {}

impl FromPropmap for IPV6Settings {
    fn from_propmap(map: PropMap) -> Self {
        Self {}
    }
}

struct ConnectionSettings {
    autoconnect: bool,
    autoconnect_priority: i32,
    metered: i32,
    name: String,
    device_type: String,
    uuid: String,
    zone: EnumWrapper<Trust>,
}

impl FromPropmap for ConnectionSettings {
    fn from_propmap(map: PropMap) -> Self {
        let autoconnect = prop_cast(&map, "autoconnect");
        let autoconnect_priority = prop_cast(&map, "autoconnect-priority");
        let uuid: String;
        let name: String;
        let device_type: String;
        let metered = prop_cast(&map, "metered");
        let zone: EnumWrapper<Trust>;
        let zone_opt: Option<&String> = prop_cast(&map, "trust");
        if zone_opt.is_none() {
            zone = EnumWrapper(Trust::from_str("").ok().unwrap());
        } else {
            zone = EnumWrapper(Trust::from_str(zone_opt.unwrap().as_str()).ok().unwrap());
        }

        let uuid_opt: Option<&String> = prop_cast(&map, "uuid");
        if uuid_opt.is_none() {
            uuid = String::from("");
        } else {
            uuid = uuid_opt.unwrap().clone();
        }
        let name_opt: Option<&String> = prop_cast(&map, "name");
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = name_opt.unwrap().clone();
        }
        let device_type_opt: Option<&String> = prop_cast(&map, "type");
        if device_type_opt.is_none() {
            device_type = String::from("");
        } else {
            device_type = device_type_opt.unwrap().clone();
        }
        Self {
            autoconnect: *autoconnect.unwrap_or_else(|| &false),
            autoconnect_priority: *autoconnect_priority.unwrap_or_else(|| &-1),
            metered: *metered.unwrap_or_else(|| &-1),
            name,
            device_type,
            uuid,
            zone,
        }
    }
}
