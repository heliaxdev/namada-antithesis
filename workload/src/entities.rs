use serde::{Deserialize,Deserializer, Serialize, Serializer};

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

impl Serialize for Alias {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(&self.name, serializer)
    }
}


impl<'de> Deserialize<'de> for Alias {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(Alias { name })
    }
}


