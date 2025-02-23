macro_rules! builder {
    ($builder:ident, $target:ident, {
        required: { $($req_field:ident : $req_ty:ty),* $(,)? },
        default: { $($def_field:ident : $def_ty:ty = $def_value:expr),* $(,)? }
    }, $verify:block, $verify_err:ty) => {
        pub struct $builder {
            $($req_field: $req_ty,)*
            $($def_field: $def_ty,)*
        }

        impl $builder {
            #[must_use]
            pub fn new($($req_field: $req_ty),*) -> Self {
                $(let $req_field: $req_ty = $req_field;)*
                $(let $def_field = $def_value;)*
                Self {
                    $($req_field,)*
                    $($def_field,)*
                }
            }

            pub fn build(self) -> Result<$target, $verify_err> {
                $(let $req_field = self.$req_field;)*
                $(let $def_field = self.$def_field;)*
                $verify?;
                Ok($target {
                    $($req_field: $req_field,)*
                    $($def_field: $def_field,)*
                })
            }

            $(
                #[must_use]
                pub fn $req_field(self, value: $req_ty) -> Self {
                    Self {
                        $req_field: value,
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

        impl From<$target> for $builder {
            fn from(value: $target) -> Self {
                Self {
                    $($req_field: value.$req_field),*,
                    $($def_field: value.$def_field),*
                }
            }
        }
    };
}
