use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    machine: Vec<Machine>,
    #[serde(default)]
    connection: Vec<Connection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Connection {
    Net(NetConnection),
    Uart(UartConnection),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetConnection {
    name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UartConnection {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn parse_toml() {
        let text = indoc! {r#"
            [[machine]]
            name = "foo"

            [[machine]]
            name = "bar"

            [[connection]]
            name = "foobar"
            type = "uart"
        "#};

        let cfg: Config = toml::from_str(text).unwrap();
        dbg!(&cfg);
        assert!(cfg.machine.len() == 2);
    }
}
