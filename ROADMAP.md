# Roadmap

This is a short public roadmap for known gaps and near-term product work. It is intentionally separate from private
task notes.

## Product Quality

- Make MAS estimation more robust:
  - Exclude race-tagged activities with unsuitable elevation profiles.
  - Let users choose which races should count toward MAS.
  - Improve the races view so eligible races are easy to audit.
- Add a first coach message for new users or empty coach histories that explains what the Running Coach can do.
- Add explicit goal-setting UI for upcoming races and training priorities.
- Add a clear/reset coach action in coach settings.

## Training Sessions

- Continue the coach suggested sessions work:
  - Suggest structured quality sessions from the Running Coach.
  - Let users accept, edit, reject, skip, or mark suggested sessions done.
  - Link completed Strava activities to planned sessions.

## Engineering

- Generate OpenAPI types from the Rust API and consume them in the frontend.
- Strengthen frontend lint/type coverage.
- Add production-grade database migration support only when the project needs non-disruptive upgrades.
