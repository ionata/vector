use serde::Serializer;
use std::fmt;
use vector_config::{
    schema::{
        apply_metadata, generate_const_string_schema, generate_number_schema,
        generate_one_of_schema,
    },
    schemars::{gen::SchemaGenerator, schema::SchemaObject},
    Configurable, GenerateError, Metadata,
};
use vector_config_common::attributes::CustomAttribute;

use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize,
};

/// Configuration for outbound request concurrency.
#[derive(Clone, Copy, Debug, Derivative, Eq, PartialEq)]
pub enum Concurrency {
    /// A fixed concurrency of 1.
    ///
    /// Only one request can be outstanding at any given time.
    None,

    /// Concurrency will be managed by Vector's [Adaptive Request Concurrency][arc] feature.
    ///
    /// [arc]: https://vector.dev/docs/about/under-the-hood/networking/arc/
    Adaptive,

    /// A fixed amount of concurrency will be allowed.
    Fixed(usize),
}

impl Serialize for Concurrency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self {
            Concurrency::None => serializer.serialize_str("none"),
            Concurrency::Adaptive => serializer.serialize_str("adaptive"),
            Concurrency::Fixed(i) => serializer.serialize_u64(*i as u64),
        }
    }
}

impl Default for Concurrency {
    fn default() -> Self {
        Self::None
    }
}

impl Concurrency {
    pub const fn if_none(self, other: Self) -> Self {
        match self {
            Self::None => other,
            _ => self,
        }
    }

    pub const fn parse_concurrency(&self, default: Self) -> Option<usize> {
        match self.if_none(default) {
            Concurrency::None | Concurrency::Adaptive => None,
            Concurrency::Fixed(limit) => Some(limit),
        }
    }
}

pub const fn concurrency_is_none(concurrency: &Concurrency) -> bool {
    matches!(concurrency, Concurrency::None)
}

impl<'de> Deserialize<'de> for Concurrency {
    // Deserialize either a positive integer or the string "adaptive"
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UsizeOrAdaptive;

        impl<'de> Visitor<'de> for UsizeOrAdaptive {
            type Value = Concurrency;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(r#"positive integer, "adaptive", or "none" "#)
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Concurrency, E> {
                if value == "adaptive" {
                    Ok(Concurrency::Adaptive)
                } else if value == "none" {
                    Ok(Concurrency::None)
                } else {
                    Err(de::Error::unknown_variant(value, &["adaptive"]))
                }
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Concurrency, E> {
                if value > 0 {
                    Ok(Concurrency::Fixed(value as usize))
                } else {
                    Err(de::Error::invalid_value(
                        Unexpected::Signed(value),
                        &"positive integer",
                    ))
                }
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Concurrency, E> {
                if value > 0 {
                    Ok(Concurrency::Fixed(value as usize))
                } else {
                    Err(de::Error::invalid_value(
                        Unexpected::Unsigned(value),
                        &"positive integer",
                    ))
                }
            }
        }

        deserializer.deserialize_any(UsizeOrAdaptive)
    }
}

// TODO: Consider an approach for generating schema of "string or number" structure used by this type.
impl Configurable for Concurrency {
    fn referenceable_name() -> Option<&'static str> {
        Some(std::any::type_name::<Self>())
    }

    fn metadata() -> Metadata<Self> {
        let mut metadata = Metadata::default();
        metadata.set_description("Configuration for outbound request concurrency.");
        metadata.add_custom_attribute(CustomAttribute::kv("docs::enum_tagging", "external"));
        metadata
    }

    fn generate_schema(_: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut none_schema = generate_const_string_schema("none".to_string());
        let mut none_metadata = Metadata::<()>::with_title("A fixed concurrency of 1.");
        none_metadata.set_description("Only one request can be outstanding at any given time.");
        none_metadata.add_custom_attribute(CustomAttribute::KeyValue {
            key: "logical_name".to_string(),
            value: "None".to_string(),
        });
        apply_metadata(&mut none_schema, none_metadata);

        let mut adaptive_schema = generate_const_string_schema("adaptive".to_string());
        let mut adaptive_metadata = Metadata::<()>::with_title(
            "Concurrency will be managed by Vector's [Adaptive Request Concurrency][arc] feature.",
        );
        adaptive_metadata
            .set_description("[arc]: https://vector.dev/docs/about/under-the-hood/networking/arc/");
        adaptive_metadata.add_custom_attribute(CustomAttribute::KeyValue {
            key: "logical_name".to_string(),
            value: "Adaptive".to_string(),
        });
        apply_metadata(&mut adaptive_schema, adaptive_metadata);

        let mut fixed_schema = generate_number_schema::<usize>();
        let mut fixed_metadata =
            Metadata::<()>::with_description("A fixed amount of concurrency will be allowed.");
        fixed_metadata.set_transparent();
        fixed_metadata.add_custom_attribute(CustomAttribute::KeyValue {
            key: "logical_name".to_string(),
            value: "Fixed".to_string(),
        });
        apply_metadata(&mut fixed_schema, fixed_metadata);

        Ok(generate_one_of_schema(&[
            none_schema,
            adaptive_schema,
            fixed_schema,
        ]))
    }
}

#[test]
fn is_serialization_reversible() {
    let variants = [
        Concurrency::None,
        Concurrency::Adaptive,
        Concurrency::Fixed(8),
    ];

    for v in variants {
        let value = serde_json::to_value(v).unwrap();
        let deserialized = serde_json::from_value::<Concurrency>(value)
            .expect("Failed to deserialize a previously serialized Concurrency value");

        assert_eq!(v, deserialized)
    }
}
