use std::{collections::HashMap, str::FromStr};

use dbus::arg::{self, prop_cast, PropMap, Variant};

pub trait PropMapConvert: Sized {
    fn from_propmap(map: PropMap) -> Self;
    fn to_propmap(&self, map: &mut PropMap);
}

pub trait Enum: Sized {
    fn from_i32(num: i32) -> Self;
    fn to_i32(&self) -> i32;
}

#[derive(Debug)]
pub struct ConversionError {
    message: &'static str,
}

#[derive(Debug)]
pub struct Connection {
    settings: ConnectionSettings,
    // x802: X802Settings,
    device: TypeSettings,
    ipv4: IPV4Settings,
    ipv6: IPV6Settings,
    // TODO check if x802 is actually even necessary?
    // TODO implement wifi security settings
}

impl Connection {
    pub fn convert_from_propmap(map: HashMap<String, PropMap>) -> Result<Self, ConversionError> {
        let mut settings: Option<ConnectionSettings> = None;
        // let mut x802: Option<X802Settings> = None;
        let mut device: Option<TypeSettings> = None;
        let mut ipv4: Option<IPV4Settings> = None;
        let mut ipv6: Option<IPV6Settings> = None;
        for (category, submap) in map {
            match category.as_str() {
                "802-11-wireless" => {
                    device = Some(TypeSettings::WIFI(WifiSettings::from_propmap(submap)))
                }
                "802-3-ethernet" => {
                    device = Some(TypeSettings::ETHERNET(EthernetSettings::from_propmap(
                        submap,
                    )))
                }
                "vpn" => device = Some(TypeSettings::VPN(VPNSettings::from_propmap(submap))),
                "ipv6" => ipv6 = Some(IPV6Settings::from_propmap(submap)),
                "ipv4" => ipv4 = Some(IPV4Settings::from_propmap(submap)),
                "connection" => settings = Some(ConnectionSettings::from_propmap(submap)),
                // "802-1x" => x802 = Some(X802Settings::from_propmap(submap)),
                _ => continue,
            }
        }
        if settings.is_none() | device.is_none() | ipv4.is_none() | ipv6.is_none() {
            return Err(ConversionError {
                message: "could not convert propmap",
            });
        }
        let settings = settings.unwrap();
        // let x802 = x802.unwrap();
        let device = device.unwrap();
        let ipv4 = ipv4.unwrap();
        let ipv6 = ipv6.unwrap();
        Ok(Self {
            settings,
            // x802,
            device,
            ipv4,
            ipv6,
        })
    }

    pub fn convert_to_propmap(&self) -> PropMap {
        let mut map = PropMap::new();
        self.settings.to_propmap(&mut map);
        match &self.device {
            TypeSettings::WIFI(wifi) => wifi.to_propmap(&mut map),
            TypeSettings::ETHERNET(ethernet) => ethernet.to_propmap(&mut map),
            TypeSettings::VPN(vpn) => vpn.to_propmap(&mut map),
        }
        self.ipv4.to_propmap(&mut map);
        self.ipv6.to_propmap(&mut map);
        map
    }
}

#[derive(Clone, Copy, Default, Debug)]
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
    fn from_i32(num: i32) -> Self {
        match num {
            0 => Trust::HOME,
            1 => Trust::WORK,
            2 => Trust::PUBLIC,
            _ => Trust::DEFAULT,
        }
    }

    fn to_i32(&self) -> i32 {
        match self {
            Trust::HOME => 0,
            Trust::WORK => 1,
            Trust::PUBLIC => 2,
            Trust::DEFAULT => 3,
        }
    }
}

#[derive(Default, Debug, Clone)]
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
    fn from_i32(num: i32) -> Self {
        match num {
            0 => Mode::INFRASTRUCTURE,
            1 => Mode::ADHOC,
            _ => Mode::AP,
        }
    }

    fn to_i32(&self) -> i32 {
        match self {
            Mode::INFRASTRUCTURE => 0,
            Mode::ADHOC => 1,
            Mode::AP => 2,
        }
    }
}

