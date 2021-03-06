/// Generates the proper call to the `ParseYaml` trait
macro_rules! gen_struct_value {
    // Default with no display name
    ($parent:expr, self, default) => {{
        gen_struct_value!($parent, self, Default::default())
    }};

    // Default value
    ($parent:expr, $disp_name:expr, default) => {{
        gen_struct_value!($parent, $disp_name, Default::default())
    }};

    // Option
    ($parent:expr, $disp_name:expr, None) => {{
        ParseYaml::from(&$parent[$disp_name])
    }};

    // Option with minimum
    ($parent:expr, $disp_name:expr, [$min:expr, None]) => {{
        let result = gen_struct_value!($parent, $disp_name, None);
        result.map(|v| if v < $min { $min } else { v })
    }};

    // Value with minimum
    ($parent:expr, $disp_name:expr, [$min:expr, $default:expr]) => {{
        let result = gen_struct_value!($parent, $disp_name, $default);
        if result < $min { $min } else { result }
    }};

    // Value with no display name that exits early on failure
    ($parent:expr, self, fail) => {{
        ParseYaml::from(&$parent)?
    }};

    // Value with no display name
    ($parent:expr, self, $default:expr) => {{
        ParseYaml::from(&$parent).unwrap_or($default)
    }};

    // Value that exits early on failure
    ($parent:expr, $disp_name:expr, fail) => {{
        ParseYaml::from(&$parent[$disp_name])?
    }};

    // Array
    ($parent:expr, $disp_name:expr, all) => {{
        ParseYaml::all(&$parent[$disp_name])
    }};

    // Value
    ($parent:expr, $disp_name:expr, $default:expr) => {{
        ParseYaml::from(&$parent[$disp_name]).unwrap_or($default)
    }};
}

macro_rules! get_default {
    (default)                    => (Default::default());
    (fail)                       => (Default::default());
    (all)                        => (Vec::new());
    ([$min:expr, $default:expr]) => ($default);
    ($default:expr)              => ($default);
}

/// Generates a new group of configuration values.
#[macro_export]
macro_rules! create_config_struct {
    ($name:ident, $($field:ident: $field_t:ty => $disp_name:tt => $default:tt,)+) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $field_t,)+
        }

        impl ParseYaml for $name {
            fn from(doc: &Yaml) -> Option<$name> {
                Some($name {
                    $($field: gen_struct_value!(doc, $disp_name, $default),)+
                })
            }
        }

        impl Default for $name {
            fn default() -> $name {
                $name {
                    $($field: get_default!($default),)+
                }
            }
        }
    };
}

/// Generates the name used to read an enum field.
macro_rules! get_enum_field_name {
    ($field:ident, self)            => (stringify!($field));
    ($field:ident, $disp_name:expr) => ($disp_name);
}

/// Generates a new group of configuration values where only one variant can be picked.
/// Supports plain enum variants (with no value required), or ones that require different types for each variant.
#[macro_export]
macro_rules! create_config_enum {
    ($name:ident, $($field:ident($field_t:ty) => $disp_name:tt,)+) => {
        #[derive(Debug)]
        pub enum $name {
            $($field($field_t),)+
        }

        impl ParseYaml for $name {
            fn from(doc: &Yaml) -> Option<$name> {
                let mut elem;

                $(
                elem = &doc[get_enum_field_name!($field, $disp_name)];

                if !elem.is_badvalue() {
                    if let Some(v) = ParseYaml::from(elem) {
                        return Some($name::$field(v));
                    }
                }
                )+

                None
            }
        }

        impl Default for $name {
            fn default() -> $name {
                panic!("unable to get default for {} enum", stringify!($name));
            }
        }
    };

    ($name:ident, $($field:ident => $disp_name:tt,)+) => {
        #[derive(Debug)]
        pub enum $name {
            $($field,)+
        }

        impl ParseYaml for $name {
            fn from(doc: &Yaml) -> Option<$name> {
                let result: Option<String> = ParseYaml::from(&doc);

                result.and_then(|result| {
                    match result.as_str() {
                        $(get_enum_field_name!($field, $disp_name) => Some($name::$field),)+
                        _ => None,
                    }
                })
            }
        }

        impl Default for $name {
            fn default() -> $name {
                panic!("unable to get default for {} enum", stringify!($name));
            }
        }
    };
}
