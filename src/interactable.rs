use crate::context::{InteractionContext, IntoInteractionParts};
use serenity::futures::future::BoxFuture;

pub trait Interactable {
    type ExpectedContext: IntoInteractionParts;
    const REFERENCE: &'static str;

    fn consume(
        ctx: &mut InteractionContext<Self::ExpectedContext>,
    ) -> BoxFuture<'_, Result<(), serenity::Error>>;
}
