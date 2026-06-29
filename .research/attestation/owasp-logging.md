---
source_handle: owasp-logging
fetched: 2026-06-28
source_url: https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: OWASP Logging Cheat Sheet

## Summary

The OWASP Logging Cheat Sheet says application logging should include security events and is valuable for security and operational use. It recommends defining logging during requirements/design according to risk, logging authentication successes/failures, authorization failures, session management failures, and security-relevant validation failures, while excluding or protecting session IDs, access tokens, passwords, encryption keys, and sensitive personal data.

## Key passages

1. From "Purpose":

> Application logging should always be included for security events. Application logs are invaluable data for both security and operational use cases.

2. From "Which events to log":

> The level and content of security monitoring, alerting, and reporting needs to be set during the requirements and design stage of projects, and should be proportionate to the information security risks. ... There is no one size fits all solution, and a blind checklist approach can lead to unnecessary "alarm fog" that means real problems go undetected.

3. From "Which events to log":

> Where possible, always log: Input validation failures ... Authentication successes and failures ... Authorization (access control) failures ... Session management failures e.g. cookie session identification value modification or suspicious JWT validation failures.

4. From "Data to exclude":

> The following should usually not be recorded directly in the logs, but instead should be removed, masked, sanitized, hashed, or encrypted: ... Session identification values ... Access tokens ... Sensitive personal data ... Authentication passwords ... Encryption keys and other primary secrets.