#[derive(Default, Debug, Clone)]
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
    fn from_i32(num: i32) -> Self {
        match num {
            0 => Band::_5GHZ,
            1 => Band::_24GHZ,
            _ => Band::DYN,
        }
    }

    fn to_i32(&self) -> i32 {
        match self {
            Band::_5GHZ => 0,
            Band::_24GHZ => 1,
            Band::DYN => 2,
        }
    }
}

#[derive(Default, Debug, Clone)]
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
    fn from_i32(num: i32) -> Self {
        match num {
            0 => Duplex::HALF,
            _ => Duplex::FULL,
        }
    }

    fn to_i32(&self) -> i32 {
        match self {
            Duplex::HALF => 0,
            Duplex::FULL => 1,
        }
    }
}

#[derive(Debug)]
pub enum TypeSettings {
    WIFI(WifiSettings),
    ETHERNET(EthernetSettings),
    VPN(VPNSettings),
}

impl ToString for TypeSettings {
    fn to_string(&self) -> String {
        match self {
            TypeSettings::WIFI(_) => String::from("wifi"),
            TypeSettings::ETHERNET(_) => String::from("ethernet"),
            TypeSettings::VPN(_) => String::from("vpn"),
        }
    }
}

#[derive(Debug, Clone)]
struct EthernetSettings {
    auto_negotiate: bool,
    duplex: Duplex,
    mtu: u32,
    name: String,
    speed: u32,
}

