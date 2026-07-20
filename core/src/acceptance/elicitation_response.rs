//! Fail-fast validation for typed elicitation response payloads.

use patchbay_contracts::patchbay::{
    response_contract, typed_correlation, ElicitationResponsePayload, Operation, OperationKind,
    ResponseContractKind,
};
use prost::Message;

use super::ActiveElicitation;

/// Validate an elicitation or approval response against its active contract.
///
/// This is deliberately a pure boundary check. It returns a diagnostic for
/// every invalid shape so the acceptance pipeline can reject before grant
/// checks, target resolution, or durable append.
pub fn validate_response_payload(
    operation: &Operation,
    active: Option<&ActiveElicitation>,
) -> Result<(), String> {
    let elicitation_id = operation
        .correlations
        .iter()
        .find_map(|correlation| match correlation.r#ref.as_ref() {
            Some(typed_correlation::Ref::ElicitationId(id)) => Some(id),
            _ => None,
        })
        .ok_or_else(|| {
            "elicitation-response Operation has no ElicitationId correlation".to_owned()
        })?;

    let active = active.ok_or_else(|| {
        format!("no active elicitation for {elicitation_id:?} (unknown or wrong domain)")
    })?;

    if active.is_terminal {
        return Err(format!(
            "elicitation {elicitation_id:?} is already terminal"
        ));
    }

    let operation_kind = OperationKind::try_from(operation.kind)
        .map_err(|_| "response Operation has an unknown operation kind".to_owned())?;
    let contract_kind = ResponseContractKind::try_from(active.contract.contract_kind)
        .map_err(|_| "response contract has an unknown contract kind".to_owned())?;

    match (operation_kind, contract_kind) {
        (OperationKind::ElicitationResponse, ResponseContractKind::Question) => {}
        (OperationKind::ApprovalResponse, ResponseContractKind::Approval) => return Ok(()),
        (kind, contract_kind) => {
            return Err(format!(
                "response kind {kind:?} does not match contract kind {contract_kind:?}"
            ));
        }
    }

    let question = match active.contract.contract_body.as_ref() {
        Some(response_contract::ContractBody::Question(question)) => question,
        None => {
            return Err("question contract is missing its typed QuestionContract body".to_owned())
        }
    };
    let payload = decode_response_payload(operation)?;

    let has_option = !payload.selected_option_id.is_empty();
    let has_free_text = !payload.free_text.is_empty();
    match (has_option, has_free_text) {
        (true, true) => {
            return Err(
                "response carries both a selected_option_id and free_text; exactly one primary answer is allowed"
                    .to_owned(),
            )
        }
        (false, false) => {
            return Err(
                "response carries neither a selected_option_id nor free_text; exactly one primary answer is required"
                    .to_owned(),
            )
        }
        _ => {}
    }

    if has_option
        && !question
            .options
            .iter()
            .any(|option| option.option_id == payload.selected_option_id)
    {
        return Err(format!(
            "selected_option_id {:?} is not one of the contract's options",
            payload.selected_option_id
        ));
    }

    if has_free_text && !question.allow_free_text {
        return Err(
            "response carries free_text but the contract does not allow_free_text".to_owned(),
        );
    }

    Ok(())
}

