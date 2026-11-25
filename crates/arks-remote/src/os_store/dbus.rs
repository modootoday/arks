//! A minimal D-Bus client: session-bus connection, `EXTERNAL` authentication,
//! method calls and signal waits, with the wire marshaling they need.
//!
//! Only what the Secret Service client uses is implemented: little-endian
//! messages out, either byte order in, and the basic, array, struct,
//! dict-entry and variant types.
//!
//! Primary: <https://dbus.freedesktop.org/doc/dbus-specification.html>
//! (message format, marshaling, authentication protocol, server addresses).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// A marshaled D-Bus value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Byte(u8),
    Bool(bool),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Double(f64),
    Str(String),
    Path(String),
    Sig(String),
    Variant(Box<Value>),
    /// Element signature and elements.
    Array(String, Vec<Value>),
    Struct(Vec<Value>),
    DictEntry(Box<Value>, Box<Value>),
}

impl Value {
    /// The value's type signature.
    pub fn signature(&self) -> String {
        match self {
            Value::Byte(_) => "y".into(),
            Value::Bool(_) => "b".into(),
            Value::I16(_) => "n".into(),
            Value::U16(_) => "q".into(),
            Value::I32(_) => "i".into(),
            Value::U32(_) => "u".into(),
            Value::I64(_) => "x".into(),
            Value::U64(_) => "t".into(),
            Value::Double(_) => "d".into(),
            Value::Str(_) => "s".into(),
            Value::Path(_) => "o".into(),
            Value::Sig(_) => "g".into(),
            Value::Variant(_) => "v".into(),
            Value::Array(e, _) => format!("a{e}"),
            Value::Struct(f) => format!("({})", f.iter().map(Value::signature).collect::<String>()),
            Value::DictEntry(k, v) => format!("{{{}{}}}", k.signature(), v.signature()),
        }
    }

    /// The string inside a `Str`, `Path` or `Sig`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) | Value::Path(s) | Value::Sig(s) => Some(s),
            _ => None,
        }
    }

    /// Array elements.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(_, v) => Some(v),
            _ => None,
        }
    }

    /// A byte array as bytes.
    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        self.as_array()?
            .iter()
            .map(|b| match b {
                Value::Byte(x) => Some(*x),
                _ => None,
            })
            .collect()
    }

    /// A string-to-string dictionary (`a{ss}`) as pairs.
    pub fn as_string_dict(&self) -> Option<Vec<(String, String)>> {
        self.as_array()?
            .iter()
            .map(|e| match e {
                Value::DictEntry(k, v) => Some((k.as_str()?.to_string(), v.as_str()?.to_string())),
                _ => None,
            })
            .collect()
    }

    /// A byte array value (`ay`).
    pub fn bytes(b: &[u8]) -> Value {
        Value::Array("y".into(), b.iter().copied().map(Value::Byte).collect())
    }

    /// A string-to-string dictionary value (`a{ss}`).
    pub fn string_dict(pairs: &[(String, String)]) -> Value {
        Value::Array(
            "{ss}".into(),
            pairs
                .iter()
                .map(|(k, v)| {
                    Value::DictEntry(
                        Box::new(Value::Str(k.clone())),
                        Box::new(Value::Str(v.clone())),
                    )
                })
                .collect(),
        )
    }
}

/// Why a bus operation failed.
#[derive(Debug)]
pub enum DbusError {
    /// No bus could be reached or authenticated with.
    Unreachable(String),
    /// The peer replied with an error message.
    Remote { name: String, message: String },
    /// Malformed traffic or an I/O failure on an open connection.
    Protocol(String),
}

impl std::fmt::Display for DbusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbusError::Unreachable(m) | DbusError::Protocol(m) => f.write_str(m),
            DbusError::Remote { name, message } if message.is_empty() => f.write_str(name),
            DbusError::Remote { name, message } => write!(f, "{name}: {message}"),
        }
    }
}

fn proto(m: impl Into<String>) -> DbusError {
    DbusError::Protocol(m.into())
}

// ---------- marshaling ----------

