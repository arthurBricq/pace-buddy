# Compliance Notes

This document is a public-readiness checklist, not legal advice. Review the current Strava API Agreement, Strava API
Policy, privacy requirements, and the selected LLM provider terms before operating Pace Buddy for external users.

Sources reviewed on June 30, 2026:

- https://www.strava.com/legal/api
- https://www.strava.com/legal/api_policy

## Current Compliance Risk

Pace Buddy uses Strava-derived activity data to build prompts and tool results for LLM-powered coaching and training
insights. Strava's current API Policy should be reviewed carefully before this feature is made available to external
users. As of the June 1, 2026 policy version reviewed during this cleanup, the policy contains explicit restrictions on
using Strava API data in AI applications, including model input, grounding, context windows, embeddings, prompts,
training, and fine-tuning.

Do not treat the repository as cleared for public production use until this has been resolved. Practical options may
include changing the product design, obtaining explicit permission, using only user-provided non-Strava context for LLM
features, or otherwise aligning the implementation with Strava's current developer terms.

## Data Use Intent

The app is designed for a user to analyze their own running history. It does not provide a social feed, leaderboard,
public athlete comparison, or Strava-like network features.

Strava data is used for:

- OAuth login and account linking.
- Activity list and activity detail views.
- Workout tagging.
- Interval parsing and workout review.
- Training-block summaries.
- MAS estimation from race-tagged activities.
- Running Coach context and coach tools, subject to the compliance risk above.

## Retention And Deletion

The app caches synced Strava data in SQLite to avoid unnecessary API calls and to power analysis features.

Before public operation, verify that retention behavior matches the current Strava developer requirements. As of the
June 1, 2026 API Policy reviewed during this cleanup, those requirements include:

- Cached Strava data must not be retained longer than seven days unless the applicable terms allow it.
- Activity deletion events must be reflected within forty-eight hours.
- Deauthorization or user deletion requests must remove Strava data and derived personal data expeditiously, with a
  thirty-day outer limit unless a valid legal basis requires longer retention.
- Deauthorization should revoke the token and delete user data held by the app.
- User disconnect/delete flows should remove Strava tokens and derived user data.
- Stream and lap caching should be audited separately because streams can contain sensitive activity traces.

## AI And Third-Party Providers

The LLM provider currently receives prompt context assembled by the backend. That context can include Strava-derived
training summaries and, through coach tools, detailed session descriptions.

Before any public launch:

- Confirm whether this usage is permitted by Strava's current terms.
- Confirm the LLM provider's data-use and retention settings.
- Avoid sending raw access tokens, refresh tokens, email addresses, or unrelated profile data to any LLM provider.
- Keep `doc/ai-coach-data-inputs.md` current whenever coach context or coach tool payloads change.

## Secrets

Do not commit credentials or deployment secrets. The following must live in environment variables, local ignored files,
or deployment secret stores:

- `JWT_SECRET`
- `STRAVA_CLIENT_ID`
- `STRAVA_CLIENT_SECRET`
- `STRAVA_WEBHOOK_VERIFY_TOKEN`
- `OPENROUTER_API_KEY`
- `ADMIN_STRAVA_ID`

The Strava webhook verification token that was previously committed in `fly.toml` should be rotated before the
repository is made public.

## Public Release Checklist

- Current Strava terms reviewed and product behavior adjusted where needed.
- Webhook verification token rotated.
- Production secrets moved to the deployment secret store.
- Local databases and recovered backups removed from the working tree.
- Data deletion behavior tested for Strava disconnect, Strava deauthorization, and activity deletion webhooks.
- LLM data sharing documented in user-facing privacy terms.
- A repository license chosen before accepting outside contributions.
