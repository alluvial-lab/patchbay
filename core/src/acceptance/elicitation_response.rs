//! Fail-fast validation for typed elicitation response payloads.

use patchbay_contracts::patchbay::{
    response_contract, typed_correlation, ApprovalDecision, ApprovalResponsePayload,
    ElicitationResponsePayload, Operation, OperationKind, ResponseContractKind,
};
use prost::Message;

use crate::authority::IssuerContext;

use super::ActiveElicitation;

const RESPONDER_AUTHORITY_DENIED: &str =
    "verified issuer is not authorized to answer this elicitation";

/// Validate that verified ingress identity owns the active response slot.
///
/// This check is deliberately distinct from grant authorization: a valid
/// target/kind grant does not authorize one actor to answer another actor's
/// Elicitation. Diagnostics never disclose either actor identifier.
pub fn validate_response_responder(
    active: &ActiveElicitation,
    issuer: &dyn IssuerContext,
) -> Result<(), String> {
    let expected = active
        .expected_responder_actor
        .as_ref()
        .filter(|actor| !actor.value.is_empty());
    let verified = issuer
        .verified_actor()
        .filter(|actor| !actor.value.is_empty());

    match (expected, verified) {
        (Some(expected), Some(verified)) if expected == verified => Ok(()),
        _ => Err(RESPONDER_AUTHORITY_DENIED.to_owned()),
    }
}

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

    if active.is_terminal && active.winning_response.as_ref() != Some(operation) {
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
        (OperationKind::ApprovalResponse, ResponseContractKind::Approval) => {
            let payload = decode_approval_payload(operation)?;
            let decision = ApprovalDecision::try_from(payload.decision).map_err(|_| {
                format!(
                    "approval response has unknown decision {}",
                    payload.decision
                )
            })?;
            match decision {
                ApprovalDecision::Approved | ApprovalDecision::Denied => return Ok(()),
                ApprovalDecision::Unspecified => {
                    return Err("approval response has an unspecified decision".to_owned())
                }
                ApprovalDecision::ReservedAllowOnce
                | ApprovalDecision::ReservedAlways
                | ApprovalDecision::ReservedPolicyAmend
                | ApprovalDecision::ReservedModifiedInput => {
                    return Err(format!(
                        "approval decision {decision:?} is reserved and not validatable in v0.1.0"
                    ))
                }
            }
        }
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
    // v0.1.0 treats every invalid_response_policy as
    // REJECT_AND_KEEP_PENDING. The terminal-on-invalid enum values are named
    // reserved seams and are not validatable until a future promotion.
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

fn decode_approval_payload(operation: &Operation) -> Result<ApprovalResponsePayload, String> {
    let envelope = operation
        .payload
        .as_ref()
        .ok_or_else(|| "approval-response Operation is missing its payload".to_owned())?;
    if envelope.content_type != patchbay_contracts::patchbay::PayloadContentType::Protobuf as i32 {
        return Err(
            "approval-response Operation payload content_type must be PAYLOAD_CONTENT_TYPE_PROTOBUF"
                .to_owned(),
        );
    }
    ApprovalResponsePayload::decode(envelope.payload.as_slice())
        .map_err(|error| format!("cannot decode ApprovalResponsePayload: {error}"))
}

fn decode_response_payload(operation: &Operation) -> Result<ElicitationResponsePayload, String> {
    let envelope = operation
        .payload
        .as_ref()
        .ok_or_else(|| "elicitation-response Operation is missing its payload".to_owned())?;
    if envelope.content_type != patchbay_contracts::patchbay::PayloadContentType::Protobuf as i32 {
        return Err(
            "elicitation-response Operation payload content_type must be PAYLOAD_CONTENT_TYPE_PROTOBUF"
                .to_owned(),
        );
    }
    ElicitationResponsePayload::decode(envelope.payload.as_slice())
        .map_err(|error| format!("cannot decode ElicitationResponsePayload: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{
        response_contract, typed_correlation, ActorId, AuthorityDomainId, DeviceId, ElicitationId,
        EndpointId, Generation, InvalidResponsePolicy, PayloadContentType, PayloadEnvelope,
        QuestionContract, ResponseContract, ResponseOption, TypedCorrelation,
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
                content_type: PayloadContentType::Protobuf as i32,
                ..PayloadEnvelope::default()
            }),
            ..Operation::default()
        }
    }

    fn approval_operation(decision: ApprovalDecision) -> Operation {
        Operation {
            kind: OperationKind::ApprovalResponse as i32,
            correlations: vec![correlation()],
            payload: Some(PayloadEnvelope {
                payload: ApprovalResponsePayload {
                    decision: decision as i32,
                }
                .encode_to_vec(),
                content_type: PayloadContentType::Protobuf as i32,
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
            expected_responder_actor: Some(ActorId {
                value: "operator".to_owned(),
            }),
            is_terminal: false,
            winning_response: None,
        }
    }

    fn active_approval() -> ActiveElicitation {
        ActiveElicitation {
            contract: ResponseContract {
                contract_kind: ResponseContractKind::Approval as i32,
                ..ResponseContract::default()
            },
            expected_responder_actor: Some(ActorId {
                value: "operator".to_owned(),
            }),
            is_terminal: false,
            winning_response: None,
        }
    }

    struct ResponderIssuer {
        actor: Option<ActorId>,
        domain: AuthorityDomainId,
    }

    impl ResponderIssuer {
        fn with_actor(value: Option<&str>) -> Self {
            Self {
                actor: value.map(|value| ActorId {
                    value: value.to_owned(),
                }),
                domain: AuthorityDomainId {
                    value: "authority-main".to_owned(),
                },
            }
        }
    }

    impl IssuerContext for ResponderIssuer {
        fn verified_actor(&self) -> Option<&ActorId> {
            self.actor.as_ref()
        }

        fn verified_endpoint(&self) -> Option<&EndpointId> {
            panic!("responder validation must not consult endpoint identity")
        }

        fn verified_device(&self) -> Option<&DeviceId> {
            panic!("responder validation must not consult device identity")
        }

        fn endpoint_generation(&self) -> Option<Generation> {
            panic!("responder validation must not consult endpoint generation")
        }

        fn authority_domain_id(&self) -> &AuthorityDomainId {
            &self.domain
        }
    }

    #[test]
    fn responder_validation_requires_exact_non_empty_verified_actor_equality() {
        let active = active_question(false);
        assert!(validate_response_responder(
            &active,
            &ResponderIssuer::with_actor(Some("operator"))
        )
        .is_ok());

        let cases = [
            (
                Some(ActorId {
                    value: "operator".to_owned(),
                }),
                Some("different-operator"),
            ),
            (None, Some("operator")),
            (
                Some(ActorId {
                    value: String::new(),
                }),
                Some("operator"),
            ),
            (
                Some(ActorId {
                    value: "operator".to_owned(),
                }),
                None,
            ),
            (
                Some(ActorId {
                    value: "operator".to_owned(),
                }),
                Some(""),
            ),
        ];

        for (expected_responder_actor, verified_actor) in cases {
            let active = ActiveElicitation {
                expected_responder_actor,
                ..active_question(false)
            };
            assert_eq!(
                validate_response_responder(&active, &ResponderIssuer::with_actor(verified_actor)),
                Err(RESPONDER_AUTHORITY_DENIED.to_owned())
            );
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
                approval_operation(ApprovalDecision::Approved),
                Some(active_approval()),
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
                    expected_responder_actor: Some(ActorId {
                        value: "operator".to_owned(),
                    }),
                    is_terminal: false,
                    winning_response: None,
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

    #[test]
    fn approval_response_validation_covers_committed_and_reserved_decisions() {
        for decision in [ApprovalDecision::Approved, ApprovalDecision::Denied] {
            let operation = approval_operation(decision);
            assert!(
                validate_response_payload(&operation, Some(&active_approval())).is_ok(),
                "committed decision {decision:?} must be accepted"
            );
        }

        let rejected = [
            (
                ApprovalDecision::Unspecified,
                "approval response has an unspecified decision",
            ),
            (
                ApprovalDecision::ReservedAllowOnce,
                "reserved and not validatable in v0.1.0",
            ),
            (
                ApprovalDecision::ReservedAlways,
                "reserved and not validatable in v0.1.0",
            ),
            (
                ApprovalDecision::ReservedPolicyAmend,
                "reserved and not validatable in v0.1.0",
            ),
            (
                ApprovalDecision::ReservedModifiedInput,
                "reserved and not validatable in v0.1.0",
            ),
        ];
        for (decision, expected_diagnostic) in rejected {
            let operation = approval_operation(decision);
            let diagnostic = validate_response_payload(&operation, Some(&active_approval()))
                .expect_err("non-committed approval decision must be rejected");
            assert!(
                diagnostic.contains(expected_diagnostic),
                "decision {decision:?} produced unexpected diagnostic {diagnostic:?}"
            );
        }

        let mut wrong_content_type = approval_operation(ApprovalDecision::Approved);
        wrong_content_type.payload.as_mut().unwrap().content_type = PayloadContentType::Json as i32;
        assert!(validate_response_payload(&wrong_content_type, Some(&active_approval())).is_err());

        let kind_mismatch = approval_operation(ApprovalDecision::Approved);
        assert!(validate_response_payload(&kind_mismatch, Some(&active_question(false))).is_err());
    }

    #[test]
    fn wrong_payload_content_types_are_rejected_before_decode() {
        for content_type in [
            PayloadContentType::Json,
            PayloadContentType::Unspecified,
            PayloadContentType::Binary,
        ] {
            let mut operation = operation(
                OperationKind::ElicitationResponse,
                ElicitationResponsePayload {
                    selected_option_id: "yes".to_owned(),
                    ..ElicitationResponsePayload::default()
                },
            );
            operation.payload.as_mut().unwrap().content_type = content_type as i32;
            assert!(
                validate_response_payload(&operation, Some(&active_question(false))).is_err(),
                "content type {content_type:?} must be rejected"
            );
        }
    }

    #[test]
    fn terminal_on_invalid_policy_is_reserved_and_keeps_validation_failed_behavior() {
        let mut active = active_question(false);
        active.contract.invalid_response_policy = InvalidResponsePolicy::TerminalDeclined as i32;
        let operation = Operation {
            payload: None,
            ..operation(
                OperationKind::ElicitationResponse,
                ElicitationResponsePayload::default(),
            )
        };

        assert!(validate_response_payload(&operation, Some(&active)).is_err());
    }

    #[test]
    fn exact_terminal_retry_passes_validation_for_storage_deduplication() {
        let operation = operation(
            OperationKind::ElicitationResponse,
            ElicitationResponsePayload {
                selected_option_id: "yes".to_owned(),
                ..ElicitationResponsePayload::default()
            },
        );
        let active = ActiveElicitation {
            is_terminal: true,
            winning_response: Some(operation.clone()),
            ..active_question(false)
        };

        assert!(validate_response_payload(&operation, Some(&active)).is_ok());
    }

    #[test]
    fn exact_terminal_approval_retry_passes_validation_for_storage_deduplication() {
        let operation = approval_operation(ApprovalDecision::Denied);
        let active = ActiveElicitation {
            is_terminal: true,
            winning_response: Some(operation.clone()),
            ..active_approval()
        };

        assert!(validate_response_payload(&operation, Some(&active)).is_ok());
    }
}