impl PropMapConvert for EthernetSettings {
    fn from_propmap(map: PropMap) -> Self {
        let auto_negotiate: Option<&bool> = prop_cast(&map, "auto-negotiate");
        let duplex: Duplex;
        let duplex_opt: Option<&String> = prop_cast(&map, "mode");
        if duplex_opt.is_none() {
            duplex = Duplex::FULL;
        } else {
            duplex = Duplex::from_str(duplex_opt.unwrap().as_str()).ok().unwrap();
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

    fn to_propmap(&self, map: &mut PropMap) {
        map.insert(
            "auto-negotiate".into(),
            Variant(Box::new(self.auto_negotiate)),
        );
        map.insert("duplex".into(), Variant(Box::new(self.duplex.to_i32())));
        map.insert("mtu".into(), Variant(Box::new(self.mtu)));
        map.insert("name".into(), Variant(Box::new(self.name.clone())));
        map.insert("speed".into(), Variant(Box::new(self.speed)));
    }
}

#[derive(Debug, Clone)]
struct VPNSettings {
    data: HashMap<String, String>,
    name: String,
    persistent: bool,
    secrets: HashMap<String, String>,
    service_type: String,
    timeout: u32,
    user_name: String,
}

impl PropMapConvert for VPNSettings {
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

    fn to_propmap(&self, map: &mut PropMap) {
        map.insert("data".into(), Variant(Box::new(self.data.clone())));
        map.insert("name".into(), Variant(Box::new(self.name.clone())));
        map.insert("persistent".into(), Variant(Box::new(self.persistent)));
        map.insert("secrets".into(), Variant(Box::new(self.secrets.clone())));
        map.insert(
            "service-type".into(),
            Variant(Box::new(self.service_type.clone())),
        );
        map.insert("timeout".into(), Variant(Box::new(self.timeout)));
        map.insert(
            "user-name".into(),
            Variant(Box::new(self.user_name.clone())),
        );
    }
}

#[derive(Debug, Clone)]
struct WifiSettings {
    band: Band,
    channel: u32,
    mode: Mode,
    mtu: u32,
    powersave: u32,
    rate: u32,
    ssid: Vec<u8>,
}

impl PropMapConvert for WifiSettings {
    fn from_propmap(map: PropMap) -> Self {
        println!("wifi debug");
        for (key, val) in map.iter() {
            dbg!(key);
            dbg!(val);
        }
        let mode: Mode;
        let band: Band;
        let mode_opt: Option<&String> = prop_cast(&map, "mode");
        if mode_opt.is_none() {
            mode = Mode::from_str("").ok().unwrap();
        } else {
            mode = Mode::from_str(mode_opt.unwrap().as_str()).ok().unwrap();
        }
        let channel = prop_cast(&map, "channel");
        let band_opt: Option<&String> = prop_cast(&map, "band");
        if band_opt.is_none() {
            band = Band::from_str("").ok().unwrap();
        } else {
            band = Band::from_str(band_opt.unwrap().as_str()).ok().unwrap();
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

    fn to_propmap(&self, map: &mut PropMap) {
        map.insert("band".into(), Variant(Box::new(self.band.to_i32())));
        map.insert("channel".into(), Variant(Box::new(self.channel)));
        map.insert("mode".into(), Variant(Box::new(self.mode.to_i32())));
        map.insert("mtu".into(), Variant(Box::new(self.mtu)));
        map.insert("powersave".into(), Variant(Box::new(self.powersave)));
        map.insert("rate".into(), Variant(Box::new(self.rate)));
        map.insert("ssid".into(), Variant(Box::new(self.ssid.clone())));
    }
}

#[derive(Debug)]
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

// impl PropMapConvert for X802Settings {
//     fn from_propmap(map: PropMap) -> Self {
//         println!("x802 debug");
//         for (key, val) in map.iter() {
//             dbg!(key);
//             dbg!(val);
//         }
//         let ca_cert: Vec<u8>;
//         let ca_cert_string: String;
//         let client_cert: Vec<u8>;
//         let domain_suffix: String;
//         let eap: Vec<String>;
//         let identity: String;
//         let pac_file: String;
//         let password: String;
//         let password_raw_flags: Vec<u8>;
//         let password_flags = prop_cast(&map, "password-flags");
//         let ca_cert_opt: Option<&Vec<u8>> = prop_cast(&map, "ca-cert");
//         if ca_cert_opt.is_none() {
//             ca_cert = Vec::new();
//         } else {
//             ca_cert = ca_cert_opt.unwrap().clone();
//         }
//         let ca_cert_string_opt: Option<&String> = prop_cast(&map, "ca-cert-string");
//         if ca_cert_string_opt.is_none() {
//             ca_cert_string = String::new();
//         } else {
//             ca_cert_string = ca_cert_string_opt.unwrap().clone();
//         }
//         let client_cert_opt: Option<&Vec<u8>> = prop_cast(&map, "client-cert");
//         if client_cert_opt.is_none() {
//             client_cert = Vec::new();
//         } else {
//             client_cert = client_cert_opt.unwrap().clone();
//         }
//         let domain_suffix_opt: Option<&String> = prop_cast(&map, "domain-suffix");
//         if domain_suffix_opt.is_none() {
//             domain_suffix = String::from("");
//         } else {
//             domain_suffix = domain_suffix_opt.unwrap().clone();
//         }
//         let eap_opt: Option<&Vec<String>> = prop_cast(&map, "eap");
//         if eap_opt.is_none() {
//             eap = Vec::new();
//         } else {
//             eap = eap_opt.unwrap().clone();
//         }
//         let identity_opt: Option<&String> = prop_cast(&map, "identity");
//         if identity_opt.is_none() {
//             identity = String::from("");
//         } else {
//             identity = identity_opt.unwrap().clone();
//         }
//         let pac_file_opt: Option<&String> = prop_cast(&map, "pac-file");
//         if pac_file_opt.is_none() {
//             pac_file = String::from("");
//         } else {
//             pac_file = pac_file_opt.unwrap().clone();
//         }
//         let password_opt: Option<&String> = prop_cast(&map, "password");
//         if password_opt.is_none() {
//             password = String::from("");
//         } else {
//             password = password_opt.unwrap().clone();
//         }
//         let password_raw_flags_opt: Option<&Vec<u8>> = prop_cast(&map, "password-raw-flags");
//         if password_raw_flags_opt.is_none() {
//             password_raw_flags = Vec::new();
//         } else {
//             password_raw_flags = password_raw_flags_opt.unwrap().clone();
//         }
//         Self {
//             ca_cert,
//             ca_cert_string,
//             client_cert,
//             domain_suffix,
//             eap,
//             identity,
//             pac_file,
//             password,
//             password_flags: *password_flags.unwrap_or_else(|| &0),
//             password_raw_flags,
//         }
//     }
// }

#[derive(Debug)]
struct Address {
    address: String,
    prefix_length: u32,
}

impl Address {
    pub fn to_map(&self) -> PropMap {
        let mut map = PropMap::new();
        map.insert("address".into(), Variant(Box::new(self.address.clone())));
        map.insert(
            "prefix-length".into(),
            Variant(Box::new(self.prefix_length)),
        );
        map
    }
}

#[derive(Debug, Default)]
enum DNSMethod {
    #[default]
    AUTO,
    MANUAL,
    LINKLOCAL,
    SHARED,
    DISABLED,
}

impl FromStr for DNSMethod {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(DNSMethod::AUTO),
            "manual" => Ok(DNSMethod::MANUAL),
            "link-local" => Ok(DNSMethod::LINKLOCAL),
            "shared" => Ok(DNSMethod::SHARED),
            _ => Ok(DNSMethod::DISABLED),
        }
    }
}

impl ToString for DNSMethod {
    fn to_string(&self) -> String {
        match self {
            DNSMethod::AUTO => String::from("auto"),
            DNSMethod::MANUAL => String::from("manual"),
            DNSMethod::LINKLOCAL => String::from("link-local"),
            DNSMethod::SHARED => String::from("shared"),
            DNSMethod::DISABLED => String::from("disabled"),
        }
    }
}

impl Enum for DNSMethod {
    fn from_i32(num: i32) -> Self {
        match num {
            0 => DNSMethod::AUTO,
            1 => DNSMethod::MANUAL,
            2 => DNSMethod::LINKLOCAL,
            3 => DNSMethod::SHARED,
            _ => DNSMethod::DISABLED,
        }
    }

    fn to_i32(&self) -> i32 {
        match self {
            DNSMethod::AUTO => 0,
            DNSMethod::MANUAL => 1,
            DNSMethod::LINKLOCAL => 2,
            DNSMethod::SHARED => 3,
            DNSMethod::DISABLED => 4,
        }
    }
}

#[derive(Debug)]
struct IPV4Settings {
    address_data: Vec<Address>,
    dns: Vec<Vec<u8>>,
    dns_options: Vec<String>,
    dns_priority: i32,
    dns_search: Vec<String>,
    gateway: String,
    ignore_auto_dns: bool,
    ignore_auto_dns_routes: bool,
    may_fail: bool,
    dns_method: DNSMethod,
    never_default: bool,
    route_data: Vec<Address>,
}

impl PropMapConvert for IPV4Settings {
    fn from_propmap(map: PropMap) -> Self {
        println!("ipv4 debug");
        for (key, val) in map.iter() {
            dbg!(key);
            dbg!(val);
        }
        let address_data = get_addresses(&map, "address-data");
        let dns: Vec<Vec<u8>>;
        let dns_opt: Option<&Vec<Vec<u8>>> = prop_cast(&map, "dns");
        if dns_opt.is_none() {
            dns = Vec::new();
        } else {
            dns = dns_opt.unwrap().clone();
        }
        let dns_options: Vec<String>;
        let dns_options_opt: Option<&Vec<String>> = prop_cast(&map, "dns-options");
        if dns_options_opt.is_none() {
            dns_options = Vec::new();
        } else {
            dns_options = dns_options_opt.unwrap().clone();
        }
        let dns_priority = *prop_cast(&map, "dns-priority").unwrap_or_else(|| &0);
        let dns_search: Vec<String>;
        let dns_search_opt: Option<&Vec<String>> = prop_cast(&map, "dns-search");
        if dns_search_opt.is_none() {
            dns_search = Vec::new();
        } else {
            dns_search = dns_search_opt.unwrap().clone();
        }
        let gateway: String;
        let gateway_opt: Option<&String> = prop_cast(&map, "gateway");
        if gateway_opt.is_none() {
            gateway = String::from("");
        } else {
            gateway = gateway_opt.unwrap().clone();
        }
        let ignore_auto_dns = *prop_cast(&map, "ignore-auto-dns").unwrap_or_else(|| &false);
        let ignore_auto_dns_routes =
            *prop_cast(&map, "ignore-auto-dns-routes").unwrap_or_else(|| &false);
        let may_fail = *prop_cast(&map, "may-fail").unwrap_or_else(|| &true);
        let dns_method: DNSMethod;
        let method_opt: Option<&String> = prop_cast(&map, "method");
        if method_opt.is_none() {
            dns_method = DNSMethod::DISABLED;
        } else {
            dns_method = DNSMethod::from_str(method_opt.unwrap().as_str()).unwrap();
        }
        let never_default = *prop_cast(&map, "never-default").unwrap_or_else(|| &true);
        let route_data = get_addresses(&map, "route-data");
        Self {
            address_data,
            dns,
            dns_options,
            dns_priority,
            dns_search,
            gateway,
            ignore_auto_dns,
            ignore_auto_dns_routes,
            may_fail,
            dns_method,
            never_default,
            route_data,
        }
    }

    fn to_propmap(&self, map: &mut PropMap) {
        let mut addresses = Vec::new();
        for address in self.address_data.iter() {
            addresses.push(address.to_map());
        }
        map.insert("address-data".into(), Variant(Box::new(addresses)));
        map.insert("dns".into(), Variant(Box::new(self.dns.clone())));
        map.insert(
            "dns-options".into(),
            Variant(Box::new(self.dns_options.clone())),
        );
        map.insert("dns-priority".into(), Variant(Box::new(self.dns_priority)));
        map.insert(
            "dns-search".into(),
            Variant(Box::new(self.dns_search.clone())),
        );
        map.insert("gateway".into(), Variant(Box::new(self.gateway.clone())));
        map.insert(
            "ignore-auto-dns".into(),
            Variant(Box::new(self.ignore_auto_dns)),
        );
        map.insert(
            "ignore-auto-dns-routes".into(),
            Variant(Box::new(self.ignore_auto_dns_routes)),
        );
        map.insert("may-fail".into(), Variant(Box::new(self.may_fail)));
        map.insert(
            "dns-method".into(),
            Variant(Box::new(self.dns_method.to_i32())),
        );
        map.insert(
            "never-default".into(),
            Variant(Box::new(self.never_default)),
        );
        let mut data = Vec::new();
        for address in self.address_data.iter() {
            data.push(address.to_map());
        }
        map.insert("route-data".into(), Variant(Box::new(data)));
    }
}

#[derive(Debug, Default)]
enum IPV6PrivacyMode {
    DISABLED,
    ENABLEDPEFERPUBLIC,
    ENABLEDPEFERTEMPORARY,
    #[default]
    UNKNOWN,
}

impl FromStr for IPV6PrivacyMode {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "disabled" => Ok(IPV6PrivacyMode::DISABLED),
            "enabled-prefer-public" => Ok(IPV6PrivacyMode::ENABLEDPEFERPUBLIC),
            "enabled-prefer-temporary" => Ok(IPV6PrivacyMode::ENABLEDPEFERTEMPORARY),
            _ => Ok(IPV6PrivacyMode::UNKNOWN),
        }
    }
}

