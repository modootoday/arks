//! freedesktop Secret Service backend over the session bus.
//!
//! Items carry the attributes of schema `org.git.Password` (`protocol`,
//! `server`, `port`, `object`, `user`, plus `xdg:schema`), the shape items
//! stored by stock git's libsecret helper are observed to have. Secrets travel
//! in a `plain` session; locked items and collections are unlocked through the
//! service's prompt when one is needed.
//!
//! Primary: <https://specifications.freedesktop.org/secret-service-spec/latest/>
//! (`Service.OpenSession` / `SearchItems` / `Unlock` / `ReadAlias`,
//! `Collection.CreateItem`, `Item.GetSecret` / `Delete`, `Prompt`).

use super::Failure;
use super::dbus::{Conn, DbusError, Value};
use crate::credential::Credential;
use std::time::Duration;

const DEST: &str = "org.freedesktop.secrets";
const SERVICE: &str = "/org/freedesktop/secrets";
const IF_SERVICE: &str = "org.freedesktop.Secret.Service";
const IF_COLLECTION: &str = "org.freedesktop.Secret.Collection";
const IF_ITEM: &str = "org.freedesktop.Secret.Item";
const IF_PROMPT: &str = "org.freedesktop.Secret.Prompt";
const SCHEMA: &str = "org.git.Password";

fn other(e: DbusError) -> Failure {
    Failure::Other(format!("Secret Service: {e}"))
}

/// Attribute pairs for a description; `user` adds the account.
pub(super) fn attributes(desc: &Credential, user: Option<&str>) -> Vec<(String, String)> {
    let mut a = vec![("xdg:schema".to_string(), SCHEMA.to_string())];
    if let Some(p) = desc.protocol.as_deref().filter(|s| !s.is_empty()) {
        a.push(("protocol".into(), p.into()));
    }
    if let Some(h) = desc.host.as_deref().filter(|s| !s.is_empty()) {
        match h.rsplit_once(':') {
            Some((s, p)) if !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()) => {
                a.push(("server".into(), s.into()));
                a.push(("port".into(), p.into()));
            }
            _ => a.push(("server".into(), h.into())),
        }
    }
    if let Some(p) = desc.path.as_deref().filter(|s| !s.is_empty()) {
        a.push(("object".into(), p.into()));
    }
    if let Some(u) = user {
        a.push(("user".into(), u.into()));
    }
    a
}

struct Service {
    conn: Conn,
    session: String,
}

fn path_at(body: &[Value], i: usize) -> Result<String, Failure> {
    body.get(i)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Failure::Other("Secret Service: unexpected reply shape".into()))
}

fn paths_at(body: &[Value], i: usize) -> Vec<String> {
    body.get(i)
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
}

impl Service {
    fn open() -> Result<Service, Failure> {
        let mut conn = Conn::session().map_err(|e| Failure::Unreachable(e.to_string()))?;
        let body = conn
            .call(
                DEST,
                SERVICE,
                IF_SERVICE,
                "OpenSession",
                &[
                    Value::Str("plain".into()),
                    Value::Variant(Box::new(Value::Str(String::new()))),
                ],
            )
            .map_err(|e| match e {
                DbusError::Remote { name, .. }
                    if name.starts_with("org.freedesktop.DBus.Error.") =>
                {
                    Failure::Unreachable(format!(
                        "no Secret Service on the D-Bus session bus ({name})"
                    ))
                }
                e => other(e),
            })?;
        let session = path_at(&body, 1)?;
        Ok(Service { conn, session })
    }

    fn call(
        &mut self,
        path: &str,
        iface: &str,
        member: &str,
        args: &[Value],
    ) -> Result<Vec<Value>, DbusError> {
        self.conn.call(DEST, path, iface, member, args)
    }