fn alignment(sig: u8) -> usize {
    match sig {
        b'n' | b'q' => 2,
        b'b' | b'i' | b'u' | b'h' | b's' | b'o' | b'a' => 4,
        b'x' | b't' | b'd' | b'(' | b'{' => 8,
        _ => 1,
    }
}

/// Split the first complete type off a signature.
fn split_type(sig: &str) -> Result<(&str, &str), DbusError> {
    let b = sig.as_bytes();
    let mut i = 0;
    while b.get(i) == Some(&b'a') {
        i += 1;
    }
    match b.get(i) {
        None => Err(proto(format!("truncated signature '{sig}'"))),
        Some(&open @ (b'(' | b'{')) => {
            let close = if open == b'(' { b')' } else { b'}' };
            let mut depth = 0usize;
            for (j, &c) in b.iter().enumerate().skip(i) {
                if c == b'(' || c == b'{' {
                    depth += 1;
                } else if c == b')' || c == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        if c != close {
                            return Err(proto(format!("bad signature '{sig}'")));
                        }
                        return Ok((&sig[..=j], &sig[j + 1..]));
                    }
                }
            }
            Err(proto(format!("unbalanced signature '{sig}'")))
        }
        Some(_) => Ok((&sig[..=i], &sig[i + 1..])),
    }
}

/// Marshals little-endian values; offsets are relative to the message start.
struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn pad(&mut self, n: usize) {
        while self.buf.len() % n != 0 {
            self.buf.push(0);
        }
    }

    fn u32(&mut self, v: u32) {
        self.pad(4);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn string(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);
    }

    fn value(&mut self, v: &Value) {
        match v {
            Value::Byte(b) => self.buf.push(*b),
            Value::Bool(b) => self.u32(u32::from(*b)),
            Value::I16(x) => {
                self.pad(2);
                self.buf.extend_from_slice(&x.to_le_bytes());
            }
            Value::U16(x) => {
                self.pad(2);
                self.buf.extend_from_slice(&x.to_le_bytes());
            }
            Value::I32(x) => {
                self.pad(4);
                self.buf.extend_from_slice(&x.to_le_bytes());
            }
            Value::U32(x) => self.u32(*x),
            Value::I64(x) => {
                self.pad(8);
                self.buf.extend_from_slice(&x.to_le_bytes());
            }
            Value::U64(x) => {
                self.pad(8);
                self.buf.extend_from_slice(&x.to_le_bytes());
            }
            Value::Double(x) => {
                self.pad(8);
                self.buf.extend_from_slice(&x.to_le_bytes());
            }
            Value::Str(s) | Value::Path(s) => self.string(s),
            Value::Sig(s) => {
                self.buf.push(s.len() as u8);
                self.buf.extend_from_slice(s.as_bytes());
                self.buf.push(0);
            }
            Value::Variant(inner) => {
                self.value(&Value::Sig(inner.signature()));
                self.value(inner);
            }
            Value::Array(elem, items) => {
                self.pad(4);
                let len_at = self.buf.len();
                self.buf.extend_from_slice(&[0; 4]);
                self.pad(alignment(elem.as_bytes().first().copied().unwrap_or(b'y')));
                let start = self.buf.len();
                for i in items {
                    self.value(i);
                }
                let len = (self.buf.len() - start) as u32;
                self.buf[len_at..len_at + 4].copy_from_slice(&len.to_le_bytes());
            }
            Value::Struct(fields) => {
                self.pad(8);
                for f in fields {
                    self.value(f);
                }
            }
            Value::DictEntry(k, v) => {
                self.pad(8);
                self.value(k);
                self.value(v);
            }
        }
    }
}

/// Unmarshals values from a whole message buffer.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
    big: bool,
}