impl ToString for IPV6PrivacyMode {
    fn to_string(&self) -> String {
        match self {
            IPV6PrivacyMode::UNKNOWN => String::from("unknown"),
            IPV6PrivacyMode::DISABLED => String::from("disabled"),
            IPV6PrivacyMode::ENABLEDPEFERPUBLIC => String::from("enabled-prefer-public"),
            IPV6PrivacyMode::ENABLEDPEFERTEMPORARY => String::from("enabled-prefer-temporary"),
        }
    }
}

impl Enum for IPV6PrivacyMode {
    fn from_i32(num: i32) -> Self {
        match num {
            -1 => IPV6PrivacyMode::UNKNOWN,
            0 => IPV6PrivacyMode::DISABLED,
            1 => IPV6PrivacyMode::ENABLEDPEFERPUBLIC,
            _ => IPV6PrivacyMode::ENABLEDPEFERTEMPORARY,
        }
    }

    fn to_i32(&self) -> i32 {
        match self {
            IPV6PrivacyMode::UNKNOWN => -1,
            IPV6PrivacyMode::DISABLED => 0,
            IPV6PrivacyMode::ENABLEDPEFERPUBLIC => 1,
            IPV6PrivacyMode::ENABLEDPEFERTEMPORARY => 2,
        }
    }
}

