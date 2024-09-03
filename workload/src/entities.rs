#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Alias {
    pub name: String,
}

impl From<String> for Alias {
    fn from(value: String) -> Self {
        Alias { name: value }
    }
}

impl From<&str> for Alias {
    fn from(value: &str) -> Self {
        Alias {
            name: value.to_string(),
        }
    }
}

impl Alias {
    pub fn faucet() -> Self {
        Self {
            name: "faucet".into(),
        }
    }

    pub fn nam() -> Self {
        Self { name: "nam".into() }
    }

    pub fn is_faucet(&self) -> bool {
        self.eq(&Self::faucet())
    }
}
