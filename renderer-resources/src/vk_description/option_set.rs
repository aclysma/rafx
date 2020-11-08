// Based on option_set crate.
// - Removed the heck dependency/case changing.
// - Fixed to work with serde
//
// There is probably a better way to do this so that we store a u32 or other integer value directly

use serde::de::{Deserialize, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserializer, Serializer};
use std::fmt::{self, Formatter};
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOrAssign, Deref};

/// Defines an option set type.
#[macro_export]
macro_rules! option_set {
    ($(#[$outer:meta])* pub struct $name:ident: $repr:ty {
        $($(#[$inner:ident $($args:tt)*])* const $variant:ident = $value:expr;)*
    }) => {
        bitflags! {
            $(#[$outer])*
            #[derive(Default)]
            #[allow(non_upper_case_globals)]
            pub struct $name: $repr {
                $(
                    $(#[$inner $($args)*])*
                    #[allow(non_upper_case_globals)]
                    const $variant = $value;
                )*
            }
        }

        impl $crate::option_set::OptionSet for $name {
            const VARIANTS: &'static [$name] = &[$($name::$variant,)*];
            const NAMES: &'static [&'static str] = &[$(stringify!($variant),)*];
        }

        impl ::serde::ser::Serialize for $name {
            fn serialize<S: ::serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                $crate::option_set::serialize(self, serializer)
            }
        }

        impl<'de> ::serde::de::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                $crate::option_set::deserialize(deserializer)
            }
        }
    };
    ($(#[$outer:meta])* struct $name:ident: $repr:ty {
        $($(#[$inner:ident $($args:tt)*])* const $variant:ident = $value:expr;)*
    }) => {
        bitflags! {
            $(#[$outer])*
            #[derive(Default)]
            #[allow(non_upper_case_globals)]
            struct $name: $repr {
                $(
                    $(#[$inner $($args)*])*
                    #[allow(non_upper_case_globals)]
                    const $variant = $value;
                )*
            }
        }

        impl $crate::OptionSet for $name {
            const VARIANTS: &'static [$name] = &[$($name::$variant,)*];
            const NAMES: &'static [&'static str] = &[$(stringify!($variant),)*];
        }

        impl ::serde::ser::Serialize for $name {
            fn serialize<S: ::serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                $crate::serialize(self, serializer)
            }
        }

        impl<'de> ::serde::de::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                $crate::deserialize(deserializer)
            }
        }
    };
}

/// Trait for bit flags that forwards to std traits for useful bit operators.
pub trait OptionSet: Copy + Default + Eq + BitAnd<Output = Self> + BitOrAssign + 'static {
    /// The basis flags (in the algebraic sense): one for each independent option.
    const VARIANTS: &'static [Self];
    /// The corresponding names. `VARIANTS.len() == NAMES.len()` must always hold.
    const NAMES: &'static [&'static str];
}

/// Serialize an OptionSet's set bits as a sequence of strings.
pub fn serialize<T, S>(
    options: &T,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T: OptionSet,
    S: Serializer,
{
    let mut seq = if !serializer.is_human_readable() {
        let mut count = 0;
        for &variant in T::VARIANTS {
            if *options & variant == variant {
                count += 1;
            }
        }
        let mut s = serializer.serialize_seq(Some(count + 1))?;
        s.serialize_element(&(count as u64))?;
        s
    } else {
        serializer.serialize_seq(None)?
    };

    for (&variant, &name) in T::VARIANTS.iter().zip(T::NAMES) {
        if *options & variant == variant {
            seq.serialize_element(&name)?;
        }
    }

    seq.end()
}

/// Deserialize set bits from a sequence of name strings.
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: OptionSet,
    D: Deserializer<'de>,
{
    let is_human_readable = deserializer.is_human_readable();
    deserializer.deserialize_seq(OptionSetVisitor(is_human_readable, PhantomData))
}

/// This visitor tries to associate bitflag values with their name.
#[derive(Debug, Clone, Copy)]
struct OptionSetVisitor<T: OptionSet>(bool, PhantomData<T>);

impl<'de, T: OptionSet> Visitor<'de> for OptionSetVisitor<T> {
    type Value = T;

    fn expecting(
        &self,
        f: &mut Formatter,
    ) -> fmt::Result {
        f.write_str("set of option strings")
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        seq: A,
    ) -> Result<Self::Value, A::Error> {
        extract_bits(seq, T::NAMES, self.0)
    }
}

/// Actually performs the sequence processing and flag extraction.
fn extract_bits<'de, A, T, S>(
    mut seq: A,
    names: &[S],
    is_human_readable: bool,
) -> Result<T, A::Error>
where
    A: SeqAccess<'de>,
    T: OptionSet,
    S: Deref<Target = str>,
{
    use serde::de::Error;

    let mut flags = T::default();

    let max_elements = if !is_human_readable {
        // For non-human-readable formats, the first element will contain the number of elements
        seq.next_element::<usize>()?.unwrap()
    } else {
        usize::MAX
    };

    let mut count = 0;
    loop {
        if count >= max_elements {
            break;
        }

        let elem = seq.next_element::<Str<'de>>()?;
        if elem.is_none() {
            break;
        }

        let elem = elem.unwrap();
        let mut iter = T::VARIANTS.iter().zip(names);

        match iter.find(|&(_, name)| **name == *elem) {
            Some((&flag, _)) => flags |= flag,
            None => return Err(A::Error::unknown_variant(&elem, T::NAMES)),
        }

        count += 1;
    }

    Ok(flags)
}

/// Equivalent of `Cow<'a, str>` except that this
/// type can deserialize from a borrowed string.
#[derive(Debug)]
enum Str<'a> {
    /// Owned.
    String(String),
    /// Borrowed.
    Str(&'a str),
}

impl<'a> Deref for Str<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match *self {
            Str::Str(s) => s,
            Str::String(ref s) => s,
        }
    }
}

/// Visitor for deserializing a `Str<'a>`.
struct StrVisitor;

impl<'a> Visitor<'a> for StrVisitor {
    type Value = Str<'a>;

    fn expecting(
        &self,
        formatter: &mut Formatter,
    ) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(
        self,
        v: &str,
    ) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E>(
        self,
        v: String,
    ) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Str::String(v))
    }

    fn visit_borrowed_str<E>(
        self,
        v: &'a str,
    ) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Str::Str(v))
    }
}

impl<'de> Deserialize<'de> for Str<'de> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StrVisitor)
    }
}
