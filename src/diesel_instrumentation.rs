use diesel::connection::{Instrumentation, InstrumentationEvent};
use std::cell::RefCell;

thread_local! {
    static QUERY_SPAN: RefCell<Option<tracing::span::EnteredSpan>> = const { RefCell::new(None) };
    static TRANSACTION_SPAN: RefCell<Option<tracing::span::EnteredSpan>> = const { RefCell::new(None) };
}

pub struct DieselInstrumentation;

impl Instrumentation for DieselInstrumentation {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        match event {
            InstrumentationEvent::BeginTransaction { depth, .. }
                // Only create a span for the outermost transaction (not savepoints)
                if depth.get() == 1 => {
                    let span = tracing::info_span!("db.transaction", "db.system" = "postgresql");
                    TRANSACTION_SPAN.with(|s| *s.borrow_mut() = Some(span.entered()));
                }
            InstrumentationEvent::CommitTransaction { depth, .. }
                if depth.get() == 1 => {
                    TRANSACTION_SPAN.with(|s| s.borrow_mut().take());
                }
            InstrumentationEvent::RollbackTransaction { depth, .. }
                if depth.get() == 1 => {
                    TRANSACTION_SPAN.with(|s| {
                        if let Some(span) = s.borrow_mut().take() {
                            tracing::warn!("transaction rolled back");
                            drop(span);
                        }
                    });
            }
            InstrumentationEvent::StartQuery { query, .. } => {
                let span = tracing::info_span!(
                    "db.query",
                    "db.system" = "postgresql",
                    "db.statement" = %query,
                    "otel.kind" = "client",
                );
                QUERY_SPAN.with(|s| *s.borrow_mut() = Some(span.entered()));
            }
            InstrumentationEvent::FinishQuery { error, .. } => {
                QUERY_SPAN.with(|s| {
                    if let Some(span) = s.borrow_mut().take() {
                        if let Some(err) = error {
                            tracing::error!(error = %err, "db query failed");
                        }
                        drop(span);
                    }
                });
            }
            _ => {}
        }
    }
}
