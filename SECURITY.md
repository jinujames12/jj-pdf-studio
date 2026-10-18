# Security policy

## Supported versions

Only the latest released version receives security fixes.

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

Use GitHub's private vulnerability reporting: open the **Security** tab of this repository and
choose **Report a vulnerability**. TODO: if private reporting is not enabled, add a security
contact email here.

Please include:

- the app version and your Windows version,
- what you did and what you expected,
- a sample file that triggers the problem, if you can share it safely.

We aim to acknowledge reports within 7 days and to publish a fix or mitigation as soon as
practical. We will credit you in the release notes unless you prefer otherwise.

## Scope and design notes

JJ PDF Studio is designed to run fully offline: it makes no network requests, collects no
telemetry, and only touches files you pick through a dialog or drag-and-drop. Reports about
malformed PDFs that crash the app or cause unexpected file access are in scope.
