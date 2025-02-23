macro_rules! builder {
    (
        $(#[$outer:meta])*
        $target:ident {
            provided: { $($provided_field:ident : $provided_ty:ty = $provided_arg_ty:ty => $provided_conv_fn:expr),* $(,)? },
            default: { $($def_field:ident : $def_ty:ty = $def_value:expr),* $(,)? }
        },
        verify: Result<(), $verify_err:ty> = $verify:block
    ) => {
        paste::paste! {
            $(#[$outer])*
            pub struct $target {
                $($provided_field: $provided_ty,)*
                $($def_field: $def_ty,)*
            }

            impl $target {
                pub fn builder($($provided_field: $provided_arg_ty),*) -> [<$target Builder>] {
                    [<$target Builder>]::new($($provided_field),*)
                }

                #[must_use]
                pub fn rebuild(self) -> [<$target Builder>] {
                    self.into()
                }
            }

            pub struct [<$target Builder>] {
                $($provided_field: $provided_ty,)*
                $($def_field: $def_ty,)*
            }

            impl [<$target Builder>] {
                #[must_use]
                pub fn new($($provided_field: $provided_arg_ty),*) -> Self {
                    $(let $provided_field: $provided_ty = $provided_conv_fn;)*
                    // This assignment is done separately to allow computing default values based
                    // on required fields
                    $(let $def_field = $def_value;)*
                    Self {
                        $($provided_field,)*
                        $($def_field,)*
                    }
                }

                pub fn build(self) -> Result<$target, $verify_err> {
                    $(let $provided_field = self.$provided_field;)*
                    $(let $def_field = self.$def_field;)*
                    let verify: Result<(), $verify_err> = $verify;
                    verify?;
                    Ok($target {
                        $($provided_field: $provided_field,)*
                        $($def_field: $def_field,)*
                    })
                }

                $(
                    #[must_use]
                    pub fn $provided_field(self, value: $provided_ty) -> Self {
                        Self {
                            $provided_field: value,
                            ..self
                        }
                    }
                )*

                $(
                    #[must_use]
                    pub fn $def_field(self, value: $def_ty) -> Self {
                        Self {
                            $def_field: value,
                            ..self
                        }
                    }
                )*
            }

            impl From<$target> for [<$target Builder>] {
                fn from(value: $target) -> Self {
                    Self {
                        $($provided_field: value.$provided_field),*,
                        $($def_field: value.$def_field),*
                    }
                }
            }
        }
    };
}
