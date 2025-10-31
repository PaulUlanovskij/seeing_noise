#[macro_export]
macro_rules! get_element_by_id {
    ($id:ident) => {
        crate::get_element_by_id($id)
            .dyn_into()
            .map_err(|_| console_log!("Failed to cast element with id {}", $id))
            .unwrap()
    };
}

#[macro_export]
macro_rules! elements {
    ($noise:ident, $(($name:tt, $type:ty)),* $(,)?) => {
        paste::paste!{
            thread_local! {
                $(
                    static [<$name:snake:upper>]: LazyCell<$type> = LazyCell::new(|| {
                        const NAME: &str = &stringify!([<$noise _ $name>]);
                        get_element_by_id!(NAME)
                    });
                )*
            }
        }
    }
}

#[macro_export]
macro_rules! parse_value {
    ($name:ident, $type:ty) => {
        paste::paste! {
            [<$name:snake:upper>].with(|s|
                s.value().parse::<$type>().map_err(|_|
                    console_log!("Failed to parse value of {} into {}",
                        stringify!([<$name:snake:upper>]),
                        stringify!($type))).unwrap())
        }
    };
}

#[macro_export]
macro_rules! is_checked {
    ($name:ident) => {
        paste::paste! {
            [<$name:snake:upper>].with(|s| s.checked())
        }
    };
}

#[macro_export]
macro_rules! set_text {
    ($name:tt, $text:expr) => {
        paste::paste! {
            [<$name:snake:upper _DISPLAY>].with(|d| d.set_inner_text($text));
        }
    };
}

#[macro_export]
macro_rules! add_callback {
    ($var:ident, $callback:literal, $closure:expr) => {
        paste::paste! {
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            $closure();
        }) as Box<dyn FnMut(_)>);
        [<$var:snake:upper>].with(|var| {
            var.add_event_listener_with_callback($callback, closure.as_ref().unchecked_ref())
                .map_err(|_| {
                    console_log!(
                        "Failed to add event_listener {} to callback {} of element {}",
                        stringify!($closure),
                        $callback,
                        stringify!($var)
                    )
                })
                .unwrap()
        });
        closure.forget();
        }
    };
}

#[macro_export]
macro_rules! radio {
    ($noise:ident, $name:ident, $default:ident, $($option:ident),* $(,)?) => {
        paste::paste! {
            #[derive(Clone, PartialEq)]
            enum [<$name:camel>] {
                [<$default:camel>],
                $(
                    [<$option:camel>],
                )*
            }
            elements!($noise,
                ($default, HtmlInputElement),
                $(
                        ($option, HtmlInputElement),
                )*
            );
            impl [<$name:camel>] {
                pub fn parse() -> Self {
                    if is_checked!($default) { [<$name:camel>]::[<$default:camel>] }
                    $(
                        else if is_checked!($option) { [<$name:camel>]::[<$option:camel>] }
                    )*
                    else { unreachable!("Somehow radio was set to none?") }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! checkbox {
    ($noise:ident, $name:ident) => {
        paste::paste! {
            #[derive(Clone)]
            struct [<$name:camel>] (bool);

            elements!($noise, ($name, HtmlInputElement));

            impl [<$name:camel>] {
                pub fn parse() -> Self {
                    Self(is_checked!($name))
                }
                pub fn value(&self) -> bool {
                    self.0
                }
            }
        }
    };
}

#[macro_export]
macro_rules! slider {
    ($noise:ident, $name:ident, $type:ty) => {
        paste::paste! {
            #[derive(Clone)]
            struct [<$name:camel>] ($type);

            elements!($noise,
                ($name, HtmlInputElement),
                ([<$name _display>], HtmlElement),
                ([<$name _control>], HtmlElement)
            );

            impl [<$name:camel>] {
                pub fn parse() -> Self {
                    Self(parse_value!($name, $type))
                }
                pub fn value(&self) -> $type {
                    self.0
                }
            }
        }
    };
}

#[macro_export]
macro_rules! set_hidden {
    ($name:ident, $is_hidden:ident) => {
        paste::paste! {
            [<$name:snake:upper>].with(|e| e.set_hidden($is_hidden));
        }
    };
}

#[macro_export]
macro_rules! define_noise {
    ($noise:ident,
        sliders:[$(($slider_name:ident, $slider_type:ty, $slider_default:literal)),*] ;
        radios:[$(($radio_name:ident, $radio_default:ident, $($radio_option:ident),* $(,)?)),*] ;
        checkboxes:[$($checkbox_name:ident),*] $(;)?
    ) => {
        paste::paste! {
            $(slider!($noise, $slider_name, $slider_type);)*
            $(radio!($noise, $radio_name, $radio_default, $($radio_option,)*);)*
            $(checkbox!($noise, $checkbox_name);)*

            elements!($noise, (noise, HtmlElement));
            #[derive(Clone)]
            struct [<$noise:camel NoiseSettings>] {
                $(
                    pub $slider_name: [<$slider_name:camel>],
                )*
                $(
                    pub $radio_name: [<$radio_name:camel>],
                )*
                $(
                    pub $checkbox_name: [<$checkbox_name:camel>],
                )*
            }

            impl [<$noise:camel NoiseSettings>] {
                pub fn parse() -> Self {
                    Self {
                        $(
                            $slider_name: [<$slider_name:camel>]::parse(),
                        )*
                        $(
                            $radio_name: [<$radio_name:camel>]::parse(),
                        )*
                        $(
                            $checkbox_name: [<$checkbox_name:camel>]::parse(),
                        )*
                    }
                }
            }

            pub struct [<$noise:camel Noise>];
            impl Noise for [<$noise:camel Noise>] {
                fn setup() {
                    $( add_callback!($slider_name, "input", [<$noise:camel Noise>]::update); )*
                    $(
                        add_callback!($radio_default, "input", [<$noise:camel Noise>]::update);
                        $( add_callback!($radio_option, "input", [<$noise:camel Noise>]::update); )*
                    )*
                    $( add_callback!($checkbox_name, "input", [<$noise:camel Noise>]::update); )*
                    
                    [<$noise:camel Noise>]::on_setup();
                    Self::deselect();
                }

                fn update() {
                    [<$noise:camel Noise>]::on_update();
                    let settings = [<$noise:camel NoiseSettings>]::parse();
                    $(
                        set_text!($slider_name, &settings.$slider_name.value().to_string());
                    )*

                    [<$noise:camel Noise>]::generate_and_draw(settings);
                }

                fn select() {
                    set_hidden!(noise, false);
                    Self::update();
                }
                fn deselect() {
                    set_hidden!(noise, true);
                    Self::reset();
                }

                fn reset() {
                    $( [<$slider_name:snake:upper>].with(|v| v.set_value_as_number($slider_default)); )*
                    $( [<$radio_default:snake:upper>].with(|v| v.set_checked(true)); )*
                    $( [<$checkbox_name:snake:upper>].with(|v| v.set_checked(false)); )*
                }
            }
        }
    }
}
