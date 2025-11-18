#[macro_export]
macro_rules! get_element_by_id {
    ($id:ident) => {
        $crate::get_element_by_id($id)
            .dyn_into()
            .map_err(|_| console_log!("Failed to cast element with id {}", $id))
            .unwrap()
    };
}

#[macro_export]
macro_rules! elements {
    ($(($name:tt, $type:ty)),* $(,)?) => {
        paste::paste!{
            thread_local! {
                $(
                    static [<$name:snake:upper>]: LazyCell<$type> = LazyCell::new(|| {
                        const NAME: &str = &stringify!($name);
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
macro_rules! set_min {
    ($name:tt, $value:expr) => {
        paste::paste! {
            [<$name:snake:upper>].with(|d| d.set_min(format!("{}", $value).as_str()));
        }
    };
}
#[macro_export]
macro_rules! set_max {
    ($name:tt, $value:expr) => {
        paste::paste! {
            [<$name:snake:upper>].with(|d| d.set_max(format!("{}", $value).as_str()));
        }
    };
}


#[macro_export]
macro_rules! define_closure {
    ($name:ident, $body:expr) => {
        paste::paste!{
            thread_local!{
                    static [<$name:snake:upper>]: LazyCell<Closure<dyn Fn()>> = LazyCell::new(|| {
                        Closure::new(||{
                        $body();
                    })
                });
            }
        }    
    };
}

#[macro_export]
macro_rules! add_callback {
    ($var:ident, $callback:literal, $closure:expr) => {
        paste::paste! {
        [<$var:snake:upper>].with(|var| {
            [<$closure:snake:upper>].with(|v| var.add_event_listener_with_callback($callback, v.as_ref().unchecked_ref()))
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
        }
    };
}

#[macro_export]
macro_rules! remove_callback {
    ($var:ident, $callback:literal, $closure:expr) => {
        paste::paste! {
        [<$var:snake:upper>].with(|var| {
            [<$closure:snake:upper>].with(|v| var.remove_event_listener_with_callback($callback, v.as_ref().unchecked_ref()))
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
        }
    };
}

#[macro_export]
macro_rules! radio {
    ($name:ident, ($default:ident, $($default_hide:ident),* $(,)?), $(($option:ident, $($option_hide:ident),* $(,)?)),* $(,)?) => {
        paste::paste! {
            #[derive(Copy, Clone, PartialEq)]
            enum [<$name:camel>] {
                [<$default:camel>],
                $(
                    [<$option:camel>],
                )*
            }
            elements!(
                ($default, HtmlInputElement),
                ([<$default _control>], HtmlElement),
                $(
                        ($option, HtmlInputElement),
                        ([<$option _control>], HtmlElement),
                )*
            );
            thread_local!{
                pub static [<$name:snake:upper _MEMORY>]: std::cell::RefCell<[<$name:camel>]> = std::cell::RefCell::from([<$name:camel>]::[<$default:camel>]);
            }
            impl [<$name:camel>] {
                pub fn parse() -> Self {
                    if is_checked!($default) { [<$name:camel>]::[<$default:camel>] }
                    $(
                        else if is_checked!($option) { [<$name:camel>]::[<$option:camel>] }
                    )*
                    else { unreachable!("Somehow radio was set to none?") }
                }
                pub fn update() {
                    let v = Self::parse();
                    match [<$name:snake:upper _MEMORY>].with(|old| old.clone()).into_inner() {
                        [<$name:camel>]::[<$default:camel>] => {
                            $( set_hidden!([<$default_hide _control>], false); )*
                        }
                        $(
                            [<$name:camel>]::[<$option:camel>] => {
                                $( set_hidden!([<$option_hide _control>], false); )*
                            }
                        )*
                    }

                    match v {
                        [<$name:camel>]::[<$default:camel>] => {
                            $( set_hidden!([<$default_hide _control>], true); )*
                        }
                        $(
                            [<$name:camel>]::[<$option:camel>] => {
                                $( set_hidden!([<$option_hide _control>], true); )*
                            }
                        )*
                    }
                }
                pub fn memorize(value: Self) {
                    [<$name:snake:upper _MEMORY>].with(|v| v.replace(value));
                }
                pub fn reset() {
                    [<$default:snake:upper>].with(|v| v.set_checked(true));
                }
            }
        }
    };
}

#[macro_export]
macro_rules! checkbox {
    ($name:ident) => {
        paste::paste! {
            #[derive(Clone)]
            struct [<$name:camel>] (bool);

            elements!(
                    ($name, HtmlInputElement),
                    ([<$name _control>], HtmlElement)
            );

            impl [<$name:camel>] {
                pub fn parse() -> Self {
                    Self(is_checked!($name))
                }
                pub fn value(&self) -> bool {
                    self.0
                }
                pub fn reset() {
                    [<$name:snake:upper>].with(|v| v.set_checked(false));
                }
            }
        }
    };
}

#[macro_export]
macro_rules! slider {
    ($name:ident, $type:ty, $default:literal) => {
        paste::paste! {
            #[derive(Clone)]
            struct [<$name:camel>] ($type);

            elements!(
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
                pub fn reset() {
                    [<$name:snake:upper>].with(|v| v.set_value_as_number($default));
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
        sliders:[$(($slider_name:ident, $slider_type:ty, $slider_min:literal, $slider_default:literal, $slider_max:literal)),*] ;
        radios:[$(($radio_name:ident, ($radio_default:ident $(, hide:[ $($radio_default_hide:ident),* $(,)? ])?), $(($radio_option:ident $(, hide:[ $($radio_option_hide:ident),* $(,)? ])?)),* $(,)?)),*] ;
        checkboxes:[$($checkbox_name:ident),*] $(;)?
    ) => {
        paste::paste! {
            $(slider!($slider_name, $slider_type, $slider_default);)*
            $(radio!($radio_name, ($radio_default, $($($radio_default_hide,)*)*), $(($radio_option, $($($radio_option_hide,)*)* ),)*);)*
            $(checkbox!($checkbox_name);)*

            elements!(($noise, HtmlElement));

            define_closure!(update_noise, [<$noise:camel Noise>]::update);
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
                    [<$noise:camel Noise>]::on_setup();
                }

                fn update() {
                    $( [<$radio_name:camel>]::update(); )*

                    [<$noise:camel Noise>]::on_update();
                    let settings = [<$noise:camel NoiseSettings>]::parse();
                    
                    $( set_text!($slider_name, &settings.$slider_name.value().to_string()); )*

                    [<$noise:camel Noise>]::generate_and_draw(settings);
                    $( [<$radio_name:camel>]::memorize([<$radio_name:camel>]::parse()); )*
                }

                fn select() {
                    $( 
                        add_callback!($slider_name, "input", update_noise); 
                        set_min!($slider_name, $slider_min); 
                        set_max!($slider_name, $slider_max); 
                        set_hidden!([<$slider_name:camel _control>], false);
                    )*
                    $(
                        add_callback!($radio_default, "input", update_noise);
                        $( add_callback!($radio_option, "input", update_noise); )*
                    )*
                    $( add_callback!($checkbox_name, "input", update_noise); )*

                    Self::reset();
                    $(
                        set_hidden!([<$radio_default:camel _control>], false);
                        $( set_hidden!([<$radio_option:camel _control>], false); )*
                    )*
                    $(
                        set_hidden!([<$checkbox_name:camel _control>], false);
                    )*
                    set_hidden!($noise, false);

                    Self::update();
                }

                fn deselect() {
                    $( remove_callback!($slider_name, "input", update_noise); )*
                    $(
                        remove_callback!($radio_default, "input", update_noise);
                        $( remove_callback!($radio_option, "input", update_noise); )*
                    )*
                    $( remove_callback!($checkbox_name, "input", update_noise); )*

                    $(
                        set_hidden!([<$slider_name:camel _control>], true);
                    )*
                    $(
                        set_hidden!([<$radio_default:camel _control>], true);
                        $( set_hidden!([<$radio_option:camel _control>], true); )*

                    )*
                    $(
                        set_hidden!([<$checkbox_name:camel _control>], true);
                    )*

                    set_hidden!($noise, true);
                }

                fn reset() {
                    $(
                        [<$slider_name:camel>]::reset();
                    )*
                    $(
                        [<$radio_name:camel>]::reset();
                    )*
                    $(
                        [<$checkbox_name:camel>]::reset();
                    )*
                }
            }
        }
    }
}
