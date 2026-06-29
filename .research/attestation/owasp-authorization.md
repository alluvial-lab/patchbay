---
source_handle: owasp-authorization
fetched: 2026-06-28
source_url: https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: OWASP Authorization Cheat Sheet

## Summary

The OWASP Authorization Cheat Sheet recommends least privilege, deny-by-default authorization, explicit configuration instead of relying only on framework defaults, and validation of permissions on every request using application-wide mechanisms where possible.

## Key passages

1. From "Enforce Least Privileges":

> Least Privileges refers to the principle of assigning users only the minimum privileges necessary to complete their job. ... Failure to enforce least privileges in an application can jeopardize the confidentiality of sensitive resources.

2. From "Deny by Default":

> The application must always make a decision, whether implicitly or explicitly, to either deny or permit the requested access. ... For security purposes an application should be configured to deny access by default.

3. From the deny-by-default best practices:

> Adopt a "deny-by-default" mentality both during initial development and whenever new functionality or resources are exposed by the app. One should be able to explicitly justify why a specific permission was granted to a particular user or group rather than assuming access to be the default position.

4. From "Validate the Permissions on Every Request":

> Permission should be validated correctly on every request, regardless of whether the request was initiated by an AJAX script, server-side, or any other source. ... Even if just a single access control check is "missed", the confidentiality and/or integrity of a resource can be jeopardized.
