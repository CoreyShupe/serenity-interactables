#[macro_export]
macro_rules! execute_event {
    ($ctx:ident { $($option:ident),*$(,)? }) => {
        match $crate::context::IntoInteractionParts::get_cid($ctx.inner()) {
            $(
            $option::REFERENCE => Some($ctx.execute_with::<$option>().await),
            _ => None,
            )*
        }
    }
}

#[macro_export]
macro_rules! define_interactable {
    (@@a) => {
        ctx
    };
    (@@a $value:ident) => {
        $value
    };
    ($($struct_name:ident<$inner_ty:ty$( as $value:ident)?> {
        $($tt:tt)*
    })*) => {$(
        struct $struct_name;
        impl Interactable for $struct_name {
            type ExpectedContext = $inner_ty;

            const REFERENCE: &'static str = $crate::casey::snake!(
                stringify!($struct_name);
            );

            fn consume(
                $crate::define_interactable!(@@a $($value)?): &mut InteractionContext<Self::ExpectedContext>,
            ) -> BoxFuture<'_, Result<(), serenity::Error>> {
                Box::pin(async move {
                    $($tt)*
                    Ok(())
                })
            }
        }
    )*};
}
