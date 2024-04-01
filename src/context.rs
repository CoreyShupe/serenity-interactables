use crate::interactable::Interactable;
use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, Http, InteractionId,
    Message, MessageId, ModalInteraction,
};
use std::sync::Arc;

pub trait IntoInteractionParts {
    fn interaction_id(&self) -> InteractionId;

    fn interaction_token(&self) -> &str;

    fn get_cid(&self) -> &str;
}

macro_rules! impl_interaction_type {
    ($($t:ty => { |$self_ident:ident| $($tt:tt)* }),*) => {$(
        impl IntoInteractionParts for $t {
            fn interaction_id(&self) -> InteractionId {
                self.id
            }

            fn interaction_token(&self) -> &str {
                &self.token
            }

            fn get_cid(&self) -> &str {
                let $self_ident = self;
                $($tt)*
            }
        }
    )*};
}

impl_interaction_type!(CommandInteraction => {
    |me| me.data.name.as_str()
}, ModalInteraction => {
    |me| me.data.custom_id.as_str()
}, ComponentInteraction => {
    |me| me.data.custom_id.as_str()
});

#[derive(Clone, Copy, Debug)]
enum ResponseTriState {
    Not,
    Responded,
    Deferred,
}

impl ResponseTriState {
    fn is_not(&self) -> bool {
        matches!(self, ResponseTriState::Not)
    }

    fn is_responded(&self) -> bool {
        matches!(self, ResponseTriState::Responded)
    }

    fn is_deferred(&self) -> bool {
        matches!(self, ResponseTriState::Deferred)
    }
}

pub struct InteractionContext<T: IntoInteractionParts> {
    inner: T,
    context: Context,
    responded: ResponseTriState,
}

impl<T: IntoInteractionParts> !IntoInteractionParts for InteractionContext<T> {}

impl<T: IntoInteractionParts> InteractionContext<T> {
    pub fn new(inner: T, context: Context) -> Self {
        Self {
            inner,
            context,
            responded: ResponseTriState::Not,
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_ctx(&self) -> &Context {
        &self.context
    }

    pub fn http(&self) -> &Arc<Http> {
        &self.context.http
    }

    pub async fn respond(
        &mut self,
        response: &CreateInteractionResponse,
    ) -> Result<(), serenity::Error> {
        if self.responded.is_responded() {
            return Err(serenity::Error::Other(
                "Interaction already responded, stopped early.",
            ));
        }
        self.responded = ResponseTriState::Responded;
        self.http()
            .create_interaction_response(
                self.inner.interaction_id(),
                self.inner.interaction_token(),
                response,
                vec![],
            )
            .await
    }

    pub async fn delete_original(&self) -> Result<(), serenity::Error> {
        self.http()
            .delete_original_interaction_response(self.inner.interaction_token())
            .await
    }

    pub async fn defer(
        &mut self,
        message: CreateInteractionResponseMessage,
    ) -> Result<(), serenity::Error> {
        if !self.responded.is_not() {
            return Err(serenity::Error::Other(
                "Interaction either deferred or responded, stopped early.",
            ));
        }
        self.respond(&CreateInteractionResponse::Defer(message))
            .await?;
        self.responded = ResponseTriState::Deferred;
        Ok(())
    }

    pub async fn ack(&mut self) -> Result<(), serenity::Error> {
        self.respond(&CreateInteractionResponse::Acknowledge).await
    }

    pub async fn followup(
        &mut self,
        message: &CreateInteractionResponseFollowup,
    ) -> Result<Message, serenity::Error> {
        if !self.responded.is_deferred() {
            return Err(serenity::Error::Other(
                "Interaction not deferred, stopped early.",
            ));
        }

        self.responded = ResponseTriState::Responded;
        self.http()
            .create_followup_message(self.inner.interaction_token(), message, vec![])
            .await
    }

    pub async fn delete_followup(&self, id: MessageId) -> Result<(), serenity::Error> {
        self.http()
            .delete_followup_message(self.inner.interaction_token(), id)
            .await
    }

    pub async fn execute_with<I: Interactable<ExpectedContext = T>>(
        &mut self,
    ) -> Result<(), serenity::Error> {
        let result = I::consume(self).await;
        if result.is_err() {
            if !self.responded.is_responded() {
                if self.responded.is_deferred() {
                    self.followup(
                        // TODO: Make this configurable and a more spelled out "error"
                        &CreateInteractionResponseFollowup::new(),
                    )
                    .await?;
                } else {
                    self.ack().await?;
                }
            }
        }
        result
    }
}