#[derive(Debug)]
struct IPV6Settings {
    address_data: Vec<Address>,
    dns: Vec<Vec<u8>>,
    dns_options: Vec<String>,
    dns_priority: i32,
    dns_search: Vec<String>,
    gateway: String,
    ignore_auto_dns: bool,
    ignore_auto_dns_routes: bool,
    ipv6_privacy: IPV6PrivacyMode,
    may_fail: bool,
    dns_method: DNSMethod,
    never_default: bool,
    route_data: Vec<Address>,
}

impl PropMapConvert for IPV6Settings {
    fn from_propmap(map: PropMap) -> Self {
        println!("ipv6 debug");
        for (key, val) in map.iter() {
            dbg!(key);
            dbg!(val);
        }
        let address_data = get_addresses(&map, "address-data");
        let dns: Vec<Vec<u8>>;
        let dns_opt: Option<&Vec<Vec<u8>>> = prop_cast(&map, "dns");
        if dns_opt.is_none() {
            dns = Vec::new();
        } else {
            dns = dns_opt.unwrap().clone();
        }
        let dns_options: Vec<String>;
        let dns_options_opt: Option<&Vec<String>> = prop_cast(&map, "dns-options");
        if dns_options_opt.is_none() {
            dns_options = Vec::new();
        } else {
            dns_options = dns_options_opt.unwrap().clone();
        }
        let dns_priority = *prop_cast(&map, "dns-priority").unwrap_or_else(|| &0);
        let dns_search: Vec<String>;
        let dns_search_opt: Option<&Vec<String>> = prop_cast(&map, "dns-search");
        if dns_search_opt.is_none() {
            dns_search = Vec::new();
        } else {
            dns_search = dns_search_opt.unwrap().clone();
        }
        let gateway: String;
        let gateway_opt: Option<&String> = prop_cast(&map, "gateway");
        if gateway_opt.is_none() {
            gateway = String::from("");
        } else {
            gateway = gateway_opt.unwrap().clone();
        }
        let ignore_auto_dns = *prop_cast(&map, "ignore-auto-dns").unwrap_or_else(|| &false);
        let ignore_auto_dns_routes =
            *prop_cast(&map, "ignore-auto-dns-routes").unwrap_or_else(|| &false);
        let ipv6_privacy =
            IPV6PrivacyMode::from_i32(*prop_cast(&map, "ip6-privacy").unwrap_or_else(|| &-1));
        let may_fail = *prop_cast(&map, "may-fail").unwrap_or_else(|| &true);
        let dns_method: DNSMethod;
        let method_opt: Option<&String> = prop_cast(&map, "method");
        if method_opt.is_none() {
            dns_method = DNSMethod::DISABLED;
        } else {
            dns_method = DNSMethod::from_str(method_opt.unwrap().as_str()).unwrap();
        }
        let never_default = *prop_cast(&map, "never-default").unwrap_or_else(|| &true);
        let route_data = get_addresses(&map, "route-data");
        Self {
            address_data,
            dns,
            dns_options,
            dns_priority,
            dns_search,
            gateway,
            ignore_auto_dns,
            ignore_auto_dns_routes,
            ipv6_privacy,
            may_fail,
            dns_method,
            never_default,
            route_data,
        }
    }

