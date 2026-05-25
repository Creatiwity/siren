use diesel::connection::{Instrumentation, InstrumentationEvent};

#[derive(Default)]
pub struct AsyncDieselInstrumentation {
    query_span: Option<tracing::Span>,
    transaction_span: Option<tracing::Span>,
}

impl Instrumentation for AsyncDieselInstrumentation {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        match event {
            InstrumentationEvent::StartQuery { query, .. } => {
                self.query_span = Some(tracing::info_span!(
                    "db.sql.query",
                    "db.system" = "postgresql",
                    "db.statement" = tracing::field::display(query),
                    "db.error" = tracing::field::Empty,
                ));
            }
            InstrumentationEvent::FinishQuery { error, .. } => {
                if let (Some(span), Some(err)) = (&self.query_span, error) {
                    span.record("db.error", tracing::field::display(err));
                }
                self.query_span = None;
            }
            InstrumentationEvent::BeginTransaction { depth, .. } if depth.get() == 1 => {
                self.transaction_span = Some(tracing::info_span!(
                    "db.sql.transaction",
                    "db.system" = "postgresql",
                    "db.rolled_back" = tracing::field::Empty,
                ));
            }
            InstrumentationEvent::CommitTransaction { depth, .. } if depth.get() == 1 => {
                self.transaction_span = None;
            }
            InstrumentationEvent::RollbackTransaction { depth, .. } if depth.get() == 1 => {
                if let Some(ref span) = self.transaction_span {
                    span.record("db.rolled_back", true);
                }
                self.transaction_span = None;
            }
            _ => {}
        }
    }
}
