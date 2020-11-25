

#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profile-with-puffin")]
        puffin::profile_scope!($name);

        #[cfg(feature = "profile-with-optick")]
        optick::event!($name);

        #[cfg(feature = "profile-with-tracing")]
        tracing::span!(tracing::Level::INFO, $name).enter();
    };

    ($name:expr, $data:expr) => {
        #[cfg(feature = "profile-with-puffin")]
        puffin::profile_scope_data!($data);

        #[cfg(feature = "profile-with-optick")]
        optick::event!($name);
        #[cfg(feature = "profile-with-optick")]
        optick::tag!($data);

        #[cfg(feature = "profile-with-tracing")]
        tracing::span!(tracing::Level::INFO, $name, tag = $data).enter();
    };
}