    fn to_propmap(&self, map: &mut PropMap) {
        let mut addresses = Vec::new();
        for address in self.address_data.iter() {
            addresses.push(address.to_map());
        }
        map.insert("address-data".into(), Variant(Box::new(addresses)));
        map.insert("dns".into(), Variant(Box::new(self.dns.clone())));
        map.insert(
            "dns-options".into(),
            Variant(Box::new(self.dns_options.clone())),
        );
        map.insert("dns-priority".into(), Variant(Box::new(self.dns_priority)));
        map.insert(
            "dns-search".into(),
            Variant(Box::new(self.dns_search.clone())),
        );
        map.insert("gateway".into(), Variant(Box::new(self.gateway.clone())));
        map.insert(
            "ignore-auto-dns".into(),
            Variant(Box::new(self.ignore_auto_dns)),
        );
        map.insert(
            "ignore-auto-dns-routes".into(),
            Variant(Box::new(self.ignore_auto_dns_routes)),
        );
        map.insert(
            "ipv6-privacy".into(),
            Variant(Box::new(self.ipv6_privacy.to_i32())),
        );
        map.insert("may-fail".into(), Variant(Box::new(self.may_fail)));
        map.insert(
            "dns-method".into(),
            Variant(Box::new(self.dns_method.to_i32())),
        );
        map.insert(
            "never-default".into(),
            Variant(Box::new(self.never_default)),
        );
        let mut data = Vec::new();
        for address in self.address_data.iter() {
            data.push(address.to_map());
        }
        map.insert("route-data".into(), Variant(Box::new(data)));
    }
}

