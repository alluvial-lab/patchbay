---
source_handle: owasp-authentication
fetched: 2026-06-28
source_url: https://raw.githubusercontent.com/OWASP/CheatSheetSeries/master/cheatsheets/Authentication_Cheat_Sheet.md
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: OWASP Authentication Cheat Sheet

## Summary

The OWASP Authentication Cheat Sheet recommends reauthentication for sensitive features and after risk events, login throttling/account lockout controls for guessing attacks, and MFA wherever possible while acknowledging that MFA may not always be practical or feasible for a given audience.

## Key passages

1. From "Require Re-authentication for Sensitive Features":

> In order to mitigate CSRF and session hijacking, it's important to require the current credentials for an account before updating sensitive account information such as the user's password or email address -- or before sensitive transactions.

2. From "Reauthentication After Risk Events":

> Reauthentication is critical when an account has experienced high-risk activity such as account recovery, password resets, or suspicious behavior patterns.

3. From "When to Trigger Reauthentication":

> [Trigger reauthentication for] suspicious account activity ... account recovery ... [and] critical actions.

4. From "Multi-Factor Authentication":

> Multi-factor authentication (MFA) is by far the best defense against the majority of password-related attacks ... As such, it should be implemented wherever possible; however, depending on the audience of the application, it may not be practical or feasible to enforce the use of MFA.

5. From "Login Throttling":

> Login Throttling is a protocol used to prevent an attacker from making too many attempts at guessing a password through normal interactive means ... [including] Maximum number of attempts.

6. From "Account Lockout":

> The counter of failed logins should be associated with the account itself, rather than the source IP address, in order to prevent an attacker from making login attempts from a large number of different IP addresses.