impl Reader<'_> {
    fn align(&mut self, n: usize) -> Result<(), DbusError> {
        let p = self.pos.next_multiple_of(n);
        if p > self.buf.len() {
            return Err(proto("truncated message"));
        }
        self.pos = p;
        Ok(())
    }

    fn take(&mut self, n: usize) -> Result<&[u8], DbusError> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len());
        let end = end.ok_or_else(|| proto("truncated message"))?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], DbusError> {
        self.align(N)?;
        let mut a = [0u8; N];
        a.copy_from_slice(self.take(N)?);
        if self.big {
            a.reverse();
        }
        Ok(a)
    }

    fn u32(&mut self) -> Result<u32, DbusError> {
        Ok(u32::from_le_bytes(self.fixed::<4>()?))
    }

    fn text(&mut self, len: usize) -> Result<String, DbusError> {
        let s = self.take(len)?.to_vec();
        self.take(1)?;
        String::from_utf8(s).map_err(|_| proto("string is not UTF-8"))
    }

    fn value(&mut self, ty: &str) -> Result<Value, DbusError> {
        let c = *ty.as_bytes().first().ok_or_else(|| proto("empty type"))?;
        Ok(match c {
            b'y' => Value::Byte(self.take(1)?[0]),
            b'b' => Value::Bool(self.u32()? != 0),
            b'n' => Value::I16(i16::from_le_bytes(self.fixed::<2>()?)),
            b'q' => Value::U16(u16::from_le_bytes(self.fixed::<2>()?)),
            b'i' => Value::I32(i32::from_le_bytes(self.fixed::<4>()?)),
            b'u' | b'h' => Value::U32(self.u32()?),
            b'x' => Value::I64(i64::from_le_bytes(self.fixed::<8>()?)),
            b't' => Value::U64(u64::from_le_bytes(self.fixed::<8>()?)),
            b'd' => Value::Double(f64::from_le_bytes(self.fixed::<8>()?)),
            b's' | b'o' => {
                let n = self.u32()? as usize;
                let s = self.text(n)?;
                if c == b's' {
                    Value::Str(s)
                } else {
                    Value::Path(s)
                }
            }
            b'g' => {
                let n = self.take(1)?[0] as usize;
                Value::Sig(self.text(n)?)
            }
            b'v' => {
                let n = self.take(1)?[0] as usize;
                let sig = self.text(n)?;
                let (one, rest) = split_type(&sig)?;
                if !rest.is_empty() {
                    return Err(proto("variant holds more than one type"));
                }
                Value::Variant(Box::new(self.value(one)?))
            }
            b'a' => {
                let elem = &ty[1..];
                let len = self.u32()? as usize;
                self.align(alignment(elem.as_bytes()[0]))?;
                let end = self.pos + len;
                if end > self.buf.len() {
                    return Err(proto("array runs past the message"));
                }
                let mut items = Vec::new();
                while self.pos < end {
                    items.push(self.value(elem)?);
                }
                Value::Array(elem.to_string(), items)
            }
            b'(' | b'{' => {
                self.align(8)?;
                let mut inner = &ty[1..ty.len() - 1];
                let mut fields = Vec::new();
                while !inner.is_empty() {
                    let (one, rest) = split_type(inner)?;
                    fields.push(self.value(one)?);
                    inner = rest;
                }
                if c == b'{' {
                    let mut it = fields.into_iter();
                    match (it.next(), it.next(), it.next()) {
                        (Some(k), Some(v), None) => Value::DictEntry(Box::new(k), Box::new(v)),
                        _ => return Err(proto("dict entry needs two fields")),
                    }
                } else {
                    Value::Struct(fields)
                }
            }
            other => return Err(proto(format!("unsupported type '{}'", other as char))),
        })
    }

    fn values(&mut self, mut sig: &str) -> Result<Vec<Value>, DbusError> {
        let mut out = Vec::new();
        while !sig.is_empty() {
            let (one, rest) = split_type(sig)?;
            out.push(self.value(one)?);
            sig = rest;
        }
        Ok(out)
    }
}

const METHOD_CALL: u8 = 1;
const METHOD_RETURN: u8 = 2;
const ERROR: u8 = 3;
const SIGNAL: u8 = 4;

/// A received message.
#[derive(Debug, Default)]
pub struct Message {
    pub kind: u8,
    pub reply_serial: Option<u32>,
    pub path: Option<String>,
    pub interface: Option<String>,
    pub member: Option<String>,
    pub error_name: Option<String>,
    pub body: Vec<Value>,
}

