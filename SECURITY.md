# Security Policy

Pace Buddy handles private athlete data, OAuth credentials, Strava tokens, and LLM prompts. Treat security issues and
privacy issues as sensitive by default.

## Reporting

Do not open a public issue for vulnerabilities, leaked secrets, private athlete data, or token exposure. Use GitHub
private vulnerability reporting if it is enabled for the repository, or contact the maintainer through a private channel.

## Secrets

Never commit:

- Strava client secrets.
- Strava access or refresh tokens.
- OpenRouter API keys.
- JWT signing secrets.
- Fly API tokens.
- Local SQLite databases, recovered production databases, or exported athlete data.

Use `.env.example` for variable names and deployment secret stores for production values.

## Local Data

SQLite databases are ignored by Git, but they can still contain sensitive Strava data. Remove local database files and
recovered backups before sharing archives, screenshots, or workspace snapshots.

## Public Readiness

Before operating the app for external users, review `COMPLIANCE.md`, rotate any previously committed secrets, and verify
account disconnect, deauthorization, and activity deletion flows end to end.
