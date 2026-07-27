use std::sync::Arc;

use tokio::sync::{Mutex, MutexGuard};

/// Serializes every production decision that can plan or append a command
/// transition. Control and adapter services receive the same instance from the
/// composition root; a service-local lock is not sufficient because revocation
/// and adapter observations share the durable command log.
#[derive(Clone, Default)]
pub struct CoreDecisionGate(Arc<Mutex<()>>);

impl CoreDecisionGate {
    pub async fn acquire(&self) -> MutexGuard<'_, ()> {
        self.0.lock().await
    }
}

#[cfg(test)]
mod tests {
    use super::CoreDecisionGate;
    use patchbay_contracts::patchbay::{
        CommandId, CommandTransition, FailureCode, GrantRevocationEffect, Operation,
        OperationState,
    };
    use patchbay_core::acceptance::{apply_grant_revocation_effect, apply_transition, CommandRecord};
    use std::sync::{Arc, Mutex};
    use tokio::sync::Barrier;

    #[tokio::test]
    async fn barrier_controlled_decisions_are_totally_ordered() {
        let gate = CoreDecisionGate::default();
        let first_ready = Arc::new(Barrier::new(2));
        let second_started = Arc::new(Barrier::new(2));
        let first_gate = gate.clone();
        let first_ready_for_task = first_ready.clone();
        let second_started_for_task = second_started.clone();
        let first = tokio::spawn(async move {
            let _guard = first_gate.acquire().await;
            first_ready_for_task.wait().await;
            second_started_for_task.wait().await;
            1_u8
        });

        first_ready.wait().await;
        let second_gate = gate.clone();
        let second_started_for_task = second_started.clone();
        let second = tokio::spawn(async move {
            let attempt = tokio::spawn(async move {
                let _guard = second_gate.acquire().await;
                2_u8
            });
            second_started_for_task.wait().await;
            attempt.await.expect("the second decision must complete")
        });

        assert_eq!(first.await.expect("the first decision must complete"), 1);
        assert_eq!(second.await.expect("the second decision must complete"), 2);
    }

    #[tokio::test]
    async fn barrier_race_cannot_commit_adapter_transition_between_revocation_plan_and_append() {
        let gate = CoreDecisionGate::default();
        let planned = Arc::new(Barrier::new(2));
        let release_revocation = Arc::new(Barrier::new(2));
        let order = Arc::new(Mutex::new(Vec::new()));

        let revocation_gate = gate.clone();
        let revocation_planned = planned.clone();
        let revocation_release = release_revocation.clone();
        let revocation_order = order.clone();
        let revocation = tokio::spawn(async move {
            let _guard = revocation_gate.acquire().await;
            revocation_order.lock().unwrap().push("revocation_plan");
            revocation_planned.wait().await;
            revocation_release.wait().await;
            revocation_order.lock().unwrap().push("revocation_append");
        });

        let adapter_gate = gate.clone();
        let adapter_planned = planned.clone();
        let adapter_order = order.clone();
        let adapter = tokio::spawn(async move {
            adapter_planned.wait().await;
            adapter_order.lock().unwrap().push("adapter_attempt");
            let _guard = adapter_gate.acquire().await;
            adapter_order.lock().unwrap().push("adapter_append");
        });

        release_revocation.wait().await;
        revocation.await.expect("revocation task must complete");
        adapter.await.expect("adapter task must complete");
        assert_eq!(
            *order.lock().unwrap(),
            vec!["revocation_plan", "adapter_attempt", "revocation_append", "adapter_append"],
        );

        // The LSN sequence produced by that order is replayed through the
        // canonical transition folds: the earlier revocation terminal wins.
        let command_id = CommandId { value: "race-command".to_owned() };
        let mut record = CommandRecord::new(
            Operation { command_id: Some(command_id.clone()), ..Operation::default() },
            1,
        )
        .expect("race command has an id");
        apply_grant_revocation_effect(
            &mut record,
            &GrantRevocationEffect {
                command_id: Some(command_id.clone()),
                from_state: OperationState::Accepted as i32,
                to_state: OperationState::Cancelled as i32,
                failure_code: FailureCode::Cancelled as i32,
            },
            2,
        )
        .expect("revocation effect is valid");
        let late_adapter = CommandTransition {
            command_id: Some(command_id),
            from_state: OperationState::Accepted as i32,
            to_state: OperationState::Completed as i32,
            failure_code: FailureCode::Unspecified as i32,
            ..CommandTransition::default()
        };
        assert!(apply_transition(&mut record, &late_adapter, 3).is_err());
        assert_eq!(record.state, OperationState::Cancelled);
        assert_eq!(record.terminal_lsn, Some(2));
    }
}