/// Encode a method call.
fn encode_call(
    serial: u32,
    dest: &str,
    path: &str,
    iface: &str,
    member: &str,
    args: &[Value],
) -> Vec<u8> {
    let mut body = Writer { buf: Vec::new() };
    for a in args {
        body.value(a);
    }
    let sig: String = args.iter().map(Value::signature).collect();
    let field =
        |code: u8, v: Value| Value::Struct(vec![Value::Byte(code), Value::Variant(Box::new(v))]);
    let mut fields = vec![
        field(1, Value::Path(path.into())),
        field(2, Value::Str(iface.into())),
        field(3, Value::Str(member.into())),
        field(6, Value::Str(dest.into())),
    ];
    if !sig.is_empty() {
        fields.push(field(8, Value::Sig(sig)));
    }
    let mut w = Writer {
        buf: vec![b'l', METHOD_CALL, 0, 1],
    };
    w.u32(body.buf.len() as u32);
    w.u32(serial);
    w.value(&Value::Array("(yv)".into(), fields));
    w.pad(8);
    w.buf.extend_from_slice(&body.buf);
    w.buf
}

/// Decode one whole message.
fn decode(buf: &[u8]) -> Result<Message, DbusError> {
    let big = match buf.first() {
        Some(b'l') => false,
        Some(b'B') => true,
        _ => return Err(proto("bad byte-order mark")),
    };
    let mut r = Reader { buf, pos: 12, big };
    let fields = r.value("a(yv)")?;
    r.align(8)?;
    let mut m = Message {
        kind: buf[1],
        ..Default::default()
    };
    let mut sig = String::new();
    for f in fields.as_array().unwrap_or_default() {
        let Value::Struct(kv) = f else { continue };
        let (Some(Value::Byte(code)), Some(Value::Variant(v))) = (kv.first(), kv.get(1)) else {
            continue;
        };
        let s = v.as_str().map(str::to_string);
        match code {
            1 => m.path = s,
            2 => m.interface = s,
            3 => m.member = s,
            4 => m.error_name = s,
            5 => {
                if let Value::U32(n) = **v {
                    m.reply_serial = Some(n);
                }
            }
            8 => sig = s.unwrap_or_default(),
            _ => {}
        }
    }
    m.body = r.values(&sig)?;
    Ok(m)
}

/// The total length of the message whose first 16 bytes are `head`.
fn message_len(head: &[u8; 16]) -> Result<usize, DbusError> {
    let num = |b: [u8; 4]| {
        if head[0] == b'B' {
            u32::from_be_bytes(b)
        } else {
            u32::from_le_bytes(b)
        }
    };
    let body = num([head[4], head[5], head[6], head[7]]) as usize;
    let fields = num([head[12], head[13], head[14], head[15]]) as usize;
    let total = (16 + fields).next_multiple_of(8) + body;
    // The specification caps a message at 128 MiB.
    if total > 128 << 20 {
        return Err(proto("message too large"));
    }
    Ok(total)
}

// ---------- addresses and authentication ----------

fn unescape(v: &str) -> String {
    let b = v.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(Ok(x)) = v.get(i + 1..i + 3).map(|h| u8::from_str_radix(h, 16))
        {
            out.push(x);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Unix-socket candidates (`path=` or `abstract=`) from a bus address string.
pub(crate) fn parse_address(addr: &str) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    for entry in addr.split(';') {
        let Some(params) = entry.strip_prefix("unix:") else {
            continue;
        };
        for kv in params.split(',') {
            if let Some(p) = kv.strip_prefix("path=") {
                out.push((false, unescape(p)));
            } else if let Some(p) = kv.strip_prefix("abstract=") {
                out.push((true, unescape(p)));
            }
        }
    }
    out
}

fn connect_one(abstract_name: bool, name: &str) -> std::io::Result<UnixStream> {
    if !abstract_name {
        return UnixStream::connect(name);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        #[cfg(target_os = "android")]
        use std::os::android::net::SocketAddrExt;
        #[cfg(target_os = "linux")]
        use std::os::linux::net::SocketAddrExt;
        let a = std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes())?;
        UnixStream::connect_addr(&a)
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        Err(std::io::Error::other("abstract sockets are Linux-only"))
    }
}

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