    /// Run a prompt (`/` means none) and return its result value.
    fn prompt(&mut self, prompt: &str) -> Result<Option<Value>, Failure> {
        if prompt == "/" {
            return Ok(None);
        }
        let rule =
            format!("type='signal',interface='{IF_PROMPT}',member='Completed',path='{prompt}'");
        self.conn
            .call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "AddMatch",
                &[Value::Str(rule)],
            )
            .map_err(other)?;
        self.call(prompt, IF_PROMPT, "Prompt", &[Value::Str(String::new())])
            .map_err(other)?;
        // A person may have to answer an unlock dialog.
        self.conn.set_timeout(Duration::from_secs(300));
        let body = self
            .conn
            .wait_signal(prompt, IF_PROMPT, "Completed")
            .map_err(other)?;
        self.conn.set_timeout(Duration::from_secs(30));
        if matches!(body.first(), Some(Value::Bool(true))) {
            return Err(Failure::Other(
                "Secret Service: the prompt was dismissed".into(),
            ));
        }
        Ok(match body.into_iter().nth(1) {
            Some(Value::Variant(v)) => Some(*v),
            v => v,
        })
    }

    fn unlock(&mut self, objects: Vec<String>) -> Result<Vec<String>, Failure> {
        let arg = Value::Array("o".into(), objects.into_iter().map(Value::Path).collect());
        let body = self
            .call(SERVICE, IF_SERVICE, "Unlock", &[arg])
            .map_err(other)?;
        let mut done = paths_at(&body, 0);
        if let Some(v) = self.prompt(&path_at(&body, 1)?)? {
            done.extend(paths_at(std::slice::from_ref(&v), 0));
        }
        Ok(done)
    }

    /// Unlocked items matching `attrs`, unlocking locked matches.
    fn search(&mut self, attrs: &[(String, String)]) -> Result<Vec<String>, Failure> {
        let body = self
            .call(
                SERVICE,
                IF_SERVICE,
                "SearchItems",
                &[Value::string_dict(attrs)],
            )
            .map_err(other)?;
        let mut items = paths_at(&body, 0);
        let locked = paths_at(&body, 1);
        if !locked.is_empty() {
            items.extend(self.unlock(locked)?);
        }
        Ok(items)
    }

    fn secret(&mut self, item: &str) -> Result<Vec<u8>, Failure> {
        let body = self
            .call(
                item,
                IF_ITEM,
                "GetSecret",
                &[Value::Path(self.session.clone())],
            )
            .map_err(other)?;
        match body.first() {
            Some(Value::Struct(f)) => f
                .get(2)
                .and_then(Value::as_bytes)
                .ok_or_else(|| Failure::Other("Secret Service: malformed secret".into())),
            _ => Err(Failure::Other("Secret Service: malformed secret".into())),
        }
    }

    fn item_attributes(&mut self, item: &str) -> Result<Vec<(String, String)>, Failure> {
        let body = self
            .call(
                item,
                "org.freedesktop.DBus.Properties",
                "Get",
                &[Value::Str(IF_ITEM.into()), Value::Str("Attributes".into())],
            )
            .map_err(other)?;
        Ok(match body.first() {
            Some(Value::Variant(v)) => v.as_string_dict().unwrap_or_default(),
            _ => Vec::new(),
        })
    }

    fn default_collection(&mut self) -> Result<String, Failure> {
        let body = self
            .call(
                SERVICE,
                IF_SERVICE,
                "ReadAlias",
                &[Value::Str("default".into())],
            )
            .map_err(other)?;
        let c = path_at(&body, 0)?;
        if c == "/" {
            return Err(Failure::Unreachable(
                "the Secret Service has no default collection".into(),
            ));
        }
        Ok(c)
    }

    fn create_item(
        &mut self,
        collection: &str,
        label: &str,
        attrs: &[(String, String)],
        secret: &[u8],
    ) -> Result<Vec<Value>, DbusError> {
        let prop = |k: &str, v: Value| {
            Value::DictEntry(
                Box::new(Value::Str(k.into())),
                Box::new(Value::Variant(Box::new(v))),
            )
        };
        let props = Value::Array(
            "{sv}".into(),
            vec![
                prop(
                    "org.freedesktop.Secret.Item.Label",
                    Value::Str(label.into()),
                ),
                prop(
                    "org.freedesktop.Secret.Item.Attributes",
                    Value::string_dict(attrs),
                ),
            ],
        );
        let secret = Value::Struct(vec![
            Value::Path(self.session.clone()),
            Value::bytes(&[]),
            Value::bytes(secret),
            Value::Str("text/plain; charset=utf8".into()),
        ]);
        self.call(
            collection,
            IF_COLLECTION,
            "CreateItem",
            &[props, secret, Value::Bool(true)],
        )
    }
}

/// `get`: the first matching item's `user` attribute and secret.
pub(super) fn get(desc: &Credential) -> Result<Option<Credential>, Failure> {
    let mut s = Service::open()?;
    let items = s.search(&attributes(desc, desc.username.as_deref()))?;
    let Some(item) = items.first() else {
        return Ok(None);
    };
    let secret = s.secret(item)?;
    let user = s
        .item_attributes(item)?
        .into_iter()
        .find(|(k, _)| k == "user")
        .map(|(_, v)| v);
    Ok(Some(Credential {
        username: user,
        password: Some(String::from_utf8_lossy(&secret).into_owned()),
        ..Default::default()
    }))
}

/// `store`: create or replace the item in the default collection.
pub(super) fn store(desc: &Credential, label: &str, user: &str, pass: &str) -> Result<(), Failure> {
    let mut s = Service::open()?;
    let collection = s.default_collection()?;
    let attrs = attributes(desc, Some(user));
    let body = match s.create_item(&collection, label, &attrs, pass.as_bytes()) {
        Err(DbusError::Remote { name, .. }) if name == "org.freedesktop.Secret.Error.IsLocked" => {
            s.unlock(vec![collection.clone()])?;
            s.create_item(&collection, label, &attrs, pass.as_bytes())
        }
        r => r,
    }
    .map_err(|e| Failure::Other(format!("Secret Service refused the store: {e}")))?;
    s.prompt(&path_at(&body, 1)?)?;
    Ok(())
}

/// `erase`: delete every matching item.
pub(super) fn erase(desc: &Credential) -> Result<(), Failure> {
    let mut s = Service::open()?;
    for item in s.search(&attributes(desc, desc.username.as_deref()))? {
        let body = s.call(&item, IF_ITEM, "Delete", &[]).map_err(other)?;
        s.prompt(&path_at(&body, 0)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_split_the_port_and_add_the_path() {
        let d = Credential::parse_attributes("protocol=https\nhost=h.example:8443\npath=x/y.git\n");
        let a = attributes(&d, Some("u"));
        let want = [
            ("xdg:schema", SCHEMA),
            ("protocol", "https"),
            ("server", "h.example"),
            ("port", "8443"),
            ("object", "x/y.git"),
            ("user", "u"),
        ];
        assert_eq!(a.len(), want.len());
        for ((k, v), (wk, wv)) in a.iter().zip(want) {
            assert_eq!((k.as_str(), v.as_str()), (wk, wv));
        }
        let d = Credential::parse_attributes("protocol=https\nhost=h\n");
        assert_eq!(attributes(&d, None).len(), 3);
    }
}
