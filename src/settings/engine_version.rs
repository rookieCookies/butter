use derive_macros::ImmutableData;
use serde::de::Visitor;

#[derive(PartialEq, Eq, Clone, Copy, ImmutableData)]
pub struct EngineVersion {
    major: u8,
    minor: u8,
    patch: u8,
}


impl EngineVersion {
    pub const CURRENT : EngineVersion = EngineVersion {
        major: 0,
        minor: 0,
        patch: 0,
    };


    pub fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self { major, minor, patch }
    }
}


impl core::fmt::Display for EngineVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}


impl core::fmt::Debug for EngineVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}


impl<'se> serde::Serialize for EngineVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}


impl<'de> serde::Deserialize<'de> for EngineVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
            let string = String::deserialize(deserializer)?;
            EngineVersionVisitor{}.visit_str(&string)
    }
}


struct EngineVersionVisitor {}

impl<'de> Visitor<'de> for EngineVersionVisitor {
    type Value = EngineVersion;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a value that is in the format of {u8}.{u8}.{u8}")
    }


    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
    
        let mut version = v.split(".");

        let Some(major) = version.next().map(|x| x.parse::<u8>().ok()).flatten()
        else { return Err(E::custom("invalid format")) };

        let Some(minor) = version.next().map(|x| x.parse::<u8>().ok()).flatten()
        else { return Err(E::custom("invalid format")) };

        let Some(patch) = version.next().map(|x| x.parse::<u8>().ok()).flatten()
        else { return Err(E::custom("invalid format")) };

        Ok(EngineVersion::new(major, minor, patch))
    }
}