fn decode_response_payload(operation: &Operation) -> Result<ElicitationResponsePayload, String> {
    let envelope = operation
        .payload
        .as_ref()
        .ok_or_else(|| "elicitation-response Operation is missing its payload".to_owned())?;
    ElicitationResponsePayload::decode(envelope.payload.as_slice())
        .map_err(|error| format!("cannot decode ElicitationResponsePayload: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{
        response_contract, typed_correlation, ElicitationId, PayloadEnvelope, QuestionContract,
        ResponseContract, ResponseOption, TypedCorrelation,
    };

    fn correlation() -> TypedCorrelation {
        TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::ElicitationId(ElicitationId {
                value: "elicitation-1".to_owned(),
            })),
        }
    }

    fn operation(kind: OperationKind, payload: ElicitationResponsePayload) -> Operation {
        Operation {
            kind: kind as i32,
            correlations: vec![correlation()],
            payload: Some(PayloadEnvelope {
                payload: payload.encode_to_vec(),
                ..PayloadEnvelope::default()
            }),
            ..Operation::default()
        }
    }

    fn active_question(allow_free_text: bool) -> ActiveElicitation {
        ActiveElicitation {
            contract: ResponseContract {
                contract_kind: ResponseContractKind::Question as i32,
                contract_body: Some(response_contract::ContractBody::Question(
                    QuestionContract {
                        options: vec![ResponseOption {
                            option_id: "yes".to_owned(),
                            label: "Yes".to_owned(),
                        }],
                        allow_free_text,
                    },
                )),
                ..ResponseContract::default()
            },
            is_terminal: false,
        }
    }

    #[test]
    fn question_response_validation_covers_all_accept_and_reject_branches() {
        let accepted = vec![
            (
                "selected option",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        selected_option_id: "yes".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(active_question(false)),
                true,
            ),
            (
                "free text",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        free_text: "custom".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(active_question(true)),
                true,
            ),
            (
                "answer and clarification",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        selected_option_id: "yes".to_owned(),
                        clarification: "because it is clear".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(active_question(false)),
                true,
            ),
            (
                "approval",
                operation(
                    OperationKind::ApprovalResponse,
                    ElicitationResponsePayload::default(),
                ),
                Some(ActiveElicitation {
                    contract: ResponseContract {
                        contract_kind: ResponseContractKind::Approval as i32,
                        ..ResponseContract::default()
                    },
                    is_terminal: false,
                }),
                true,
            ),
        ];

        for (name, operation, active, expected_ok) in accepted {
            assert_eq!(
                validate_response_payload(&operation, active.as_ref()).is_ok(),
                expected_ok,
                "acceptance case {name}"
            );
        }

        let rejected = vec![
            (
                "missing correlation",
                Operation {
                    correlations: vec![],
                    ..operation(
                        OperationKind::ElicitationResponse,
                        ElicitationResponsePayload {
                            selected_option_id: "yes".to_owned(),
                            ..ElicitationResponsePayload::default()
                        },
                    )
                },
                Some(active_question(false)),
            ),
            (
                "unknown elicitation",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        selected_option_id: "yes".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                None,
            ),
            (
                "terminal elicitation",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        selected_option_id: "yes".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(ActiveElicitation {
                    is_terminal: true,
                    ..active_question(false)
                }),
            ),
            (
                "approval against question",
                operation(
                    OperationKind::ApprovalResponse,
                    ElicitationResponsePayload::default(),
                ),
                Some(active_question(false)),
            ),
            (
                "question without typed body",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        selected_option_id: "yes".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(ActiveElicitation {
                    contract: ResponseContract {
                        contract_kind: ResponseContractKind::Question as i32,
                        ..ResponseContract::default()
                    },
                    is_terminal: false,
                }),
            ),
            (
                "both primary answers",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        selected_option_id: "yes".to_owned(),
                        free_text: "custom".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(active_question(true)),
            ),
            (
                "neither primary answer",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload::default(),
                ),
                Some(active_question(true)),
            ),
            (
                "mismatched option",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        selected_option_id: "no".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(active_question(false)),
            ),
            (
                "free text disallowed",
                operation(
                    OperationKind::ElicitationResponse,
                    ElicitationResponsePayload {
                        free_text: "custom".to_owned(),
                        ..ElicitationResponsePayload::default()
                    },
                ),
                Some(active_question(false)),
            ),
        ];

        for (name, operation, active) in rejected {
            assert!(
                validate_response_payload(&operation, active.as_ref()).is_err(),
                "rejection case {name}"
            );
        }
    }
}
