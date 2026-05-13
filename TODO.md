# Alpha release roadmap

## Bugs

- The MAS computation does not appear to be robust.
    - The race tabs often does not include races.
    - Races with D+ end up in terrible MAS, they should not be counted
    - No possibility of user overrides of which races to take into account.

## Features

- Coach suggestions: suggest quality sessions

- Coaching goals: dedicated UI to set goals

- Upon creating new account (or upon empty coach history), the coach should send a message that clarifies what it can
  do, some sort of "getting to know each other, here's what I am able to do."

- "Clear coach" in the settings

## Refactoring

- Generate OpenAPI types from Rust (with utoipa) and use those in the frontend !