fn get_addresses(map: &PropMap, address_type: &'static str) -> Vec<Address> {
    let mut address_data: Vec<Address> = Vec::new();
    let address_data_opt: Option<&Vec<PropMap>> = prop_cast(map, address_type);
    if address_data_opt.is_some() {
        for entry in address_data_opt.unwrap() {
            let address: String;
            let prefix_length: u32;
            let address_opt = entry.get("address");
            let prefix_length_opt = entry.get("prefix");
            if address_data_opt.is_none() {
                address = String::from("");
            } else {
                address = arg::cast::<String>(address_opt.unwrap()).unwrap().clone();
            }
            if prefix_length_opt.is_none() {
                prefix_length = 0;
            } else {
                prefix_length = arg::cast::<u32>(prefix_length_opt.unwrap())
                    .unwrap()
                    .clone();
            }

            address_data.push(Address {
                address,
                prefix_length,
            })
        }
    }
    address_data
}

#[derive(Debug)]
struct ConnectionSettings {
    autoconnect: bool,
    autoconnect_priority: i32,
    metered: i32,
    name: String,
    device_type: String,
    uuid: String,
    zone: Trust,
}

impl PropMapConvert for ConnectionSettings {
    fn from_propmap(map: PropMap) -> Self {
        println!("settings debug");
        for (key, val) in map.iter() {
            dbg!(key);
            dbg!(val);
        }
        let autoconnect = prop_cast(&map, "autoconnect");
        let autoconnect_priority = prop_cast(&map, "autoconnect-priority");
        let uuid: String;
        let name: String;
        let device_type: String;
        let metered = prop_cast(&map, "metered");
        let zone: Trust;
        let zone_opt: Option<&String> = prop_cast(&map, "trust");
        if zone_opt.is_none() {
            zone = Trust::from_str("").ok().unwrap();
        } else {
            zone = Trust::from_str(zone_opt.unwrap().as_str()).ok().unwrap();
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

    fn to_propmap(&self, map: &mut PropMap) {
        map.insert("autoconnect".into(), Variant(Box::new(self.autoconnect)));
        map.insert(
            "autoconnect-priority".into(),
            Variant(Box::new(self.autoconnect_priority)),
        );
        map.insert("metered".into(), Variant(Box::new(self.metered)));
        map.insert("name".into(), Variant(Box::new(self.name.clone())));
        map.insert(
            "device-type".into(),
            Variant(Box::new(self.device_type.clone())),
        );
        map.insert("uuid".into(), Variant(Box::new(self.uuid.clone())));
        map.insert("zone".into(), Variant(Box::new(self.zone.to_i32())));
    }
}