/// An authenticated bus connection.
pub struct Conn {
    stream: UnixStream,
    serial: u32,
    queued: Vec<Message>,
}

impl Conn {
    /// Connect to the session bus (`DBUS_SESSION_BUS_ADDRESS`, else
    /// `$XDG_RUNTIME_DIR/bus`), authenticate and say `Hello`.
    pub fn session() -> Result<Conn, DbusError> {
        let addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default();
        let mut candidates = parse_address(&addr);
        if addr.is_empty()
            && let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR")
        {
            let p = std::path::Path::new(&dir).join("bus");
            if p.exists() {
                candidates.push((false, p.to_string_lossy().into_owned()));
            }
        }
        if candidates.is_empty() {
            return Err(DbusError::Unreachable(
                "no D-Bus session bus address".into(),
            ));
        }
        let mut last = String::new();
        for (abs, name) in candidates {
            match connect_one(abs, &name) {
                Ok(stream) => return Conn::start(stream),
                Err(e) => last = format!("D-Bus session bus {name}: {e}"),
            }
        }
        Err(DbusError::Unreachable(last))
    }

    fn start(stream: UnixStream) -> Result<Conn, DbusError> {
        let unreachable =
            |e: std::io::Error| DbusError::Unreachable(format!("D-Bus session bus: {e}"));
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(unreachable)?;
        let mut c = Conn {
            stream,
            serial: 0,
            queued: Vec::new(),
        };
        // SAFETY: getuid has no preconditions and cannot fail.
        let uid = unsafe { libc::getuid() };
        let hello = format!("\0AUTH EXTERNAL {}\r\n", hex(&uid.to_string()));
        c.stream.write_all(hello.as_bytes()).map_err(unreachable)?;
        let line = c.read_line().map_err(unreachable)?;
        if !line.starts_with("OK ") {
            return Err(DbusError::Unreachable(format!(
                "D-Bus session bus refused authentication ({})",
                line.trim()
            )));
        }
        c.stream.write_all(b"BEGIN\r\n").map_err(unreachable)?;
        c.call(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "Hello",
            &[],
        )
        .map_err(|e| DbusError::Unreachable(format!("D-Bus session bus: {e}")))?;
        Ok(c)
    }

    fn read_line(&mut self) -> std::io::Result<String> {
        let mut out = Vec::new();
        let mut b = [0u8; 1];
        while !out.ends_with(b"\r\n") {
            if out.len() > 4096 {
                return Err(std::io::Error::other("authentication line too long"));
            }
            self.stream.read_exact(&mut b)?;
            out.push(b[0]);
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// Change how long a read may block.
    pub fn set_timeout(&mut self, d: Duration) {
        let _ = self.stream.set_read_timeout(Some(d));
    }

    fn read_message(&mut self) -> Result<Message, DbusError> {
        let io = |e: std::io::Error| proto(format!("D-Bus read: {e}"));
        let mut head = [0u8; 16];
        self.stream.read_exact(&mut head).map_err(io)?;
        let total = message_len(&head)?;
        let mut buf = head.to_vec();
        buf.resize(total, 0);
        self.stream.read_exact(&mut buf[16..]).map_err(io)?;
        decode(&buf)
    }

    /// Call a method and wait for its reply body.
    pub fn call(
        &mut self,
        dest: &str,
        path: &str,
        iface: &str,
        member: &str,
        args: &[Value],
    ) -> Result<Vec<Value>, DbusError> {
        self.serial += 1;
        let serial = self.serial;
        let msg = encode_call(serial, dest, path, iface, member, args);
        self.stream
            .write_all(&msg)
            .map_err(|e| proto(format!("D-Bus write: {e}")))?;
        loop {
            let m = self.read_message()?;
            match m.kind {
                METHOD_RETURN if m.reply_serial == Some(serial) => return Ok(m.body),
                ERROR if m.reply_serial == Some(serial) => {
                    return Err(DbusError::Remote {
                        name: m.error_name.unwrap_or_default(),
                        message: m
                            .body
                            .first()
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    });
                }
                SIGNAL => self.queued.push(m),
                _ => {}
            }
        }
    }

    /// Wait for a signal `iface.member` emitted from object `path`.
    pub fn wait_signal(
        &mut self,
        path: &str,
        iface: &str,
        member: &str,
    ) -> Result<Vec<Value>, DbusError> {
        let hit = |m: &Message| {
            m.path.as_deref() == Some(path)
                && m.interface.as_deref() == Some(iface)
                && m.member.as_deref() == Some(member)
        };
        if let Some(i) = self.queued.iter().position(hit) {
            return Ok(self.queued.remove(i).body);
        }
        loop {
            let m = self.read_message()?;
            if m.kind == SIGNAL && hit(&m) {
                return Ok(m.body);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_split_into_complete_types() {
        assert_eq!(split_type("a{sv}s").unwrap(), ("a{sv}", "s"));
        assert_eq!(split_type("(oayays)").unwrap(), ("(oayays)", ""));
        assert_eq!(split_type("aao").unwrap(), ("aao", ""));
        assert!(split_type("(ss").is_err());
        assert!(split_type("a").is_err());
    }

    #[test]
    fn call_encoding_round_trips_through_decode() {
        let args = vec![
            Value::Str("plain".into()),
            Value::Variant(Box::new(Value::Str(String::new()))),
            Value::string_dict(&[
                ("protocol".into(), "https".into()),
                ("server".into(), "h".into()),
            ]),
            Value::Struct(vec![
                Value::Path("/s/1".into()),
                Value::bytes(&[]),
                Value::bytes(b"pw"),
                Value::Str("text/plain".into()),
            ]),
            Value::Bool(true),
            Value::U64(7),
        ];
        let buf = encode_call(9, "d.e", "/p", "i.f", "M", &args);
        assert_eq!(buf.len() % 8, 0);
        let head: [u8; 16] = buf[..16].try_into().unwrap();
        assert_eq!(message_len(&head).unwrap(), buf.len());
        let m = decode(&buf).unwrap();
        assert_eq!(m.kind, METHOD_CALL);
        assert_eq!(m.path.as_deref(), Some("/p"));
        assert_eq!(m.interface.as_deref(), Some("i.f"));
        assert_eq!(m.member.as_deref(), Some("M"));
        assert_eq!(m.body, args);
        assert_eq!(
            m.body[2].as_string_dict().unwrap(),
            [
                ("protocol".to_string(), "https".to_string()),
                ("server".into(), "h".into())
            ]
        );
        let Value::Struct(s) = &m.body[3] else {
            panic!()
        };
        assert_eq!(s[2].as_bytes().unwrap(), b"pw");
    }

    #[test]
    fn big_endian_reply_decodes() {
        // METHOD_RETURN, reply serial 3, signature "u", body 0x01020304.
        let mut b = vec![b'B', METHOD_RETURN, 0, 1, 0, 0, 0, 4, 0, 0, 0, 5];
        let fields: Vec<u8> = [
            &[5u8, 1, b'u', 0][..],
            // The second field starts at offset 24, already 8-aligned.
            &[0, 0, 0, 3],
            &[8, 1, b'g', 0, 1, b'u', 0],
        ]
        .concat();
        b.extend_from_slice(&(fields.len() as u32).to_be_bytes());
        b.extend_from_slice(&fields);
        while b.len() % 8 != 0 {
            b.push(0);
        }
        b.extend_from_slice(&[1, 2, 3, 4]);
        let head: [u8; 16] = b[..16].try_into().unwrap();
        assert_eq!(message_len(&head).unwrap(), b.len());
        let m = decode(&b).unwrap();
        assert_eq!(m.reply_serial, Some(3));
        assert_eq!(m.body, [Value::U32(0x0102_0304)]);
    }

    #[test]
    fn addresses_parse_paths_and_abstract_names() {
        assert_eq!(
            parse_address("unix:path=/run/user/1000/bus"),
            [(false, "/run/user/1000/bus".to_string())]
        );
        assert_eq!(
            parse_address("unix:abstract=/tmp/dbus-x%2dy,guid=ab;tcp:host=h"),
            [(true, "/tmp/dbus-x-y".to_string())]
        );
        assert!(parse_address("tcp:host=h,port=1").is_empty());
    }
}
